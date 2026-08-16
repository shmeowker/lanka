#![allow(dead_code)]
#![allow(unused)]

use askama::Template;
use axum::{
	Router,
	extract::{
		DefaultBodyLimit,
		FromRef,
		FromRequestParts,
		Multipart,
		Path,
		State
	},
	http::StatusCode,
	response::{Html, IntoResponse, Redirect, Response},
	routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, Utc};
use derive_more::Deref;
use serde::Deserialize;
use sqlx::{
	AssertSqlSafe,
	FromRow,
	MySqlPool,
	SqlSafeStr,
	SqlStr
};
use std::{
	error::Error,
	net::SocketAddr,
	sync::Arc,
};
use tower_http::services::ServeDir;

mod auth;
mod handlers;
mod managers;

use auth::*;
use handlers::*;
use managers::*;

static TITLE: &str = "Lanka";
static DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
static UPLOAD_SIZE_LIMIT: usize = 100 * 1048576; // N * 1 MB
static HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 8888);


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let shared_state = Arc::new(AppState::new().await);
	let app = Router::<Arc<AppState>>::new()
		.nest_service("/assets", ServeDir::new("assets"))
		.nest_service("/static", ServeDir::new("static"))
		.route("/login", post(login))
		.route("/", get(index))
		.route("/{board}", get(render_board).post(create_thread))
		.route("/{board}/{thread}", get(render_thread).post(create_post))
		.with_state(shared_state)
		.layer(DefaultBodyLimit::max(UPLOAD_SIZE_LIMIT));

	let config = RustlsConfig::from_pem_file("cert.pem", "key.pem").await?;
	let addr = SocketAddr::from(HOST);

	axum_server::bind_rustls(addr, config)
		.serve(app.into_make_service())
		.await?;

	Ok(())
}

/// Holds all database queries
#[derive(Clone, Copy)]
enum DatabaseQuery {
	// AttachmentManager queries
	//ListAttachmentsByName,
	//DeleteAttachmentsByName,
	//DeleteAttachmentById,
	ListAttachmentsForPost,
	CreateAttachment,
	//ListOrphanedAttachments,
	//DeleteOrphanedAttachments,
	
	// BoardManager queries
	GetBoardByName,
	ListBoards,
	BoardExists,
	ListThemes,
	ListBoardsByTheme,
	//CreateBoard,
	//EditBoard,
	//DeleteBoard,
	//CreateTheme,
	//DeleteTheme,
	
	// PostManager queries
	GetPost,
	ListThreads,
	ListThreadPosts,
	CreatePost,
	BumpThread,
	//SetThreadPin,
	//SetThreadLock,
	//DeletePost,
	//DeleteThread,
	//MoveThread,
	
	// UserManager queries
	GetUserById,
	GetUserByName,
	GetUserByLogin,
	CreateUser,
	//ChangeUserEmail,
	//ChangeUserName,
	//ChangeUserPassword,
	
	// SessionManager queries
	CreateSession,
	RenewSession,
	GetSessionByToken,
	ListUserSessions,
	DeleteSessionByToken,
	DeleteSessionById,
}

impl DatabaseQuery {
	#[inline]
	const fn into_str(self) -> &'static str {
		// Attention: The queries below are for MariaDB (InnoDB).
		// Compatibility with other engines is not guaranteed.
		match self {
			// AttachmentManager queries
			Self::ListAttachmentsForPost => "select * from attachments where post = ?",
			Self::CreateAttachment => "insert into attachments (post, name, size, original_name) values (?, ?, ?, ?)",
			// BoardManager queries
			Self::GetBoardByName => "select * from boards where name = ?",
			Self::ListBoards => "select * from boards",
			Self::BoardExists => "select exists(select 1 from boards where id = ?)",
			Self::ListThemes => "select * from themes",
			Self::ListBoardsByTheme => "select * from boards where theme = ?",
			
			// PostManager queries
			Self::GetPost => "select posts.*, (select json_arrayagg(json_object('name', a.name, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id) attachments from posts where id = ?",
			Self::ListThreads => "select posts.*, (select json_arrayagg(json_object('name', a.name, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id) attachments from posts where board = ? and ifnull(thread, 0) = 0 order by bumped desc",
			Self::ListThreadPosts => "select posts.*, (select json_arrayagg(json_object('name', a.name, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id) attachments from posts where id = ? or thread = ?;",
			Self::CreatePost => "insert into posts (board, thread, reply, content, author) values (?, ?, ?, ?, ?) returning id",
			Self::BumpThread => "update posts set bumped = current_timestamp() where id = ?",
			
			// UserManager queries
			Self::GetUserById => "select * from users where id = ?",
			Self::GetUserByName => "select * from users where name = ?",
			Self::GetUserByLogin => "select * from users where name = ? or email = ?",
			Self::CreateUser => "insert into users (name, password, email) values (?, ?, ?)",
			
			// SessionManager queries
			Self::CreateSession => "insert into sessions (user, token_hash) values (?, ?) returning *",
			Self::RenewSession => "update sessions set expires = date_add(current_timestamp() + interval 7 day) where token_hash = ?",
			Self::GetSessionByToken => "select * from sessions where token_hash = ?",
			Self::ListUserSessions => "select * from sessions where user = ?",
			Self::DeleteSessionByToken => "delete from sessions where token_hash = ?",
			Self::DeleteSessionById => "delete from sessions where id = ?",
		}
	}
}

impl SqlSafeStr for DatabaseQuery {
	#[inline]
	fn into_sql_str(self) -> SqlStr {
		AssertSqlSafe(self.into_str()).into_sql_str()
	}
}

type Rejection = (StatusCode, &'static str);
type LState = State<Arc<AppState>>;

struct AppState {
	board: BoardManager,
	post: PostManager,
	user: UserManager,
	session: SessionManager,
}

impl AppState {
	async fn new() -> Self {
		let pool = MySqlPool::connect(DATABASE)
			.await
			.expect("Failed to connect to the database.");

		Self {
			board: BoardManager::new(&pool),
			post: PostManager::new(&pool),
			user: UserManager::new(&pool),
			session: SessionManager::new(pool),
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
			Ok(html) => Html(html).into_response(),
			Err(err) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("Failed to render template: {err}"),
			).into_response(),
		}
	}
}
