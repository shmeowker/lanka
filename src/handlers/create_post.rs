use axum::extract::multipart::Field;
use futures_util::stream::StreamExt;
use std::path::Path as StdPath;
use tokio::fs::{File, remove_file, rename};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
	CurrentUser,
	FileSummary,
	IntoResponse,
	LState,
	Multipart,
	Path,
	Redirect,
	Response,
	Rejection,
	StatusCode
};

type ParsedFormFields = (
	Option<u64>,
	Option<String>,
	Vec<FileSummary>,
	bool,
);


async fn handle_upload(mut field: Field<'_>) -> Option<FileSummary> {
	let original_name = field
		.file_name()
		.filter(move |n| !n.is_empty())
		.map(str::to_owned)?;
	let mut ext = StdPath::new(&original_name)
		.extension()
		.and_then(move |ext| ext.to_str())
		.unwrap_or("")
		.to_string();
	
	if !ext.is_empty() {
		ext.insert(0, '.');
	}
	let uuid = Uuid::new_v4();
	let name = format!("{uuid}.stream");
	let temp_path = StdPath::new("static/").join(&name);

	let abort = || {
		tokio::spawn(remove_file(temp_path.to_owned()));
		None
	};
	
	let mut hasher = blake3::Hasher::new();
	let mut file = File::create(&temp_path).await.unwrap();
	let mut first = true;
	let mut size: usize = 0;
	
	while let Some(chunk) = field.next().await {
		let bytes = match chunk {
			Ok(data) => data,
			Err(_) => return abort(),
		};
		if first {
			if bytes.is_empty() {
				return abort();
			}
			first = false;
		}
		hasher.update(&bytes);
		file.write_all(&bytes).await;
		size += bytes.len();
	}
	if file.flush().await.is_err() {
		return abort();
	}

	let mut output = [0u8; 16];
	hasher.finalize_xof().fill(&mut output);
	let file_hash = hex::encode(output);

	let name = format!("{file_hash}{ext}");
	let path = temp_path.with_file_name(&name);
	let _ = rename(temp_path, path).await;

	Some((name, size, original_name))
}

async fn parse_form(mut multipart: Multipart) -> Result<ParsedFormFields, Rejection> {
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
				if let Some(file) = handle_upload(field).await {
					attachments.push(file);
				}
			}
			&_ => (),
		}
	}
	if attachments.is_empty() && content.is_none() {
		return Err((StatusCode::BAD_REQUEST, "No valid content provided."));
	}

	Ok((reply, content, attachments, anonymous))
}

pub async fn create_thread(
	state: LState,
	Path(location): Path<Vec<String>>,
	CurrentUser(user): CurrentUser,
	multipart: Multipart,
) -> Result<impl IntoResponse, Rejection> {
	let (reply, content, attachments, anonymous) = parse_form(multipart).await?;
	
	let author = match anonymous {
		true => None,
		false => match user {
			Some(user) => Some(user.name),
			None => None,
		}
	};
	
	match &location[..] {
		[board] => {
			if !state.board.exists(board).await {
				return Err((StatusCode::BAD_REQUEST, "Invalid board."))
			}
			match state
				.post
				.create(board.clone(), None, reply, content, attachments, author)
				.await
			{
				Ok(_) => Ok(Redirect::to(format!("/{}", board).as_str())),
				Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error.")),
			}
		}
		_ => Err((StatusCode::BAD_REQUEST, "Invalid form URL."))
	}
}

pub async fn create_post(
	state: LState,
	Path(location): Path<Vec<String>>,
	CurrentUser(user): CurrentUser,
	multipart: Multipart,
) -> Result<impl IntoResponse, Rejection> {
	let (reply, content, attachments, anonymous) = parse_form(multipart).await?;
	
	let author = match anonymous {
		true => None,
		false => match user {
			Some(user) => Some(user.name),
			None => None,
		}
	};
	
	match &location[..] {
		[board, thread] => {
			if !state.board.exists(board).await {
				return Err((StatusCode::BAD_REQUEST, "Invalid board."))
			}
			match thread.parse::<u64>() {
				Ok(thread) => match state.post.get(&thread).await {
					Some(_) => {
						match state
							.post
							.create(board.clone(), Some(thread), reply, content, attachments, author)
							.await
						{
							Ok(_) => Ok(Redirect::to(format!("/{}/{}", board, thread).as_str())),
							Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error.")),
						}
					},
					None => Err((StatusCode::BAD_REQUEST, "Invalid thread ID.")),
				},
				Err(_) => Err((StatusCode::BAD_REQUEST, "Invalid thread."))
			}
		},
		_ => Err((StatusCode::BAD_REQUEST, "Invalid form URL."))
	}
}
