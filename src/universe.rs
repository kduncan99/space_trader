use crate::galaxy::{GalaxyId, Galaxy};
use crate::user;
use crate::user::{UserId, User};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use rusqlite::{Connection, Result};

static USERS: LazyLock<Mutex<HashMap<UserId, User>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static GALAXIES: LazyLock<Mutex<HashMap<GalaxyId, Galaxy>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
pub static DATABASE: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(|| Mutex::new(None));

/// Initializes a game with one conventional galaxy.
///
/// # Arguments
/// * `database` open database connection, with the database initialized to contain all the appropriate
/// empty tables.
pub fn initialize(database: Connection) -> Result<()> {
    let admin = User::new("Admin".to_string(), "admin".to_string(), "Great Overlord".to_string(), None);
    admin.persist(&database)?;
    USERS.lock().unwrap().insert(admin.user_id, admin);

    let neo = User::new("Neo".to_string(), "anderson".to_string(), "Agent Smith".to_string(), Some(45));
    neo.persist(&database)?;
    USERS.lock().unwrap().insert(neo.user_id, neo);

    let galaxy_id: GalaxyId = 1;
    let galaxy_name = String::from("Kronos");
    let sector_count = 1000;
    let galaxy = Galaxy::new_conventional_galaxy(galaxy_id, galaxy_name, sector_count);
    //galaxy.dump();//TODO remove
    galaxy.persist(&database)?;
    GALAXIES.lock().unwrap().insert(galaxy_id, galaxy);
    DATABASE.lock().unwrap().replace(database);

    Ok(())
}

/// Loads a game with the content of the given database
///
/// # Arguments
/// * `database` open database connection, containing a valid game database.
pub fn load(database: Connection) -> Result<()> {
    DATABASE.lock().unwrap().replace(database);
    USERS.lock().unwrap().clear();
    GALAXIES.lock().unwrap().clear();

    let binding = DATABASE.lock().unwrap();
    let database = binding.as_ref().unwrap();
    USERS.lock().unwrap().extend(user::load_users(database)?);
    GALAXIES.lock().unwrap().extend(Galaxy::load_galaxies(database)?);

    Ok(())
}

/// Allows the caller to validate a set of user credentials
///
/// # Arguments
/// * `user_name` case-insensitive user-name to be verified
/// * `password` clear-text (for now) password to be verified
/// 
/// # Returns
/// user-id of the configured user *IF* verification passes
pub fn validate_credentials(user_name: String, password: String) -> Option<UserId> {
    for user in USERS.lock().unwrap().values() {
        if user.user_name.to_ascii_lowercase() == user_name.to_ascii_lowercase() {
            if user.user_password == password {
                return Some(user.user_id);
            }
            break;
        }
    }
    None
}
