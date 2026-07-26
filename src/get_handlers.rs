use super::*;

pub async fn render_board(
	Path(board): Path<String>,
	state: State,
) -> Result<HtmlTemplate<BoardTemplate>, (StatusCode, String)> {
	let posts = state.post.board(&board).await;

	let template = BoardTemplate::new(
		state,
		vec![format!("{board}")],
		posts,
	).await;

	Ok(HtmlTemplate(template))
}


pub async fn render_thread(
	Path((board, thread)): Path<(String, u64)>,
	state: State,
) -> Result<HtmlTemplate<BoardTemplate>, (StatusCode, String)> {
	let posts = state.post.thread(&thread).await;
	
	let template = BoardTemplate::new(
		state,
		vec![
			format!("{board}"),
			format!("{thread}"),
		],
		posts,
	).await;

	Ok(HtmlTemplate(template))
}
