use crate::{
    Attachment, AttachmentManager, DatabaseQuery, DateTime, Deserialize, FileSummary, FromRow,
    MySqlPool, Serialize, Utc,
};
use ahash::{HashMap, HashMapExt};


#[derive(FromRow, Deserialize, Serialize, Clone, PartialEq)]
pub struct Post {
    pub id: u64,
    pub board: Box<str>,
    pub thread: Option<u64>,
    pub reply: Option<u64>,
    pub content: Option<Box<str>>,
    #[sqlx(skip)]
    pub attachments: Box<[Attachment]>,
    pub author: Option<Box<str>>,
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

    async fn load_attachments(&self, posts: &mut [Post]) {
        if posts.is_empty() {
            return;
        }

        let attachments = self.attachment.list_assoc(&posts).await;

        let mut attachments_by_post: HashMap<u64, Vec<Attachment>> =
            HashMap::with_capacity(posts.len());

        for attachment in attachments {
            if let Some(post_id) = attachment.post {
                attachments_by_post
                    .entry(post_id)
                    .or_default()
                    .push(attachment);
            }
        }

        for post in posts {
            post.attachments = attachments_by_post
                .remove(&post.id)
                .unwrap_or_default()
                .into_boxed_slice();
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
        let post = sqlx::query_as::<_, Post>(DatabaseQuery::GetPost)
            .bind(post_id)
            .fetch_one(&self.pool)
            .await
            .ok()?;

        let mut posts = vec![post];
        self.load_attachments(&mut posts).await;

        posts.pop()
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
        let mut posts = sqlx::query_as::<_, Post>(DatabaseQuery::ListThreads)
            .bind(board)
            .fetch_all(&self.pool)
            .await
            .unwrap_or(vec![]);
        
        self.load_attachments(&mut posts).await;

        posts
    }

    pub async fn thread(&self, thread: &u64) -> Vec<Post> {
        let mut posts = sqlx::query_as::<_, Post>(DatabaseQuery::ListThreadPosts)
            .bind(thread)
            .bind(thread)
            .fetch_all(&self.pool)
            .await
            .unwrap_or(vec![]);

        self.load_attachments(&mut posts).await;

        posts
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
