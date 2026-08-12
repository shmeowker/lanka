use serde_json::Value;
use crate::{
	DatabaseQuery,
	MySqlPool,
};


pub struct Attachment {
	pub name: String,
	pub post: Option<u64>,
	pub size: u64,
}

#[derive(Clone)]
pub struct AttachmentManager {
	pool: MySqlPool,
}

impl AttachmentManager {
	pub fn new(pool: &MySqlPool) -> Self {
		Self {
			pool: pool.clone(),
		}
	}
	pub async fn list_for_post(&self, post_id: u64) -> Vec<Attachment> {
		todo!();
	}
	pub async fn create(&self, post_id: &u64, data: Value) -> Result<(), sqlx::Error> {
		sqlx::query(DatabaseQuery::CreateAttachment)
			.bind(post_id)
			.bind(data["name"].as_str())
			.bind(data["size"].as_u64())
			.bind(data["original_name"].as_str())
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	pub async fn delete_orphans(&self) -> Result<u64, sqlx::Error> {
		todo!();
	}
}