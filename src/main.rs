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
use sqlx::{MySqlPool, FromRow};
use std::{
	net::SocketAddr, 
	sync::Arc,
	error::Error,
};
use tower_http::services::ServeDir;
use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, Utc};
use derive_more::Deref;
//use tokio::task_local;

mod form_handlers;
mod get_handlers;

use form_handlers::*;
use get_handlers::*;

const TITLE: &str = "Lanka";
const DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
const UPLOAD_SIZE_LIMIT: usize = 100 * 1048576;
const HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 8888);


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
	println!("Starting server on https://{}", addr);
	axum_server::bind_rustls(addr, config)
		.serve(app.into_make_service())
		.await?;
	Ok(())
}



/*
#[derive(FromRow, Serialize, Deserialize, Clone, Debug, Default)]
struct User {
	id: u64,
	name: String,
	admin: bool,
	password: Option<String>,
	email: Option<String>,
}
struct UserManager {
	pool: &MySqlPool,
}
impl UserManager {
	fn new(state: State) -> Self {
		Self { &state.pool }
	}
	fn anonymous() -> User {
		User { 
			name: "Anon".to_string(), 
			..Default::default()
		}
	}
	async fn get(self: Self, id: u64) -> Option<User> {
		let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?;")
			.bind(id)
			.fetch_optional(self.pool)
			.await
			.ok()
			.flatten();
		Some(user?)
	}
	async fn create(self: Self, name: String, password: String, email: String) {
		sqlx::query("INSERT INTO users(name, password, email) values(?, ?, ?);")
			.bind(name)
			.bind(password)
			.bind(email)
			.execute(self.pool);
	}
}
*/


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
		let query: &str = "SELECT * FROM boards";
		let boards = sqlx::query_as::<_, Board>(query)
			.fetch_all(&self.pool)
			.await
			.unwrap();
		return boards;
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
	post: PostData,
	#[deref(ignore)]
	op: bool,
	#[deref(ignore)]
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
		let query: &str = "SELECT * FROM posts WHERE id = ?";
		let post = sqlx::query_as::<_, PostData>(query)
			.bind(id)
			.fetch_one(&self.pool)
			.await;
		return post.ok();
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
		sqlx::query("INSERT INTO posts(board, thread, reply, content, attachments, author) values(?, ?, ?, ?, ?, ?)")
			.bind(board)
			.bind(&thread)
			.bind(reply)
			.bind(content)
			.bind(attachments)
			.bind(author)
			.execute(&self.pool)
			.await?;
		if thread.is_some() {
			sqlx::query("UPDATE posts SET bumped = current_timestamp() WHERE id = ?")
				.bind(thread)
				.execute(&self.pool)
				.await?;
		}
		Ok(())
	}
	async fn board(&self, board: &String) -> Vec<Post<ThreadTemplate>> {
		let query: &str = "SELECT * FROM posts WHERE board = ? AND IFNULL(thread, 0) = 0 ORDER BY bumped DESC";
		let raw = sqlx::query_as::<_, PostData>(query)
			.bind(&board)
			.fetch_all(&self.pool)
			.await
			.unwrap();
		let threads: Vec<Post<ThreadTemplate>> = raw
			.into_iter()
			.map(|post| Post {
				data: post.clone(), 
				template: ThreadTemplate(post)
			})
			.collect();
		return threads;
	}
	async fn thread(&self, thread: &u64) -> Vec<Post<PostTemplate>> {
		let query: &str = "SELECT * FROM posts WHERE (id = ? AND IFNULL(thread, 0) = 0) OR thread = ?";
		let raw = sqlx::query_as::<_, PostData>(query)
			.bind(&thread)
			.bind(&thread)
			.fetch_all(&self.pool)
			.await
			.unwrap();
		let mut op_posts: Vec<u64> = vec![];
		if let Some(first) = raw.get(0) {
			let op_name = first.author.clone();
			op_posts = raw
				.iter()
				.filter_map(|post| {
					if post.author.is_some() && op_name.is_some() && post.author == op_name {
						return Some(post.id);
					}
					None
				})
				.collect();
		}
		raw.into_iter()
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
					|item|
					item.template.render().unwrap()
				)
				.collect(),
		}
	}
}


