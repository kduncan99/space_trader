use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection};
use crate::user::UserId;

pub type MessageId = u64;

static NEXT_MESSAGE_ID: LazyLock<Mutex<MessageId>> = LazyLock::new(|| Mutex::new(1));
static MESSAGES: LazyLock<Mutex<HashMap<MessageId, Message>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct Message {
    pub message_id: MessageId,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub date_time: SystemTime,
    pub message: String,
}

pub fn create_message(database: &Connection, from_user_id: UserId, to_user_id: UserId, message: &String) -> Result<MessageId, String> {
    let mut next_message_id = NEXT_MESSAGE_ID.lock().unwrap();
    let message_id = *next_message_id;
    *next_message_id += 1;

    let msg = Message{message_id, from_user_id, to_user_id, date_time: SystemTime::now(), message: message.clone()};
    match msg.persist(database) {
        Ok(_) => (),
        Err(e) => { return Err(e.to_string()); },
    }

    MESSAGES.lock().unwrap().insert(message_id, msg);
    Ok(message_id)
}

pub fn load_messages(database: &Connection) -> Result<(), String> {
    MESSAGES.lock().unwrap().clear();
    
    match || -> rusqlite::Result<()> {
        let mut stmt = database.prepare("SELECT messageId, fromUserId, toUserId, timeStamp, text \
                                                    FROM messages ORDER BY messageId")?;
        let message_iter = stmt.query_map([], |row| {
            Ok(Message::new(row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?))
        })?;

        let mut highest_message_id: MessageId = 0;
        for message_result in message_iter {
            let msg = message_result?;
            highest_message_id = msg.message_id;
            MESSAGES.lock().unwrap().insert(msg.message_id, msg);
        }

        *NEXT_MESSAGE_ID.lock().unwrap() = highest_message_id + 1;
        println!("Loaded {} messages", MESSAGES.lock().unwrap().len());
        Ok(())
    }() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Cannot load messages:{}", e)),
    }
}

impl Message {
    fn new(message_id: MessageId, from_user_id: UserId, to_user_id: UserId, seconds: u64, message: String) -> Message {
        let d = Duration::from_secs(seconds);
        let st = UNIX_EPOCH + d;
        Message{message_id, from_user_id, to_user_id, date_time: st, message: message.clone()}
    }

    pub fn persist(&self, database: &Connection) -> Result<(), String> {
        match || -> rusqlite::Result<()> {
            let statement = "INSERT INTO messages (messageId, fromUserId, toUserId, timeStamp, text) VALUES (?1, ?2, ?3, ?4, ?5);";
            let unix_time = self.date_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
            let params =
                params![self.message_id, self.from_user_id, self.to_user_id, unix_time, self.message];
            database.execute(statement, params)?;
            Ok(())
        }() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Cannot persist message:{}", e)),
        }
    }
}
