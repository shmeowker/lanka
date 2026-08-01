use crate::{DatabaseQuery, Deserialize, FromRow, MySqlPool};

#[derive(FromRow, Deserialize)]
pub struct Board {
	pub name: String,
	pub title: String,
	pub description: Option<String>,
	pub locked: Option<bool>,
}
#[derive(Clone)]
pub struct BoardManager {
	pub pool: MySqlPool,
}
impl BoardManager {
	pub async fn list(&self) -> Vec<Board> {
		sqlx::query_as::<_, Board>(DatabaseQuery::ListBoards)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![])
	}
	pub async fn exists(&self, name: String) -> bool {
		sqlx::query_scalar(DatabaseQuery::BoardExists)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.unwrap_or(false)
	}
}
