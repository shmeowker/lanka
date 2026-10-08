#![allow(clippy::redundant_field_names)]
#![allow(dead_code)]
#![warn(unused_imports)]

use askama::Template;
use axum::{
    Router,
    extract::{DefaultBodyLimit, FromRef, FromRequestParts, Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, Utc};
use derive_more::Deref;
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool};
use std::{
    error::Error,
    net::SocketAddr,
    sync::Arc,
};
use tower_http::services::ServeDir;

mod errors;
mod queries;
mod auth;
mod handlers;
mod managers;

use errors::*;
use queries::*;
use auth::*;
use handlers::*;
use managers::*;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const TITLE: &str = "Lanka";
const DATABASE: &str = "mysql://root:password@127.0.0.1:3306/lanka";
const UPLOAD_SIZE_LIMIT: usize = 100 * 1048576; // N * 1 MB
const POST_CONTENT_SIZE_LIMIT: usize = 4 * 1024; // N * 1 KB
const HOST: ([u8; 4], u16) = ([127, 0, 0, 1], 8888);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let shared_state = Arc::new(AppState::new().await);
    let app = Router::<Arc<AppState>>::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(index))
        .route("/login", post(login))
        .route("/logout", post(logout))
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
            )
                .into_response(),
        }
    }
}