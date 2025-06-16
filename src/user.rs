use chrono::{DateTime, Local};

pub struct User {
    pub user_id: i32,
    pub user_name: String,
    pub user_password: String, // encrypted
    pub game_name: String,
    //
    pub minutes_per_day: i32,
    pub minutes_remaining_today: i32,
    pub latest_login: DateTime<Local>,
}
