use crate::galaxy::{GalaxyId, Galaxy};
use crate::user::{UserId, User};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use rusqlite::{Connection, Result};

pub struct Universe {
    pub users: HashMap<UserId, User>,
    pub galaxies: HashMap<GalaxyId, Galaxy>,
}

pub static UNIVERSE: LazyLock<Mutex<Universe>> = LazyLock::new(|| Mutex::new(Universe::new()));

impl Universe {
    pub fn new() -> Universe {
        Universe {
            users: HashMap::new(),
            galaxies: HashMap::new(),
        }
    }

    pub fn inject_galaxy(&mut self, galaxy: Galaxy) {
        self.galaxies.insert(galaxy.get_galaxy_id(), galaxy);
    }

    /// Initializes a game with one conventional galaxy.
    ///
    /// # Arguments
    /// * `database` open database connection, with the database initialized to contain all the appropriate
    /// empty tables.
    pub fn initialize(&mut self, database: Connection) -> Result<()> {
        let admin = User::new("Admin".to_string(), "admin".to_string(), "Great Overlord".to_string(), None);
        admin.persist(&database)?;
        self.users.insert(admin.user_id, admin);

        let neo = User::new("Neo".to_string(), "anderson".to_string(), "Agent Smith".to_string(), Some(45));
        neo.persist(&database)?;
        self.users.insert(neo.user_id, neo);

        let galaxy_id: GalaxyId = 1;
        let galaxy_name = String::from("Kronos");
        let sector_count = 1000;
        let galaxy = Galaxy::new_conventional_galaxy(galaxy_id, galaxy_name, sector_count);
        //galaxy.dump();//TODO remove
        galaxy.persist(&database)?;
        self.inject_galaxy(galaxy);

        Ok(())
    }

    /// Loads a game with the content of the given database
    ///
    /// # Arguments
    /// * `database` open database connection, containing a valid game database.
    pub fn load(&mut self, database: &Connection) -> Result<()> {
        self.users = User::load_users(database)?;
        self.galaxies = Galaxy::load_galaxies(database)?;
        Ok(())
    }

    pub fn validate_credentials(&self, user_name: String, password: String) -> Option<UserId> {
        for user in self.users.values() {
            if user.user_name.to_ascii_lowercase() == user_name.to_ascii_lowercase() {
                if user.user_password == password {
                    return Some(user.user_id);
                }
                break;
            }
        }
        None
    }
}
