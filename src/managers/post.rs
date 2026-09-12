use crate::{
    AttachmentManager, DatabaseQuery, DateTime, Deserialize, FileSummary, FromRow,
    MySqlPool, Serialize, Utc,
};


#[derive(FromRow, Deserialize, Serialize, Clone, PartialEq)]
pub struct Post {
    pub id: u64,
    pub board: String,
    pub thread: Option<u64>,
    pub reply: Option<u64>,
    pub content: Option<String>,
    pub attachments: Option<String>,
    pub author: Option<String>,
    pub created: DateTime<Utc>,
    pub bumped: DateTime<Utc>,
    pub pinned: Option<bool>,
    pub locked: Option<bool>,
}


#[derive(Clone)]
pub struct PostManager {
    pool: MySqlPool,
    pub attachment: AttachmentManager,
}

impl PostManager {
    pub fn new(pool: &MySqlPool) -> Self {
        Self {
            pool: pool.clone(),
            attachment: AttachmentManager::new(pool),
        }
    }

    #[allow(unused)]
    pub async fn post_exists(&self, post_id: &u64) -> bool {
        todo!()
    }

    #[allow(unused)]
    pub async fn thread_exists(&self, thread_id: &u64) -> bool {
        todo!()
    }

    pub async fn get(&self, post_id: &u64) -> Option<Post> {
        sqlx::query_as::<_, Post>(DatabaseQuery::GetPost)
            .bind(post_id)
            .fetch_one(&self.pool)
            .await
            .ok()
    }

    /// Create a post, returning its ID on success.
    ///
    /// If `thread` is None, the post is considered a thread.
    /// If `author` is None, the post is anonymous.
    pub async fn create(
        &self,
        board: String,
        thread: Option<u64>,
        reply: Option<u64>,
        content: Option<String>,
        mut attachments: Vec<FileSummary>,
        author: Option<String>,
    ) -> Result<u64, sqlx::Error> {
        let post_id: u64 = sqlx::query_scalar(DatabaseQuery::CreatePost)
            .bind(board)
            .bind(thread)
            .bind(reply)
            .bind(content)
            .bind(author)
            .fetch_one(&self.pool)
            .await?;
        for data in attachments.drain(..) {
            let _ = self.attachment.create(&post_id, data).await;
        }
        if thread.is_some() {
            sqlx::query(DatabaseQuery::BumpThread)
                .bind(thread)
                .execute(&self.pool)
                .await?;
        }
        Ok(post_id)
    }

    pub async fn board(&self, board: &String) -> Vec<Post> {
        sqlx::query_as::<_, Post>(DatabaseQuery::ListThreads)
            .bind(board)
            .fetch_all(&self.pool)
            .await
            .unwrap_or(vec![])
    }

    pub async fn thread(&self, thread: &u64) -> Vec<Post> {
        sqlx::query_as::<_, Post>(DatabaseQuery::ListThreadPosts)
            .bind(thread)
            .bind(thread)
            .fetch_all(&self.pool)
            .await
            .unwrap_or(vec![])
    }
    
    pub async fn count_all(&self) -> u64 {
        sqlx::query_scalar(DatabaseQuery::CountExistingPosts)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
    }
    
    pub async fn count_threads(&self) -> u64 {
        sqlx::query_scalar(DatabaseQuery::CountExistingThreads)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as u64
    }
    
    pub async fn count_total(&self) -> u64 {
        sqlx::query_scalar(DatabaseQuery::CountTotalPosts)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
    }
}
