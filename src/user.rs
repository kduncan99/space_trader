use std::cmp::max;
use std::collections::{HashMap};
use std::sync::Mutex;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, Result};

pub type UserId = usize;

const ADMIN_USER_ID : UserId = 1;

lazy_static! {
    static ref NEXT_USER_ID: Mutex<UserId> = Mutex::new(1);
}

pub struct User {
    pub user_id: UserId,
    pub user_name: String,
    pub user_password: String,
    pub game_name: String,
    //
    pub is_disabled: bool,
    pub requests_per_day: Option<i32>, // if None, user has no limit
    pub requests_remaining: Option<i32> // None if the above is None
}

pub fn load_users(database: &Connection) -> Result<HashMap<UserId, User>> {
    let mut stmt =
        database.prepare("SELECT userId, userName, password, gameName, isDisabled, requestsPerDay, requestsRemaining FROM users")?;
    let mapped_users = stmt.query_map(params![], |row| {
        Ok(User{user_id: row.get(0)?,
            user_name: row.get(1)?,
            user_password: row.get(2)?,
            game_name: row.get(3)?,
            is_disabled: row.get(4)?,
            requests_per_day: row.get(5)?,
            requests_remaining: row.get(6)?})
    })?;

    let mut next_user_id = NEXT_USER_ID.lock().unwrap();
    let mut user_map = HashMap::new();
    for user_result in mapped_users {
        let user = user_result?;
        *next_user_id = max(*next_user_id, user.user_id + 1);
        println!("Loaded user {} {}", user.user_id, user.user_name);
        user_map.insert(user.user_id, user);
    }

    Ok(user_map)
}

impl User {
    pub fn new(user_name: String, user_password: String, game_name: String, requests_per_day: Option<i32>) -> User {
        let mut next_user_id = NEXT_USER_ID.lock().unwrap();
        let user_id = *next_user_id;
        *next_user_id += 1;
        User { user_id, user_name, user_password, game_name, is_disabled: false,
            requests_per_day: requests_per_day, requests_remaining: requests_per_day }
    }

    pub fn persist(&self, database: &Connection) -> Result<()> {
        let statement = "INSERT INTO users \
                            (userId, userName, password, gameName, isDisabled, requestsPerDay, requestsRemaining) \
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        let i_disabled = if self.is_disabled { 1 } else { 0 };
        let params =
            params![self.user_id, self.user_name, self.user_password, self.game_name,
                i_disabled, self.requests_per_day, self.requests_remaining];
        database.execute(statement, params)?;
        Ok(())
    }
}