use crate::{
	DatabaseQuery,
	MySqlPool,
};

/// (True name, size, original name)
///
/// Information about uploaded file to insert into the database.
pub type FileSummary = (String, usize, String);

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
	pub async fn create(&self, post_id: &u64, data: FileSummary) -> Result<(), sqlx::Error> {
		let (name, size, original_name) = data;
		sqlx::query(DatabaseQuery::CreateAttachment)
			.bind(post_id)
			.bind(name)
			.bind(size as u64)
			.bind(original_name)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	pub async fn delete_orphans(&self) -> Result<u64, sqlx::Error> {
		todo!();
	}
}