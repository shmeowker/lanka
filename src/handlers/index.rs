use crate::{
	Board,
	CurrentUser,
	HtmlTemplate,
	LState,
	IntoResponse,
	Template,
	TITLE,
	User,
};

struct ForumStats {
    post_count: u64,
    thread_count: u64,
    board_count: usize,
    total_posts: u64,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
	stats: ForumStats,
	user: Option<User>,
	boards: Vec<Board>,
}

pub async fn index(state: LState, CurrentUser(user): CurrentUser) -> impl IntoResponse {
	let boards = state.board.list().await;
	let forum_stats = ForumStats {
		post_count: state.post.count_all().await,
		thread_count: state.post.count_threads().await,
		board_count: boards.len(),
		total_posts: state.post.count_total().await,
    };
	let template = IndexTemplate {
		stats: forum_stats,
		user: user,
		boards: boards,
	};
	HtmlTemplate(template)
}