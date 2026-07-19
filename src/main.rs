use askama::Template;
use axum::{
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
	middleware::{self, Next},
	routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, FromRow};
use std::{
	net::SocketAddr, 
	sync::Arc, 
	error::Error
};
use tower_http::services::ServeDir;
use axum_server::tls_rustls::RustlsConfig;
//use tokio::task_local;
use chrono::{DateTime, Utc};
use derive_more::Deref;

mod helpers;

const TITLE: &str = "Lanka";
const DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
const UPLOAD_SIZE_LIMIT: usize = 100 * 1048576;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let shared_state = Arc::new(AppState::new().await);
	let method_filter_layer = middleware::from_fn_with_state(
		shared_state.clone(), distributor
	);
	let mut app = Router::new()
		.nest_service("/static", ServeDir::new("static"))
		.nest_service("/attachments", ServeDir::new("attachments"))
		.route("/{board}", get(render_board))
		.route("/{board}/{thread}", get(render_thread))
		.layer(method_filter_layer)
		.layer(DefaultBodyLimit::max(UPLOAD_SIZE_LIMIT))
		.layer(tower_livereload::LiveReloadLayer::new())
		.with_state(shared_state);
	let config = RustlsConfig::from_pem_file(
		"cert.pem", "key.pem"
	).await?;
	let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	println!("Starting server on https://{}", addr);
	axum_server::bind_rustls(addr, config)
		.serve(app.into_make_service())
		.await?;
	Ok(())
}


async fn distributor(
	state: State,
	request: Request<Body>,
	next: Next
) -> impl IntoResponse {
	if request.method() == Method::POST {
		let (mut parts, body) = request.into_parts();
		let path = match Path::<Vec<String>>::from_request_parts(&mut parts, &state).await {
			Ok(p) => p,
			Err(err) => return err.into_response(),
		};
		let multipart = match Multipart::from_request(Request::from_parts(parts, body), &state).await {
			Ok(m) => m,
			Err(err) => return err.into_response(),
		};
		return form_handler(state, path, multipart).await;
	}
	return next.run(request).await;
}


async fn form_handler(
	state: State,
	Path(location): Path<Vec<String>>,
	mut multipart: Multipart
) -> Response<Body> {
	let mut thread: Option<u64> = None;
	let mut reply: Option<u64> = None;
	let mut content: Option<String> = None;
	let mut attachments: Option<String> = None;
	let mut author: Option<String> = Some("tester".to_string());

	let mut uploaded: Vec<String> = vec![];
	
	while let Some(field) = multipart.next_field().await.unwrap() {
		let name = field.name().unwrap_or_default().to_string();
		match name.as_str() {
			"reply" => {
				if let Ok(number) = field.text().await.unwrap_or_default().parse::<u64>() {
					if let Some(reply_post) = state.post.get(number).await {
						reply = Some(reply_post.id);
					}
				}
			}
			"content" => {
				content = match field.text().await.ok() {
					Some(empty) if empty == "".to_string() => None,
					Some(nonempty) => Some(nonempty),
					None => None,
				};
			}
			"anonymous" => {
				let text = field.text().await.unwrap_or_default();
				if text == "on" || text == "true" {
					author = None;
				}
			}
			"attachments[]" => {
				if let Some(filename) = helpers::handle_file_upload(field).await {
					uploaded.push(filename);
				}
			}
			&_ => ()
		}
	}
	if uploaded.get(0).is_some() {
		attachments = Some(uploaded.join(","));
	}
	if attachments.is_none() && content.is_none() {
		return (StatusCode::BAD_REQUEST, "").into_response();
	}
	match &location[..] {
		[board] => {
			if let Ok(_) = state.post.create_thread(
				board.to_string(),
				reply,
				content,
				attachments,
				author,
			).await {
				return Redirect::to(format!("/{}", board).as_str()).into_response();
			} else {
				return (StatusCode::BAD_REQUEST, "").into_response();
			}
		}
		[board, thread] => {
			if let Ok(thread) = thread.parse::<u64>() {
				if None == state.post.get(thread).await {
					return (StatusCode::BAD_REQUEST, "").into_response();
				} else {
					let result = state.post.create_in_thread(
						board.to_string(),
						thread,
						reply,
						content,
						attachments,
						author,
					).await;
					match result {
						Ok(_) => Redirect::to(format!("/{}/{}", board, thread).as_str()).into_response(),
						Err(err) => (StatusCode::OK, format!("{}", err)).into_response(),
					}
				}
			} else {
				return (StatusCode::OK, "Invalid form data.").into_response();
			}
		}
		_ => {
			return (StatusCode::OK, "Invalid form data.").into_response();
		}
	}
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


#[derive(FromRow, Deserialize, Clone)]
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
	async fn create_thread(
		&self,
		board: String,
		reply: Option<u64>,
		content: Option<String>,
		attachments: Option<String>,
		author: Option<String>,
	) -> Result<(), sqlx::Error> {
		let result = sqlx::query("INSERT INTO posts(board, reply, content, attachments, author) values(?, ?, ?, ?, ?)")
			.bind(board)
			.bind(reply)
			.bind(content)
			.bind(attachments)
			.bind(author)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	async fn create_in_thread(
		&self, 
		board: String, 
		thread: u64,
		reply: Option<u64>,
		content: Option<String>,
		attachments: Option<String>,
		author: Option<String>,
	) -> Result<(), sqlx::Error> {
		let _ = sqlx::query("INSERT INTO posts(board, thread, reply, content, attachments, author) values(?, ?, ?, ?, ?, ?)")
			.bind(board)
			.bind(&thread)
			.bind(reply)
			.bind(content)
			.bind(attachments)
			.bind(author)
			.execute(&self.pool)
			.await?;
		let _ = sqlx::query("UPDATE posts SET bumped = current_timestamp() WHERE id = ?")
			.bind(thread)
			.execute(&self.pool)
			.await?;
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
		let mut op_name: Option<String> = None;
		let mut op_posts: Vec<u64> = vec![];
		if let Some(first) = raw.get(0) {
			op_name = first.author.clone();
			op_posts = raw
				.iter()
				.filter_map(|post| {
					if post.author.is_some() && op_name.is_some() {
						return Some(post.id);
					}
					None
				})
				.collect();
		}
		let posts: Vec<Post<PostTemplate>> = raw
			.into_iter()
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
			.collect();
		return posts;
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
			board: BoardManager { pool: pool.clone() },
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


async fn render_board(
	Path(board): Path<String>,
	state: State,
) -> Result<HtmlTemplate<BoardTemplate>, (StatusCode, String)> {
	let posts = state.post.board(&board).await;

	let template = BoardTemplate::new(
		state,
		vec![format!("{board}")],
		posts,
	).await;

	Ok(HtmlTemplate(template))
}


async fn render_thread(
	Path((board, thread)): Path<(String, u64)>,
	state: State,
) -> Result<HtmlTemplate<BoardTemplate>, (StatusCode, String)> {
	let posts = state.post.thread(&thread).await;
	
	let template = BoardTemplate::new(
		state,
		vec![
			format!("{board}"),
			format!("{thread}"),
		],
		posts,
	).await;

	Ok(HtmlTemplate(template))
}
