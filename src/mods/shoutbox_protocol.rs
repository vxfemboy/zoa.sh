use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShoutboxMessage {
    NewMessage { id: String, username: String, content: String, timestamp: String },
    MessageList(Vec<ChatMessage>),
    UserCount(usize),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub username: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShoutboxCommand {
    GetMessages,
    SendMessage { username: String, content: String },
}


