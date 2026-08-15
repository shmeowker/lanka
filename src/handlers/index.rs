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

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
	title: String,
	user: Option<User>,
	boards: Vec<Board>,
}

pub async fn index(state: LState, CurrentUser(user): CurrentUser) -> impl IntoResponse {
	let boards = state.board.list().await;
	let template = IndexTemplate {
		title: TITLE.to_string(),
		user: user,
		boards: boards,
	};
	HtmlTemplate(template)
}