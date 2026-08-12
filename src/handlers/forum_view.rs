use crate::{
	Board,
	CurrentUser,
	HtmlTemplate,
	IntoResponse,
	LState,
	Path,
	Post,
	PostKind,
	PostTemplate,
	StatusCode,
	TITLE,
	Template,
};


#[derive(Template)]
#[template(path = "forum.html")]
struct ForumTemplate {
	title: String,
	boards: Vec<Board>,
	breadcrumbs: Vec<String>,
	posts: String,
	user: CurrentUser,
}

impl ForumTemplate {
	async fn new<P: PostKind + Template>(
		state: LState,
		breadcrumbs: Vec<String>,
		posts: Vec<Post<P>>,
		user: CurrentUser,
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
			user: user,
		}
	}
}


pub async fn render_board(
	Path(board): Path<String>,
	state: LState,
	user: CurrentUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let posts = state.post.board(&board).await;

	let template = ForumTemplate::new(
		state,
		vec![format!("{board}")],
		posts,
		user
	).await;

	Ok(HtmlTemplate(template))
}

pub async fn render_thread(
	Path((board, thread)): Path<(String, u64)>,
	state: LState,
	user: CurrentUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let posts = state.post.thread(&thread).await;

	let template = ForumTemplate::new(
		state,
		vec![format!("{board}"), format!("{thread}")],
		posts,
		user
	).await;

	Ok(HtmlTemplate(template))
}
