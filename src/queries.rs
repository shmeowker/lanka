use sqlx::{AssertSqlSafe, SqlSafeStr, SqlStr};

/// Holds all database queries
#[derive(Clone, Copy)]
pub enum DatabaseQuery {
    // AttachmentManager queries
    //ListAttachmentsByName,
    //DeleteAttachmentsByName,
    //DeleteAttachmentById,
    ListAttachmentsForPost,
    CreateAttachment,
    //ListOrphanedAttachments,
    //DeleteOrphanedAttachments,

    // BoardManager queries
    GetBoardByName,
    ListBoards,
    BoardExists,
    ListTopics,
    ListBoardsByTopic,
    //CreateBoard,
    //EditBoard,
    //DeleteBoard,
    //CreateTheme,
    //DeleteTheme,

    // PostManager queries
    PostExists,
    ThreadExists,
    GetPost,
    ListThreads,
    ListThreadPosts,
    CreatePost,
    BumpThread,
    CountExistingPosts,
    CountExistingThreads,
    CountTotalPosts,
    //SetThreadPin,
    //SetThreadLock,
    //DeletePost,
    //DeleteThread,
    //MoveThread,

    // UserManager queries
    GetUserById,
    GetUserByName,
    GetUserByLogin,
    CreateUser,
    //ChangeUserEmail,
    //ChangeUserName,
    //ChangeUserPassword,

    // SessionManager queries
    CreateSession,
    RenewSession,
    GetSessionByToken,
    ListUserSessions,
    DeleteSessionByToken,
    DeleteSessionById,
}

impl DatabaseQuery {
    #[inline]
    const fn into_str(self) -> &'static str {
        // Attention: The queries below are for MariaDB.
        // Not compatible with other databases.
        match self {
            // AttachmentManager queries
            Self::ListAttachmentsForPost => "select * from attachments where post = ?",
            Self::CreateAttachment => {
                "insert into attachments (post, name, size, original_name) values (?, ?, ?, ?)"
            }
            // BoardManager queries
            Self::GetBoardByName => "select * from boards where name = ?",
            Self::ListBoards => "select * from boards",
            Self::BoardExists => "select exists(select 1 from boards where name = ?)",
            Self::ListTopics => "select * from themes",
            Self::ListBoardsByTopic => "select * from boards where theme = ?",

            // PostManager queries
            Self::PostExists => {
                "select exists(select 1 from posts where thread is null and id = ?)"
            }
            Self::ThreadExists => "select exists(select 1 from posts where id = ?)",
            Self::GetPost => {
                "select posts.*, coalesce((select json_arrayagg(json_object('id', a.id, 'name', a.name, 'post', a.post, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id), '[]') attachments from posts where id = ?"
            }
            Self::ListThreads => {
                "select posts.*, coalesce((select json_arrayagg(json_object('id', a.id, 'name', a.name, 'post', a.post, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id), '[]') attachments from posts where board = ? and thread is null order by bumped desc"
            }
            Self::ListThreadPosts => {
                "select posts.*, coalesce((select json_arrayagg(json_object('id', a.id, 'name', a.name, 'post', a.post, 'size', a.size, 'original_name', a.original_name)) from attachments a where a.post = posts.id), '[]') attachments from posts where id = ? or thread = ?;"
            }
            Self::CreatePost => {
                "insert into posts (board, thread, reply, content, author) values (?, ?, ?, ?, ?) returning id"
            }
            Self::BumpThread => "update posts set bumped = current_timestamp() where id = ?",
            Self::CountExistingPosts => {
                "select TABLE_ROWS from INFORMATION_SCHEMA.TABLES where TABLE_SCHEMA = 'lanka' and TABLE_NAME = 'posts'"
            } // Replace 'lanka' to your database name
            Self::CountExistingThreads => "select count(*) from posts where thread is null",
            Self::CountTotalPosts => "select id from posts order by id desc limit 1",

            // UserManager queries
            Self::GetUserById => "select * from users where id = ?",
            Self::GetUserByName => "select * from users where name = ?",
            Self::GetUserByLogin => "select * from users where name = ? or email = ?",
            Self::CreateUser => "insert into users (name, password, email) values (?, ?, ?)",

            // SessionManager queries
            Self::CreateSession => {
                "insert into sessions (user, token_hash) values (?, ?) returning *"
            }
            Self::RenewSession => {
                "update sessions set expires = date_add(current_timestamp() + interval 7 day) where token_hash = ?"
            }
            Self::GetSessionByToken => "select * from sessions where token_hash = ?",
            Self::ListUserSessions => "select * from sessions where user = ?",
            Self::DeleteSessionByToken => "delete from sessions where token_hash = ?",
            Self::DeleteSessionById => "delete from sessions where id = ?",
        }
    }
}

impl SqlSafeStr for DatabaseQuery {
    #[inline]
    fn into_sql_str(self) -> SqlStr {
        AssertSqlSafe(self.into_str()).into_sql_str()
    }
}