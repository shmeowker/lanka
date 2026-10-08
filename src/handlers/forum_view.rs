use crate::{
    Board, CurrentUser, Deref, HtmlTemplate, IntoResponse, LState, Rejection, Path, Post,
    StatusCode, TITLE, Template, User,
};

#[derive(Template)]
#[template(path = "forum.html")]
struct ForumTemplate {
    boards: Vec<Board>,
    board: Board,
    thread: Option<u64>,
    posts: Box<str>,
    user: Option<User>,
}

impl ForumTemplate {
    async fn new(
        state: LState,
        board: Board,
        thread: Option<u64>,
        posts: Vec<Post>,
        user: Option<User>,
    ) -> Self {
        let boards = state.board.list().await;
        let rendered_posts = match thread {
            Some(_) => Self::render_thread(posts),
            None => Self::render_board(posts),
        };

        Self {
            boards: boards,
            board: board,
            thread: thread,
            posts: rendered_posts,
            user: user,
        }
    }

    fn render_board(posts: Vec<Post>) -> Box<str> {
        let mut buffer = String::with_capacity(4096);
        for post in posts {
            let _ = ThreadTemplate { post }.render_into(&mut buffer);
        }

        buffer.into_boxed_str()
    }

    fn render_thread(posts: Vec<Post>) -> Box<str> {
        let mut buffer = String::with_capacity(4096);
        let mut op_posts = std::collections::HashSet::new();
        let mut op: Option<Box<str>> = None;
        
        if let Some(root) = posts.first() {
            op = root.author.clone();
            op_posts.insert(root.id);
        }

        for post in posts {
            let is_op = post.author.as_ref().is_some_and(|name| {
                op.as_ref().is_some_and(|op| *name == *op)
            });
            if is_op {
                op_posts.insert(post.id);
            }
            let reply_op = post
                .reply
                .as_ref()
                .is_some_and(|reply| op_posts.contains(reply));

            let _ = PostTemplate {
                    post: post,
                    op: is_op,
                    reply_op: reply_op,
                }
                .render_into(&mut buffer);
        }

        buffer.into_boxed_str()
    }
}

#[derive(Template, Deref)]
#[template(path = "post.html")]
pub struct PostTemplate {
    #[deref]
    pub post: Post,
    pub op: bool,
    pub reply_op: bool,
}

#[derive(Template, Deref)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub post: Post,
}

pub async fn render_board(
    Path(board): Path<String>,
    state: LState,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, Rejection> {
    let posts = state.post.board(&board).await;
    let Some(board) = state.board.get(&board).await else {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Board '{board}' does not exists."),
        ).into());
    };

    let template = ForumTemplate::new(state, board, None, posts, user).await;

    Ok(HtmlTemplate(template))
}

pub async fn render_thread(
    Path((board, thread)): Path<(String, u64)>,
    state: LState,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, Rejection> {
    let posts = state.post.thread(&thread).await;
    let Some(board) = state.board.get(&board).await else {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Board '{board}' does not exists."),
        ).into());
    };

    let template = ForumTemplate::new(state, board, Some(thread), posts, user).await;

    Ok(HtmlTemplate(template))
}
