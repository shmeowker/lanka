
//use image::{ImageReader, codecs::jpeg::JpegEncoder};
//use std::pin::Pin;




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