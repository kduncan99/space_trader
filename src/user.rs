use std::collections::{HashMap};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection};

pub type UserId = usize;

pub const ADMIN_USER_ID: UserId = 1;
const ADMIN_USER_NAME: &'static str  = "Admin";
const ADMIN_PASSWORD: &'static str = "admin";
const ADMIN_GAME_NAME: &'static str  = "Cosmic Overlord";
const DEFAULT_REQUESTS_PER_DAY: i32 = 150;

static NEXT_USER_ID: LazyLock<Mutex<UserId>> = LazyLock::new(|| Mutex::new(1));
static USERS: LazyLock<Mutex<HashMap<UserId, User>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub enum ValidationResult {
    Success(UserId),
    NoSuchUser,
    IncorrectPassword,
    UserIsDisabled,
}

pub struct User {
    pub user_id: UserId,
    pub user_name: String,
    pub user_password: String,
    pub game_name: String,
    pub last_login_timestamp: Option<SystemTime>,
    pub is_disabled: bool,
    pub requests_per_day: Option<i32>, // if None, user has no limit
    pub requests_remaining: Option<i32> // None if the above is None
}

pub fn create_admin_user(database: &Connection) -> Result<UserId, String> {
    let mut next_user_id = NEXT_USER_ID.lock().unwrap();
    if *next_user_id >= ADMIN_USER_ID {
        *next_user_id = ADMIN_USER_ID + 1;
    }

    create_user(database,
                ADMIN_USER_ID,
                ADMIN_USER_NAME.to_string(),
                ADMIN_PASSWORD.to_string(),
                ADMIN_GAME_NAME.to_string(),
                None)
}

pub fn create_normal_user(database: &Connection,
                          user_name: String,
                          password: String,
                          game_name: String) -> Result<UserId, String> {
    let mut next_user_id = NEXT_USER_ID.lock().unwrap();
    let user_id = *next_user_id;
    *next_user_id += 1;
    create_user(database, user_id, user_name, password, game_name, Some(DEFAULT_REQUESTS_PER_DAY))
}

fn create_user(database: &Connection,
               user_id: UserId,
               user_name: String,
               user_password: String,
               user_game_name: String,
               requests_per_day: Option<i32>) -> Result<UserId, String> {
    if user_exists(&user_name) {
        return Err("User already exists".to_string());
    }

    let user = User {
        user_id,
        user_name,
        user_password,
        last_login_timestamp: None,
        game_name: user_game_name,
        is_disabled: false,
        requests_per_day,
        requests_remaining: requests_per_day };

    match user.persist(database) {
        Ok(_) => (),
        Err(e) => { return Err(e.to_string()); },
    }

    USERS.lock().unwrap().insert(user_id, user);
    Ok(user_id)
}

pub fn get_user(user_id: UserId) -> Option<User> {
    for user in USERS.lock().unwrap().values() {
        if user.user_id == user_id {
            return Some(user.clone());
        }
    }
    None
}

pub fn load_users(database: &Connection) -> Result<(), String> {
    USERS.lock().unwrap().clear();

    match || -> rusqlite::Result<()> {
        let mut stmt =
            database.prepare("SELECT userId, userName, password, gameName, lastLoginTimeStamp, \
                                    isDisabled, requestsPerDay, requestsRemaining FROM users")?;
        let user_iter = stmt.query_map(params![], |row| {
            Ok(User::new(row.get(0)?, 
                         row.get(1)?,
                         row.get(2)?,
                         row.get(3)?,
                         row.get(4)?,
                         row.get(5)?,
                         row.get(6)?,
                         row.get(7)?))
        })?;

        let mut highest_user_id: UserId = 0;
        for user_result in user_iter {
            let user = user_result?;
            highest_user_id = user.user_id;
            println!("Loaded user {} {}", user.user_id, user.user_name);
            USERS.lock().unwrap().insert(user.user_id, user);
        }

        *NEXT_USER_ID.lock().unwrap() = highest_user_id + 1;
        Ok(())
    }() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Cannot load users:{}", e)),
    }
}

fn user_exists(user_name: &String) -> bool {
    for user in USERS.lock().unwrap().values() {
        if user_name.to_lowercase() == user.user_name.to_lowercase() {
            return true;
        }
    }
    false
}

pub fn validate_credentials(user_name: &String, password: &String) -> ValidationResult {
    for user in USERS.lock().unwrap().values() {
        if user_name.to_lowercase() == user.user_name.to_lowercase() {
            if *password == user.user_password {
                if user.is_disabled {
                    return ValidationResult::UserIsDisabled;
                } else {
                    return ValidationResult::Success(user.user_id);
                }
            } else {
                return ValidationResult::IncorrectPassword;
            }
        }
    }

    ValidationResult::NoSuchUser
}

impl User {
    fn clone(&self) -> User {
        User{user_id: self.user_id,
            user_name: self.user_name.clone(),
            user_password: "".to_string(),
            game_name: self.game_name.clone(),
            last_login_timestamp: self.last_login_timestamp,
            is_disabled: self.is_disabled,
            requests_per_day: self.requests_per_day,
            requests_remaining: self.requests_remaining}
    }

    fn new(user_id: UserId,
           user_name: String,
           password: String,
           game_name: String,
           last_login: Option<u64>,
           is_disabled: bool,
           requests_per_day: Option<i32>,
           requests_remaining: Option<i32>) -> User {
        let time_stamp =
            if last_login.is_none() {
                None
            } else {
                let d = Duration::from_secs(last_login.unwrap());
                Some(UNIX_EPOCH + d)
            };
        User{user_id, user_name, user_password: password, game_name, last_login_timestamp: time_stamp,
            is_disabled, requests_per_day, requests_remaining}
    }

    pub fn persist(&self, database: &Connection) -> Result<(), String> {
        match || -> rusqlite::Result<()> {
            let statement = "INSERT INTO users \
                            (userId, userName, password, gameName, lastLoginTimeStamp, isDisabled, requestsPerDay, requestsRemaining) \
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
            let is_disabled = if self.is_disabled { 1 } else { 0 };
            let opt_time_stamp =
                if self.last_login_timestamp.is_none() {
                    None
                } else {
                    Some(self.last_login_timestamp.unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs())
                };
            let params =
                params![self.user_id, self.user_name, self.user_password, self.game_name, opt_time_stamp,
                    is_disabled, self.requests_per_day, self.requests_remaining];
            database.execute(statement, params)?;
            Ok(())
        }() {
            Ok(_) => Ok(()),
            Err(_) => Err(format!("Failed to persist user {}", self.user_name)),
        }
    }
}