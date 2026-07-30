use axum::extract::{Multipart, multipart::Field};
use axum::http::StatusCode;
use std::path::Path as StdPath;
use futures_util::stream::{StreamExt};
use tokio::fs::{remove_file, rename, File};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::{
	State, Redirect, Path, IntoResponse, Response
};

type ParsedPostFields = (Option<u64>, Option<String>, Option<String>, Option<String>);


async fn handle_file_upload(
	mut field: Field<'_>
) -> Option<String> {
	let mut ext = field.file_name()
		.and_then(|name| {
			if name.is_empty() {
				return None;
			}
			StdPath::new(name).extension()
		})
    .and_then(|ext| ext.to_str())
    .unwrap_or("")
		.to_string();
	if ext != "".to_string() {
		ext = format!(".{}", ext)
	}
	let uuid = Uuid::new_v4();
	let name = format!("{}.stream", uuid);
	let temp_path = StdPath::new("attachments/").join(&name);
	
	let abort = || {
		tokio::spawn(remove_file(temp_path.to_owned()));
		None
	};

	let mut hasher = blake3::Hasher::new();
	let mut file = File::create(&temp_path).await.unwrap();
	let mut first = true;
	while let Some(chunk) = field.next().await {
		let bytes = match chunk {
			Ok(data) => data,
			Err(_) => return abort(),
		};
		if first {
			if bytes.is_empty() { return abort() }
			first = false;
		}
		hasher.update(&bytes);
		let _ = file.write_all(&bytes).await;
	}
	if file.flush().await.is_err() { return abort() }
	
	let mut output = [0u8; 16]; 
	hasher.finalize_xof().fill(&mut output);
	let file_hash = hex::encode(output);
	
	let name = format!("{}{}", file_hash, ext);
	let path = temp_path.with_file_name(&name);
	let _ = rename(temp_path, path).await;
	
	Some(name)
}


async fn parse_create_form(
	mut multipart: Multipart
) -> Result<ParsedPostFields, Response> {
	let mut reply: Option<u64> = None;
	let mut content: Option<String> = None;
	let mut attachments: Option<String> = None;
	let mut author: Option<String> = Some("tester".to_string());

	let mut uploaded: Vec<String> = vec![];
	
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
					Some(empty) if empty == "".to_string() => None,
					Some(nonempty) => Some(nonempty),
					None => None,
				};
			}
			"anonymous" => {
				let text = field.text().await.unwrap_or_default();
				if text == "on" || text == "true" {
					author = None;
				}
			}
			"attachments" => {
				if let Some(filename) = handle_file_upload(field).await {
					uploaded.push(filename);
				}
			}
			&_ => ()
		}
	}
	if !uploaded.is_empty() {
		attachments = Some(uploaded.join(","));
	}
	if attachments.is_none() && content.is_none() {
		return Err((StatusCode::BAD_REQUEST, "No valid content provided.").into_response());
	}

	Ok((reply, content, attachments, author))
}


pub async fn create_thread(
	state: State,
	Path(location): Path<Vec<String>>,
	multipart: Multipart
) -> Result<Response, Response> {
	let (
		reply,
		content,
		attachments,
		author,
	) = parse_create_form(multipart).await?;
	match &location[..] {
		[board] => {
			match state.post.create(
				board.to_string(),
				None,
				reply,
				content,
				attachments,
				author,
			).await {
				Ok(_) => return Ok(Redirect::to(format!("/{}", board).as_str()).into_response()),
				Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Error during database call.").into_response()),
			}
		},
		_ => {
			return Err((StatusCode::BAD_REQUEST, "Invalid form URL.").into_response());
		}
	}
}

pub async fn create_post(
	state: State,
	Path(location): Path<Vec<String>>,
	multipart: Multipart
) -> Result<Response, Response> {
	let (
		reply,
		content,
		attachments,
		author,
	) = parse_create_form(multipart).await?;
	match &location[..] {
		[board, thread] => {
			match thread.parse::<u64>() {
				Ok(thread) => {
					match state.post.get(thread).await {
						Some(_) => {
							match state.post.create(
								board.to_string(),
								Some(thread),
								reply,
								content,
								attachments,
								author,
							).await {
								Ok(_) => Ok(Redirect::to(format!("/{}/{}", board, thread).as_str()).into_response()),
								Err(err) => Err((StatusCode::OK, format!("{}", err)).into_response()),
							}
						}
						None => return Err((StatusCode::BAD_REQUEST, "Invalid thread ID.").into_response()),
					}
				}
				Err(_) => {
					return Err((StatusCode::BAD_REQUEST, "Invalid thread.").into_response());
				}
			}
		}
		_ => {
			return Err((StatusCode::BAD_REQUEST, "Invalid form URL.").into_response());
		}
	}
}