#![allow(dead_code)]

use askama::Template;
use axum::{
	Router,
	http::{StatusCode},
	extract::{
		State,
		Multipart,
		Path,
		FromRef,
		FromRequestParts,
		DefaultBodyLimit,
	},
	response::{Html, IntoResponse, Response, Redirect},
	routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize};
use sqlx::{
	MySqlPool,
	FromRow,
	SqlSafeStr,
	AssertSqlSafe,
	SqlStr,
};
use std::{
	net::SocketAddr, 
	sync::Arc,
	error::Error,
};
use tower_http::services::ServeDir;
use axum_server::tls_rustls::RustlsConfig;
use derive_more::Deref;

mod form_handlers;
mod get_handlers;

mod managers;
mod auth;

use managers::{
	user::*,
	post::*,
	session::*,
	board::*,
};

use auth::*;

use form_handlers::*;
use get_handlers::*;

static TITLE: &str = "Lanka";
static DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
static UPLOAD_SIZE_LIMIT: usize = 100 * 1048576; // N * 1 MB
static HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 8888);


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let shared_state = Arc::new(AppState::new().await);
	let app = Router::<Arc<AppState>>::new()
		.nest_service("/static", ServeDir::new("static"))
		.nest_service("/attachments", ServeDir::new("attachments"))
		.route("/{board}", get(render_board).post(create_thread))
		.route("/{board}/{thread}", get(render_thread).post(create_post))
		.with_state(shared_state)
		.layer(DefaultBodyLimit::max(UPLOAD_SIZE_LIMIT));
	
	let config = RustlsConfig::from_pem_file(
		"cert.pem", "key.pem"
	).await?;
	let addr = SocketAddr::from(HOST);
	
	axum_server::bind_rustls(addr, config)
		.serve(app.into_make_service())
		.await?;
	
	Ok(())
}

#[derive(Clone, Copy)]
enum DatabaseQuery {
	ListBoards,
	BoardExists,
	GetPost,
	ListThreads,
	ListThreadPosts,
	CreatePost,
	BumpThread,
	GetUserById,
	GetUserByName,
	CreateUser,
	GetSessionByToken,
	ListUserSessions,
	DeleteSessionByToken,
}
impl DatabaseQuery {
	#[inline]
	fn into_str(self) -> &'static str {
		match self {
			Self::ListBoards => "select * from boards",
			Self::BoardExists => "select exists(select 1 from boards where id = ?)",
			Self::GetPost => "select * from posts where id = ?",
			Self::ListThreads => "select * from posts where board = ? and ifnull(thread, 0) = 0 order by bumped desc",
			Self::ListThreadPosts => "select * from posts where id = ? or thread = ?",
			Self::CreatePost => "insert into posts (board, thread, reply, content, attachments, author) values (?, ?, ?, ?, ?, ?)",
			Self::BumpThread => "update posts SET bumped = current_timestamp() where id = ?",
			Self::GetUserById => "select * from users where id = ?",
			Self::GetUserByName => "select * from users where name = ?",
			Self::CreateUser => "insert into users (name, password, email) values (?, ?, ?)",
			Self::GetSessionByToken => "select * from sessions where token_hash = ?",
			Self::ListUserSessions => "select * from sessions where user = ?",
			Self::DeleteSessionByToken => "delete from sessions where token_hash = ?",
		}
	}
}
impl SqlSafeStr for DatabaseQuery {
	#[inline]
	fn into_sql_str(self) -> SqlStr {
		AssertSqlSafe(self.into_str()).into_sql_str()
	}
}


type LState = State<Arc<AppState>>;
#[derive(FromRef, Clone)]
struct AppState {
	post: PostManager,
	board: BoardManager,
	user: UserManager,
	session: SessionManager,
}
impl AppState {
	async fn new() -> Self {
		let pool = MySqlPool::connect(DATABASE)
			.await
			.expect("Failed to connect to the database.");
		
		Self { 
			post: PostManager { pool: pool.clone() },
			board: BoardManager { pool: pool.clone() },
			user: UserManager { pool: pool.clone() },
			session: SessionManager { pool: pool },
		}
	}
}


struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where 
	T: Template,
{
	fn into_response(self) -> Response {
		match self.0.render() {
			//Ok(html) => (StatusCode::OK, html).into_response(),
			Ok(html) => Html(html).into_response(),
			Err(err) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("Failed to render template: {err}"),
			).into_response(),
		}
	}
}


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
		posts: Vec<Post<P>>
	) -> Self {
		let boards = state.board.list().await;
		
		Self {
			title: TITLE.to_string(),
			boards: boards,
			breadcrumbs: breadcrumbs,
			posts: posts
				.iter()
				.map(
					|post|
					post.template.render().unwrap()
				)
				.collect(),
		}
	}
}


