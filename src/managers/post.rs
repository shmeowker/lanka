use rayon::prelude::*;
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
	fn get_template(&self) -> PostData;
}

impl<T> PostKind for T
where
	T: std::ops::Deref<Target = PostData>,
{
	fn get_template(&self) -> PostData {
		(*self).clone()
	}
}

pub struct Post<P: PostKind> {
	pub data: PostData,
	pub template: P,
}

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
	pub pool: MySqlPool,
	pub attachment: AttachmentManager,
}

impl PostManager {
	pub fn new(pool: &MySqlPool) -> Self {
		Self {
			pool: pool.clone(),
			attachment: AttachmentManager::new(&pool),
		}
	}
	pub async fn get(&self, id: u64) -> Option<PostData> {
		sqlx::query_as::<_, PostData>(DatabaseQuery::GetPost)
			.bind(id)
			.fetch_one(&self.pool)
			.await
			.ok()
	}
	/// Create a post. If `thread` is None, creates a thread.
	/// If `author` is None, post is anonymous.
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
			.into_par_iter()
			.map(|post| Post {
				data: post.clone(),
				template: ThreadTemplate(post),
			})
			.collect()
	}
	pub async fn thread(&self, thread: &u64) -> Vec<Post<PostTemplate>> {
		let data = sqlx::query_as::<_, PostData>(DatabaseQuery::ListThreadPosts)
			.bind(&thread)
			.bind(thread)
			.fetch_all(&self.pool)
			.await
			.unwrap_or(vec![]);
		let mut op_posts: Vec<u64> = vec![];
		if let Some(ref init) = data.get(0) {
			op_posts = data
				.par_iter()
				.filter_map(|post| {
					if post.author.is_some() && post.author == init.author {
						return Some(post.id);
					}
					None
				})
				.collect();
		}
		data
			.into_par_iter()
			.map(|post| {
				let mut op = false;
				let mut reply_op = false;
				if op_posts.contains(&post.id) {
					op = true;
				}
				if let Some(reply) = &post.reply.as_ref() {
					reply_op = op_posts.contains(&reply);
				}
				Post {
					data: post.clone(),
					template: PostTemplate {
						post: post,
						op: op,
						reply_op: reply_op,
					},
				}
			})
			.collect()
	}
}
