use crate::{TITLE, HtmlTemplate, IntoResponse, LState, Path, StatusCode, Template, Board, Post, PostTemplate, PostKind};

#[derive(Template)]
#[template(path = "board.html")]
struct ForumTemplate {
	title: String,
	boards: Vec<Board>,
	breadcrumbs: Vec<String>,
	posts: String,
}
impl ForumTemplate {
	async fn new<P: PostKind + Template>(
		state: LState,
		breadcrumbs: Vec<String>,
		posts: Vec<Post<P>>,
	) -> Self {
		let boards = state.board.list().await;

		Self {
			title: TITLE.to_string(),
			boards: boards,
			breadcrumbs: breadcrumbs,
			posts: posts
				.iter()
				.map(|post| post.template.render().unwrap())
				.collect(),
		}
	}
}


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
