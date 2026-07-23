use axum::extract::multipart::{Field, MultipartError};
use axum::http::StatusCode;
use axum::body::Bytes;
use futures_util::stream::{Stream, StreamExt};
//use std::pin::Pin;
use std::io::Write;
use std::path::Path;
use std::error::Error;
use tokio::fs::{remove_file, rename, File};
use tokio::io::AsyncWriteExt;
//use image::{ImageReader, codecs::jpeg::JpegEncoder};
use uuid::Uuid;


pub async fn handle_file_upload(
	mut field: Field<'_>
) -> Option<String> {
	let mut ext = field.file_name()
		.and_then(|name| {
			if name.is_empty() {
				return None;
			}
			Path::new(name).extension()
		})
    .and_then(|ext| ext.to_str())
    .unwrap_or("")
		.to_string();
	if ext != "".to_string() {
		ext = format!(".{}", ext)
	}
	let uuid = Uuid::new_v4();
	let mut name = format!("{}.stream", uuid);
	let temp_path = Path::new("attachments/").join(&name);

	let mut hasher = blake3::Hasher::new();
	let mut file = File::create(&temp_path).await.unwrap();
	let mut mime_type = "application/octet-stream".to_string();
	let mut first = true;
	while let Some(chunk) = field.next().await {
		let bytes = match chunk {
			Ok(data) => data,
			Err(_) => {
				remove_file(temp_path);
				return None;
			}
		};
		if first {
			if bytes.is_empty() {
				remove_file(temp_path).await;
				return None;
			}
			mime_type = match infer::get(&bytes) {
				Some(kind) => kind.mime_type().to_string(),
				None => "application/octet-stream".to_string(),
			};
			first = false;
		}
		hasher.update(&bytes);
		file.write_all(&bytes).await;
	}
	file.flush().await.ok();
	
	let mut output = [0u8; 16]; 
	hasher.finalize_xof().fill(&mut output);
	let file_hash = hex::encode(output);
	
	name = format!("{}{}", file_hash, ext);
	let path = temp_path.with_file_name(&name);
	rename(temp_path, path).await.ok();
	
	Some(name)
}



/*
pub async fn upload_thumbnail_handler(
	mut stream: FieldStream
) -> Result<Vec<u8>, (StatusCode, String)> {
	// Returns compressed JPEG blob

	let blob = tokio::task::spawn_blocking(move || {
		generate_video_thumbnail(&file_path)
	})
	.await
	.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Worker thread panicked".to_string()))?
	.map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

	drop(temp_file); 
	
	return Ok(compressed_jpeg_blob);
}


fn generate_video_thumbnail(
	path: &Path
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    ffmpeg_next::init()?;

    let mut input_context = ffmpeg_next::format::input(&path)?;
    
    let video_stream = input_context
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .ok_or("Could not find a valid video track or standard multimedia frame in file")?;
        
    let video_stream_index = video_stream.index();

    let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(video_stream.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    // Seek to 1-second timestamp milestone to skip potential black frames
    let seek_target = 1 * f64::from(ffmpeg_next::ffi::AV_TIME_BASE);
    let _ = input_context.seek(seek_target as i64, ..seek_target as i64);

    let mut scaler = ffmpeg_next::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg_next::format::Pixel::RGB24,
        256,
        256,
        ffmpeg_next::software::scaling::flag::Flags::BILINEAR,
    )?;

    let mut frame = ffmpeg_next::util::frame::Video::empty();
    let mut scaled_frame = ffmpeg_next::util::frame::Video::empty();

    for (stream, packet) in input_context.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            if decoder.receive_frame(&mut frame).is_ok() {
                scaler.run(&frame, &mut scaled_frame)?;
                return convert_to_jpeg_blob(scaled_frame.data(0));
            }
        }
    }

    Err("Failed to decode a viable visual frame before the end of stream".into())
}


fn convert_to_jpeg_blob(
	bytes: &[u8]
) -> Result<Vec<u8>, Box<dyn Error>> {
    let img = ImageReader::new(Cursor::new(bytes))
			.with_guessed_format()?
			.decode()?;
    let thumbnail = img.thumbnail(256, 256);
    let mut compressed = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut compressed, 50);
    thumbnail.write_with_encoder(encoder)?;
    Ok(compressed)
}
*/