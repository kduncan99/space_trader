use chrono::{DateTime, Local};

pub struct Message {
    pub from_user_id: i32,
    pub to_user_id: i32,
    pub message: String,
    pub date_time: DateTime<Local>,
}
