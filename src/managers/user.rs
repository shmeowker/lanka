use crate::{DatabaseQuery, Deserialize, FromRow, MySqlPool};

#[derive(FromRow, Deserialize, Clone, Default)]
pub struct User {
	id: Option<u64>,
	name: String,
	password: String,
	email: Option<String>,
	admin: bool,
}
#[derive(Clone)]
pub struct UserManager {
	pub pool: MySqlPool,
}
impl UserManager {
	pub async fn get_by_id(&self, id: u64) -> Option<User> {
		sqlx::query_as::<_, User>(DatabaseQuery::GetUserById)
			.bind(id)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	pub async fn get_by_name(&self, name: String) -> Option<User> {
		sqlx::query_as::<_, User>(DatabaseQuery::GetUserByName)
			.bind(name)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	pub async fn create(
		&self,
		name: String,
		password: String,
		email: Option<String>,
	) -> Result<(), sqlx::Error> {
		sqlx::query(DatabaseQuery::CreateUser)
			.bind(name)
			.bind(password)
			.bind(email)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
}
