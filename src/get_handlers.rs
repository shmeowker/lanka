use crate::{ForumTemplate, HtmlTemplate, IntoResponse, LState, Path, StatusCode};

pub async fn render_board(
	Path(board): Path<String>,
	state: LState,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let posts = state.post.board(&board).await;

	let template = ForumTemplate::new(state, vec![format!("{board}")], posts).await;

	Ok(HtmlTemplate(template))
}

pub async fn render_thread(
	Path((board, thread)): Path<(String, u64)>,
	state: LState,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let posts = state.post.thread(&thread).await;

	let template =
		ForumTemplate::new(state, vec![format!("{board}"), format!("{thread}")], posts).await;

	Ok(HtmlTemplate(template))
}
