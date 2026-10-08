use crate::{
    FromRow,
    Serialize,
    Deserialize,
	DatabaseQuery,
	MySqlPool,
    Post,
};
use sqlx::AssertSqlSafe;

/// (Stored name, size, original name)
///
/// Information about uploaded file to insert into the database.
pub type FileSummary = (String, usize, String);

#[derive(FromRow, Serialize, Deserialize, Clone, PartialEq)]
pub struct Attachment {
    pub id: u64,
	pub name: Box<str>,
	pub post: Option<u64>,
	pub size: u64,
    pub original_name: Option<Box<str>>,
}

impl Attachment {
    fn extension(&self) -> Option<&str> {
        self.name.rfind('.').map(|i| &self.name[i+1..])
    }

    pub fn is_video(&self) -> bool {
        if let Some(ext) = self.extension() {
            return matches!(ext, "mp4" | "avi" | "flv" | "mov" | "ogg" | "ogv" | "webm");
        }
        false
    }

    pub fn is_audio(&self) -> bool {
        if let Some(ext) = self.extension() {
            return matches!(ext, "mp3" | "wav" | "aac" | "m4a" | "oga" | "ogg" | "flac" | "opus");
        }
        false
    }

    pub fn is_image(&self) -> bool {
        if let Some(ext) = self.extension() {
            return matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "svg");
        }
        false
    }
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

    pub async fn list_assoc(&self, posts: &[Post]) -> Vec<Attachment> {
        let placeholders = std::iter::repeat_n("?", posts.len())
            .collect::<Box<[&str]>>()
            .join(", ");

        let query = DatabaseQuery::ListAttachmentsForPosts
            .into_str()
            .replace("?", &placeholders);

        let mut query = sqlx::query_as::<_, Attachment>(AssertSqlSafe(query));

        for post in posts.iter() {
            query = query.bind(post.id);
        }

        query
            .fetch_all(&self.pool)
            .await
            .unwrap_or(vec![])
    }

    #[allow(unused)]
	pub async fn list_for_post(&self, post_id: u64) -> Vec<Attachment> {
		todo!();
	}

    #[allow(unused)]
    pub async fn delete_for_post(&self, post_id: u64, name: String) -> Result<(), sqlx::Error> {
        todo!();
    }
    
    #[allow(unused)]
    pub async fn delete(&self, name: String) -> Result<(), sqlx::Error> {
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