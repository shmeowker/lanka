use axum::extract::multipart::Field;
use futures_util::stream::StreamExt;
use std::path::Path as StdPath;
use tokio::fs::{File, remove_file, rename};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    CurrentUser, FileSummary, IntoResponse, LState, Multipart, ORejection, Path, Redirect,
    StatusCode,
};

enum FileUploadError {
    MultipartError(String),
    EmptyFile(String),
    StorageError(String),
    NamelessFile,
}

struct ParsedForm {
    reply: Option<u64>,
    content: Option<String>,
    attachments: Vec<FileSummary>,
    anonymous: bool,
}

async fn handle_upload(mut field: Field<'_>) -> Result<FileSummary, FileUploadError> {
    let Some(original_name) = field
        .file_name()
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
    else {
        return Err(FileUploadError::NamelessFile);
    };
    let ext = StdPath::new(&original_name)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|s| !s.is_empty())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();

    let uuid = Uuid::new_v4();
    let name = format!("{uuid}.stream");
    let temp_path = StdPath::new("static/").join(&name);

    let mut hasher = blake3::Hasher::new();
    let mut file = File::create(&temp_path).await.unwrap();
    let mut first = true;
    let mut size: usize = 0;

    while let Some(chunk) = field.next().await {
        let bytes = match chunk {
            Ok(data) => data,
            Err(_) => {
                let _ = remove_file(temp_path).await;
                return Err(FileUploadError::MultipartError(original_name));
            }
        };
        if first {
            if bytes.is_empty() {
                let _ = remove_file(temp_path).await;
                return Err(FileUploadError::EmptyFile(original_name));
            }
            first = false;
        }
        hasher.update(&bytes);
        if file.write_all(&bytes).await.is_err() {
            let _ = remove_file(temp_path).await;
            return Err(FileUploadError::StorageError(original_name));
        }
        size += bytes.len();
    }
    if file.flush().await.is_err() {
        let _ = remove_file(temp_path).await;
        return Err(FileUploadError::StorageError(original_name));
    }

    let mut output = [0u8; 16];
    hasher.finalize_xof().fill(&mut output);
    let file_hash = hex::encode(output);

    let name = format!("{file_hash}{ext}");
    let path = temp_path.with_file_name(&name);
    let _ = rename(temp_path, path).await;

    Ok((name, size, original_name))
}

async fn parse_form(mut multipart: Multipart) -> Result<ParsedForm, ORejection> {
    let mut reply: Option<u64> = None;
    let mut content: Option<String> = None;
    let mut attachments: Vec<FileSummary> = vec![];
    let mut anonymous: bool = false;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "reply" => {
                if let Ok(number) = field.text().await.unwrap_or_default().parse::<u64>() {
                    reply = Some(number);
                }
            }
            "content" => {
                content = match field.text().await.ok() {
                    Some(empty) if empty.is_empty() => None,
                    Some(nonempty) => Some(nonempty),
                    None => None,
                };
            }
            "anonymous" => {
                let text = field.text().await.unwrap_or_default();
                anonymous = text == "on" || text == "true";
            }
            "attachments" => {
                let file = match handle_upload(field).await {
                    Ok(name) => name,
                    Err(FileUploadError::MultipartError(name)) => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            format!("Failed to upload {name}.")
                        ));
                    }
                    Err(FileUploadError::EmptyFile(name)) => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            format!("The file {name} is empty."),
                        ));
                    }
                    Err(FileUploadError::StorageError(name)) => {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to store {name}."),
                        ));
                    }
                    Err(FileUploadError::NamelessFile) => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "Some of the files has no name.".to_owned(),
                        ));
                    }
                };

                attachments.push(file);
            }
            &_ => (),
        }
    }

    if attachments.is_empty() && content.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No valid content provided.".to_owned(),
        ));
    }

    Ok(ParsedForm {
        reply,
        content,
        attachments,
        anonymous,
    })
}

pub async fn create_thread(
    state: LState,
    Path(location): Path<Vec<String>>,
    CurrentUser(user): CurrentUser,
    multipart: Multipart,
) -> Result<impl IntoResponse, ORejection> {
    let parsed = parse_form(multipart).await?;

    let author = match parsed.anonymous {
        true => None,
        false => match user {
            Some(user) => Some(user.name),
            None => None,
        },
    };

    match &location[..] {
        [board] => {
            if !state.board.exists(board).await {
                return Err((StatusCode::BAD_REQUEST, "Invalid board.".to_owned()));
            }
            match state
                .post
                .create(
                    board.clone(),
                    None,
                    parsed.reply,
                    parsed.content,
                    parsed.attachments,
                    author,
                )
                .await
            {
                Ok(_) => Ok(Redirect::to(format!("/{}", board).as_str())),
                Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error.".to_owned())),
            }
        }
        _ => Err((StatusCode::BAD_REQUEST, "Invalid form URL.".to_owned())),
    }
}

pub async fn create_post(
    state: LState,
    Path(location): Path<Vec<String>>,
    CurrentUser(user): CurrentUser,
    multipart: Multipart,
) -> Result<impl IntoResponse, ORejection> {
    let parsed = parse_form(multipart).await?;

    let author = match parsed.anonymous {
        true => None,
        false => match user {
            Some(user) => Some(user.name),
            None => None,
        },
    };

    match &location[..] {
        [board, thread] => {
            if !state.board.exists(board).await {
                return Err((StatusCode::BAD_REQUEST, "Invalid board.".to_owned()));
            }
            match thread.parse::<u64>() {
                Ok(thread) => match state.post.get(&thread).await {
                    Some(_) => {
                        match state
                            .post
                            .create(
                                board.clone(),
                                Some(thread),
                                parsed.reply,
                                parsed.content,
                                parsed.attachments,
                                author,
                            )
                            .await
                        {
                            Ok(_) => Ok(Redirect::to(format!("/{board}/{thread}").as_str())),
                            Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error.".to_owned())),
                        }
                    }
                    None => Err((StatusCode::BAD_REQUEST, "Invalid thread ID.".to_owned())),
                },
                Err(_) => Err((StatusCode::BAD_REQUEST, "Invalid thread.".to_owned())),
            }
        }
        _ => Err((StatusCode::BAD_REQUEST, "Invalid form URL.".to_owned())),
    }
}
