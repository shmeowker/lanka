use crate::{
	Board,
	CurrentUser,
	HtmlTemplate,
	LState,
	IntoResponse,
	Response,
	Template,
	TITLE,
	User,
};

use ezlz::t;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
	stats: [u64; 4],
	user: Option<User>,
	boards: Vec<Board>,
}

pub async fn index(state: LState, CurrentUser(user): CurrentUser) -> impl IntoResponse {
	let boards = state.board.list().await;
	let forum_stats = [
		state.post.count_all().await,
		state.post.count_threads().await,
		boards.len() as u64,
		state.post.count_total().await,
	];
	let template = IndexTemplate {
		stats: forum_stats,
		user: user,
		boards: boards,
	};
	HtmlTemplate(template)
}