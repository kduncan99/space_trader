use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use rusqlite::{Connection, Result};
use crate::user::UserId;

pub type MessageId = u64;

static MESSAGES: LazyLock<Mutex<HashMap<MessageId, Message>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct Message {
    pub message_id: MessageId,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub date_time: Instant,
    pub message: String,
}

impl Message {
    pub fn persist(database: &Connection) -> Result<()> {
        Ok(())
    }
}