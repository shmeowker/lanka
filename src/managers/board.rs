use futures::future::join_all;

use crate::{
	DatabaseQuery,
	Deserialize,
	FromRow,
	MySqlPool
};


#[derive(FromRow, Deserialize)]
pub struct Theme {
	pub name: String,
}

pub struct ThemeBoards {
	pub theme: Theme,
	pub boards: Vec<Board>,
}

#[derive(FromRow, Deserialize, Clone)]
pub struct Board {
	pub name: String,
	pub theme: String,
	pub title: String,
	pub description: Option<String>,
	pub locked: Option<bool>,
}

#[derive(Clone)]
pub struct BoardManager {
	pool: MySqlPool,
}

impl BoardManager {
	pub fn new(pool: &MySqlPool) -> Self {
		Self { 
			pool: pool.clone(),
		}
	}
	pub async fn get(&self, name: &String) -> Option<Board> {
		sqlx::query_as::<_, Board>(DatabaseQuery::GetBoardByName)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	pub async fn list(&self) -> Vec<Board> {
		sqlx::query_as::<_, Board>(DatabaseQuery::ListBoards)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![])
	}
	pub async fn exists(&self, name: &String) -> bool {
		sqlx::query_scalar(DatabaseQuery::BoardExists)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.unwrap_or(false)
	}
	pub async fn list_by_theme(&self, theme: Theme) -> ThemeBoards {
		let boards = sqlx::query_as::<_, Board>(DatabaseQuery::ListBoardsByTheme)
			.bind(&theme.name)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![]);
		ThemeBoards {
			theme: theme,
			boards: boards,
		}
	}
	pub async fn sorted_by_themes(&self, theme: Theme) -> Vec<ThemeBoards> {
		let results = sqlx::query_as::<_, Theme>(DatabaseQuery::ListThemes)
			.bind(&theme.name)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![])
			.into_iter()
			.map(async |t| {
				self.list_by_theme(t).await
			})
			.collect::<Vec<_>>();
		join_all(results).await
	}
}
