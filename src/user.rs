use std::collections::{HashMap};
use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, Result};

pub type UserId = usize;

lazy_static! {
    static ref NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
}

pub struct User {
    pub user_id: UserId,
    pub user_name: String,
    pub user_password: Option<String>, // encrypted - if None, user must update on login
    pub game_name: Option<String>, // if None, user must update on login
    //
    pub is_disabled: bool,
    pub minutes_per_day: Option<i32>, // if None, user has no time limit
    pub minutes_remaining_today: Option<i32> // None if the above is None
}

impl User {
    pub fn new(user_name: String, game_name: Option<String>, minutes_per_day: Option<i32>) -> User {
        let user_id = NEXT_USER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        User { user_id, user_name, user_password: None, game_name: game_name,
            is_disabled: false, minutes_per_day, minutes_remaining_today: minutes_per_day }
    }

    pub fn load_users(database: &Connection) -> Result<HashMap<UserId, User>> {
        let mut stmt =
            database.prepare("SELECT userId, userName, password, gameName, isDisabled, minutesPerDay, minutesRemaining FROM users")?;
        let mapped_users = stmt.query_map(params![], |row| {
            Ok(User{user_id: row.get(0)?,
                user_name: row.get(1)?,
                user_password: row.get(2)?,
                game_name: row.get(3)?,
                is_disabled: row.get(4)?,
                minutes_per_day: row.get(5)?,
                minutes_remaining_today: row.get(6)?})
        })?;

        let mut user_map = HashMap::new();
        for user_result in mapped_users {
            let user = user_result?;
            NEXT_USER_ID.store(user.user_id + 1, std::sync::atomic::Ordering::SeqCst);
            println!("Loaded user {} {}", user.user_id, user.user_name);
            user_map.insert(user.user_id, user);
        }

        Ok(user_map)
    }

    pub fn persist(&self, database: &Connection) -> Result<()> {
        let statement = "INSERT INTO users \
                            (userId, userName, password, gameName, isDisabled, minutesPerDay, minutesRemaining) \
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        let i_disabled = if self.is_disabled { 1 } else { 0 };
        let params =
            params![self.user_id, self.user_name, self.user_password, self.game_name,
                i_disabled, self.minutes_per_day, self.minutes_remaining_today];
        database.execute(statement, params)?;
        Ok(())
    }
}