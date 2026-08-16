use crate::{
	Board,
	CurrentUser,
	HtmlTemplate,
	IntoResponse,
	LState,
	ORejection,
	Path,
	Post,
	PostKind,
	PostTemplate,
	StatusCode,
	TITLE,
	Template,
	User,
};


#[derive(Template)]
#[template(path = "forum.html")]
struct ForumTemplate {
	boards: Vec<Board>,
	board: Board,
	thread: Option<u64>,
	posts: String,
	user: Option<User>,
}

impl ForumTemplate {
	async fn new<P: PostKind + Template>(
		state: LState,
		board: Board,
		thread: Option<u64>,
		posts: Vec<Post<P>>,
		user: Option<User>,
	) -> Self {
		let boards = state.board.list().await;

		Self {
			boards: boards,
			board: board,
			thread: thread,
			posts: posts
				.iter()
				.map(|post| post.render().unwrap())
				.collect(),
			user: user,
		}
	}
}


pub async fn render_board(
	Path(board): Path<String>,
	state: LState,
	CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, ORejection> {
	let posts = state.post.board(&board).await;
	let Some(board) = state.board.get(&board).await else {
		return Err((StatusCode::NOT_FOUND, format!("Board '{board}' does not exists.")));
	};

	let template = ForumTemplate::new(
		state,
		board,
		None,
		posts,
		user
	).await;

	Ok(HtmlTemplate(template))
}

pub async fn render_thread(
	Path((board, thread)): Path<(String, u64)>,
	state: LState,
	CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, ORejection> {
	let posts = state.post.thread(&thread).await;
	let Some(board) = state.board.get(&board).await else {
		return Err((StatusCode::NOT_FOUND, format!("Board '{board}' does not exists.")));
	};

	let template = ForumTemplate::new(
		state,
		board,
		Some(thread),
		posts,
		user
	).await;

	Ok(HtmlTemplate(template))
}
