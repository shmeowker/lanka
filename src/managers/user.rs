use crate::{
	DatabaseQuery,
	Deserialize,
	FromRow,
	MySqlPool
};


#[derive(FromRow, Deserialize, Clone, Debug)]
pub struct User {
	pub id: u64,
	pub name: String,
	password: String,
	pub email: Option<String>,
	pub admin: bool,
}

impl User {
	pub fn match_password(&self, password: &String) -> bool {
		let password_hash = blake3::hash(password.as_bytes()).to_string();
		self.password == password_hash
	}
}

#[derive(Clone)]
pub struct UserManager {
	pub pool: MySqlPool,
}

impl UserManager {
	pub fn new(pool: &MySqlPool) -> Self {
		Self { pool: pool.clone() }
	}
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
	pub async fn get_by_login(&self, login: &String) -> Option<User> {
		sqlx::query_as::<_, User>(DatabaseQuery::GetUserByLogin)
			.bind(&login)
			.bind(login)
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
