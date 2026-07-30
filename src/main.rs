#![allow(dead_code)]

pub use askama::Template;
pub use axum::{
	Router,
	body::Body,
	http::{StatusCode, Method},
	extract::{
		State as AxumState, 
		Multipart, 
		Path, 
		Request, 
		FromRequest, 
		FromRequestParts,
		DefaultBodyLimit,
	},
	response::{Html, IntoResponse, Json, Response, Redirect},
	middleware::Next,
	routing::{get, post},
};
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
use chrono::{DateTime, Utc};
use derive_more::Deref;

mod form_handlers;
mod get_handlers;

use form_handlers::*;
use get_handlers::*;

static TITLE: &str = "Lanka";
static DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
static UPLOAD_SIZE_LIMIT: usize = 100 * 1048576; // N * 1 MB
static HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 8888);


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let shared_state = Arc::new(AppState::new().await);
	let app = Router::new()
		.nest_service("/static", ServeDir::new("static"))
		.nest_service("/attachments", ServeDir::new("attachments"))
		.route("/{board}", get(render_board).post(create_thread))
		.route("/{board}/{thread}", get(render_thread).post(create_post))
		.layer(DefaultBodyLimit::max(UPLOAD_SIZE_LIMIT))
		.with_state(shared_state);
	
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
}
impl DatabaseQuery {
	fn as_str(&self) -> &str {
		match &self {
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
		}
	}
}
impl SqlSafeStr for DatabaseQuery {
	fn into_sql_str(self) -> SqlStr {
		AssertSqlSafe(self.as_str()).into_sql_str()
	}
}


#[derive(FromRow, Deserialize, Clone, Default)]
struct User {
	id: Option<u64>,
	name: String,
	password: String,
	email: Option<String>,
	admin: bool,
}
struct UserManager {
	pool: MySqlPool,
}
impl UserManager {
	async fn get_by_id(&self, id: u64) -> Option<User> {
		let query = DatabaseQuery::GetUserById;
		sqlx::query_as::<_, User>(query)
			.bind(id)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	async fn get_by_name(&self, name: String) -> Option<User> {
		let query = DatabaseQuery::GetUserByName;
		sqlx::query_as::<_, User>(query)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	async fn create(
		&self,
		name: String,
		password: String,
		email: Option<String>
	) -> Result<(), sqlx::Error> {
		let query = DatabaseQuery::CreateUser;
		let _ = sqlx::query(query)
			.bind(name)
			.bind(password)
			.bind(email)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
}


#[derive(FromRow, Deserialize)]
struct Board {
	name: String,
	title: String,
	description: Option<String>,
	locked: Option<bool>,
}
#[derive(Clone)]
struct BoardManager {
	pool: MySqlPool,
}
impl BoardManager {
	async fn list(&self) -> Vec<Board> {
		let query = DatabaseQuery::ListBoards;
		sqlx::query_as::<_, Board>(query)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![])
	}
	async fn exists(&self, name: String) -> bool {
		let query = DatabaseQuery::BoardExists;
		sqlx::query_scalar(query)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.unwrap_or(false)
	}
}


trait PostWrapper {
	fn get_template(&self) -> PostData;
}
impl<T> PostWrapper for T where 
	T: std::ops::Deref<Target = PostData>,
	{
		fn get_template(&self) -> PostData {
			(*self).clone()
		}
	}

struct Post<W: PostWrapper> {
	data: PostData,
	template: W,
}
#[derive(FromRow, Deserialize, Clone, PartialEq)]
struct PostData {
	id: u64,
	board: String,
	thread: Option<u64>,
	content: Option<String>,
	attachments: Option<String>,
	reply: Option<u64>,
	bumped: DateTime<Utc>,
	created: DateTime<Utc>,
	author: Option<String>,
	pinned: Option<bool>,
	locked: Option<bool>,
}

#[derive(Template, Deref)]
#[template(path = "post.html")]
struct PostTemplate {
	#[deref]
	post: PostData,
	op: bool,
	reply_op: bool,
}

#[derive(Template, Deref)]
#[template(path = "thread.html")]
struct ThreadTemplate(PostData);

#[derive(Clone)]
struct PostManager {
	pool: MySqlPool,
}
impl PostManager {
	async fn get(&self, id: u64) -> Option<PostData> {
		let query = DatabaseQuery::GetPost;
		sqlx::query_as::<_, PostData>(query)
			.bind(id)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	async fn create(
		&self, 
		board: String, 
		thread: Option<u64>,
		reply: Option<u64>,
		content: Option<String>,
		attachments: Option<String>,
		author: Option<String>,
	) -> Result<(), sqlx::Error> {
		let query = DatabaseQuery::CreatePost;
		sqlx::query(query)
			.bind(board)
			.bind(&thread)
			.bind(reply)
			.bind(content)
			.bind(attachments)
			.bind(author)
			.execute(&self.pool)
			.await?;
		if thread.is_some() {
			let query = DatabaseQuery::BumpThread;
			sqlx::query(query)
				.bind(thread)
				.execute(&self.pool)
				.await?;
		}
		Ok(())
	}
	async fn board(&self, board: &String) -> Vec<Post<ThreadTemplate>> {
		let query = DatabaseQuery::ListThreads;
		let data = sqlx::query_as::<_, PostData>(query)
			.bind(&board)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![]);
		data.into_iter()
			.map(|post| Post {
				data: post.clone(), 
				template: ThreadTemplate(post)
			})
			.collect()
	}
	async fn thread(&self, thread: &u64) -> Vec<Post<PostTemplate>> {
		let query = DatabaseQuery::ListThreadPosts;
		let data = sqlx::query_as::<_, PostData>(query)
			.bind(&thread)
			.bind(thread)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![]);
		let mut op_posts: Vec<u64> = vec![];
		if let Some(ref init) = data.get(0) {
			op_posts = data
				.iter()
				.filter_map(
					|post| {
					if post.author.is_some() && post.author == init.author {
						return Some(post.id);
					}
					None
				})
				.collect();
		}
		data.into_iter()
			.map(|post| {
				let mut op = false;
				let mut reply_op = false;
				if op_posts.contains(&post.id) {
					op = true;
				}
				if let Some(reply) = &post.reply.as_ref() {
					reply_op = op_posts.contains(&reply);
				}
				Post {
					data: post.clone(), 
					template: PostTemplate { 
						post: post,
						op: op,
						reply_op: reply_op,
					}
				}
			})
			.collect()
	}
}

// #[derive(FromRow, Serialize, Deserialize, Clone)]
// struct Session {
// 	id: u64,
// 	user: u64,
// 	token_hash: String,
// 	created: u64,
// 	expires: u64,
// 	last_active: u64,
// 	ip: String,
// }


type State = AxumState<Arc<AppState>>;
struct AppState {
	post: PostManager,
	board: BoardManager,
}
impl AppState {
	async fn new() -> Self {
		let pool = MySqlPool::connect(DATABASE)
			.await
			.expect("Failed to connect to the database.");
		
		Self { 
			post: PostManager { pool: pool.clone() },
			board: BoardManager { pool: pool },
		}
	}
}


struct HtmlTemplate<T>(T);
impl<T> IntoResponse for HtmlTemplate<T> where T: Template,
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


// task_local! {
// 	pub static USER: User;
// }

// async fn auth(req: Request, next: Next) -> Result<Response, StatusCode> {
// 	let auth_header = req
// 		.headers()
// 		.get(header::AUTHORIZATION)
// 		.and_then(|header| header.to_str().ok())
// 		.ok_or(StatusCode::UNAUTHORIZED)?;
// 	if let Some(current_user) = authorize_current_user(auth_header).await {
// 		// State is setup here in the middleware
// 		Ok(USER.scope(current_user, next.run(req)).await)
// 	} else {
// 		Err(StatusCode::UNAUTHORIZED)
// 	}
// }

// async fn authorize_current_user(token: &str) -> Option<User> {
// }


#[derive(Template)]
#[template(path = "board.html")]
struct BoardTemplate {
	title: String,
	boards: Vec<Board>,
	breadcrumbs: Vec<String>,
	posts: String,
}
impl BoardTemplate {
	async fn new<W: PostWrapper + Template>(
		state: State,
		breadcrumbs: Vec<String>,
		posts: Vec<Post<W>>
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


