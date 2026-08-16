use std::collections::HashSet;
use crate::{
	AttachmentManager,
	DatabaseQuery,
	DateTime,
	Deref,
	Deserialize,
	FileSummary,
	FromRow,
	MySqlPool,
	Template,
	Utc
};


pub trait PostKind {
	fn get_template(&self) -> &PostData;
}

impl<T> PostKind for T
where
	T: std::ops::Deref<Target = PostData>,
{
	fn get_template(&self) -> &PostData {
		self
	}
}

#[derive(Deref)]
pub struct Post<P: PostKind>(P);

#[derive(FromRow, Deserialize, Clone, PartialEq)]
pub struct PostData {
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

#[derive(Template, Deref)]
#[template(path = "post.html")]
pub struct PostTemplate {
	#[deref]
	pub post: PostData,
	pub op: bool,
	pub reply_op: bool,
}

#[derive(Template, Deref)]
#[template(path = "thread.html")]
pub struct ThreadTemplate(PostData);

#[derive(Clone)]
pub struct PostManager {
	pool: MySqlPool,
	pub attachment: AttachmentManager,
}

impl PostManager {
	pub fn new(pool: &MySqlPool) -> Self {
		Self {
			pool: pool.clone(),
			attachment: AttachmentManager::new(&pool),
		}
	}
	pub async fn post_exists(&self, post_id: &u64) -> bool {
		todo!()
	}
	pub async fn thread_exists(&self, thread_id: &u64) -> bool {
		todo!()
	}
	pub async fn get(&self, post_id: &u64) -> Option<PostData> {
		sqlx::query_as::<_, PostData>(DatabaseQuery::GetPost)
			.bind(post_id)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	/// Create a post.
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
	) -> Result<(), sqlx::Error> {
		let post_id: u64 = sqlx::query_scalar(DatabaseQuery::CreatePost)
			.bind(board)
			.bind(&thread)
			.bind(reply)
			.bind(content)
			.bind(author)
			.fetch_one(&self.pool)
			.await?;
		for data in attachments.drain(..) {
			self.attachment.create(&post_id, data).await;
		}
		if thread.is_some() {
			sqlx::query(DatabaseQuery::BumpThread)
				.bind(thread)
				.execute(&self.pool)
				.await?;
		}
		Ok(())
	}
	pub async fn board(&self, board: &String) -> Vec<Post<ThreadTemplate>> {
		let data = sqlx::query_as::<_, PostData>(DatabaseQuery::ListThreads)
			.bind(board)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![]);
		data
			.into_iter()
			.map(|post| Post(ThreadTemplate(post)))
			.collect()
	}
	pub async fn thread(&self, thread: &u64) -> Vec<Post<PostTemplate>> {
    let data = sqlx::query_as::<_, PostData>(DatabaseQuery::ListThreadPosts)
      .bind(thread)
			.bind(thread)
			.fetch_all(&self.pool)
			.await
			.unwrap_or_default();

    let mut op_posts = HashSet::new();

    if let Some(init) = data.first() {
      op_posts.extend(
        data.iter()
          .filter(|post| post.author.is_some() && post.author == init.author)
          .map(|post| post.id),
      );
    }

    data.into_iter()
			.map(|post| {
				let op = op_posts.contains(&post.id);
				let reply_op = post
					.reply
					.as_ref()
					.is_some_and(|reply| op_posts.contains(reply));

				Post (
					PostTemplate { 
						post: post, 
						op: op, 
						reply_op: reply_op },
				)
			})
			.collect()
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
