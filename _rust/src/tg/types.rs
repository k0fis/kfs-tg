#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKind {
    Private { user_id: i64 },
    BasicGroup { group_id: i64 },
    Supergroup { group_id: i64 },
    Channel,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub unread_count: i32,
    pub last_message: Option<String>,
    pub last_read_inbox_message_id: i64,
    pub kind: ChatKind,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Message {
    pub id: i64,
    pub chat_id: i64,
    pub sender_name: String,
    pub text: String,
    pub timestamp: i64,
    pub is_outgoing: bool,
    pub file_id: Option<i32>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub username: Option<String>,
}
