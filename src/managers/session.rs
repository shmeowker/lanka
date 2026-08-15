use axum::http::HeaderValue;
use moka::future::Cache;
use rand::distr::{Alphanumeric, SampleString};
use std::time::Duration;

use crate::{
	DatabaseQuery,
	DateTime,
	Deserialize,
	FromRow,
	MySqlPool,
	Utc
};


#[derive(FromRow, Deserialize, Clone, PartialEq)]
pub struct Session {
	pub id: u64,
	pub user: u64,
	pub token_hash: String,
	pub created: DateTime<Utc>,
	pub expires: DateTime<Utc>,
	pub last_active: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SessionManager {
	pool: MySqlPool,
	cache: Cache<String, Session>,
}

impl SessionManager {
	pub fn new(pool: MySqlPool) -> Self {
		let cache = Cache::builder()
      .max_capacity(1024)
      .time_to_live(Duration::from_secs(600))
    	.build();
		Self {
			pool: pool,
			cache: cache,
		}
	}
	fn hash_token(token: &str) -> String {
		blake3::hash(token.as_bytes()).to_string()
	}
	fn generate_token() -> String {
		Alphanumeric.sample_string(&mut rand::rng(), 64)
	}
	/// Returns a `String` containing a fresh token on success.
	pub async fn create(&self, user_id: u64) -> Result<String, sqlx::Error> {
		let token = Self::generate_token();
		let token_hash = Self::hash_token(&token.as_ref());
		let session = sqlx::query_as::<_, Session>(DatabaseQuery::CreateSession)
			.bind(user_id)
			.bind(&token_hash)
			.fetch_one(&self.pool)
			.await?;
		self.cache.insert(token.clone(), session).await;
		Ok(token)
	}
	/// Extend a session's expiration
	pub async fn renew(&self, token: &str) -> Result<(), sqlx::Error> {
		let token_hash = Self::hash_token(token);
		let session = sqlx::query_as::<_, Session>(DatabaseQuery::RenewSession)
			.bind(token_hash)
			.fetch_one(&self.pool)
			.await?;
		self.cache.insert(token.to_string(), session).await;
		Ok(())
	}
	pub async fn get_by_token(&self, token: &str) -> Option<Session> {
		if let Some(session) = self.cache.get(&token.to_owned()).await {
			return Some(session);
		}
		let token_hash = Self::hash_token(token);
		let session = sqlx::query_as::<_, Session>(DatabaseQuery::GetSessionByToken)
			.bind(token_hash)
			.fetch_one(&self.pool)
			.await
			.ok();
		if let Some(ref session) = session {
			self.cache.insert(token.to_string(), session.clone()).await;
		}
		session
	}
	pub async fn list_by_user_id(&self, user_id: u64) -> Vec<Session> {
		sqlx::query_as::<_, Session>(DatabaseQuery::ListUserSessions)
			.bind(user_id)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![])
	}
	pub async fn delete_by_token(&self, token: &str) -> Result<(), sqlx::Error> {
		let token_hash = Self::hash_token(token);
		sqlx::query(DatabaseQuery::DeleteSessionByToken)
			.bind(token_hash)
			.execute(&self.pool)
			.await?;
		self.cache.invalidate(&token.to_string()).await;
		Ok(())
	}
	pub async fn delete_by_id(&self, id: u64) -> Result<(), sqlx::Error> {
		sqlx::query(DatabaseQuery::DeleteSessionById)
			.bind(id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
}
