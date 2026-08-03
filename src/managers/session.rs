use rand::distr::{Alphanumeric, SampleString};
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
	pub pool: MySqlPool,
}

impl SessionManager {
	fn hash_token(token: &str) -> String {
		blake3::hash(token.as_bytes()).to_string()
	}
	fn generate_token() -> String {
		Alphanumeric.sample_string(&mut rand::rng(), 64)
	}
	/// Returns raw generated token
	pub async fn create(&self, user_id: u64) -> Result<String, sqlx::Error> {
		let token = Self::generate_token();
		let token_hash = Self::hash_token(&token.as_ref());
		sqlx::query(DatabaseQuery::CreateSession)
			.bind(&user_id)
			.bind(token_hash)
			.execute(&self.pool)
			.await?;
		Ok(token)
	}
	/// Extend a session's expiration by 7 days
	pub async fn renew(&self, token: &str) -> Result<(), sqlx::Error> {
		let token_hash = Self::hash_token(token);
		sqlx::query(DatabaseQuery::RenewSession)
			.bind(token_hash)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	pub async fn get_by_token(&self, token: &str) -> Option<Session> {
		let token_hash = Self::hash_token(token);
		sqlx::query_as::<_, Session>(DatabaseQuery::GetSessionByToken)
			.bind(token_hash)
			.fetch_one(&self.pool)
			.await
			.ok()
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
		Ok(())
	}
}
