use crate::galaxy::{GalaxyId, Galaxy};
use crate::sector::{SectorId, Sector};
use crate::planet::{PlanetId, Planet};
use crate::port::{PortId, Port};

use std::collections::HashMap;
use rusqlite::{Connection, Result};

pub struct Universe {
    pub galaxies: HashMap<GalaxyId, Galaxy>,
    pub sectors: HashMap<SectorId, Sector>,
    pub planets: HashMap<PlanetId, Planet>,
    pub ports: HashMap<PortId, Port>,
}

impl Universe {
    pub fn new() -> Universe {
        Universe {
            // users
            // players (some are users, some are autonomous)
            // states (special form of players with varying sizes of spheres of influence)
            galaxies: HashMap::new(),
            sectors: HashMap::new(),
            planets: HashMap::new(),
            ports: HashMap::new(),
            // ships (all are owned by players)
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
    pub fn initialize(&mut self, database: Connection) {
        let galaxy_id: GalaxyId = 1;
        let galaxy_name = String::from("Kronos");
        let sector_count = 1000;
        let galaxy = Galaxy::new_conventional_galaxy(galaxy_id, galaxy_name, sector_count);
        galaxy.persist(&database);
        self.inject_galaxy(galaxy);
    }

    /// Loads a game with the content of the given database
    ///
    /// # Arguments
    /// * `database` open database connection, containing a valid game database.
    pub fn load(&mut self, database: &Connection) {
        fn sub(universe: &mut Universe, database: &Connection) -> Result<()> {
            let mut stmt = database.prepare("SELECT * FROM galaxies;")?;
            let galaxies = stmt.query_map([], |row| {
                Ok(Galaxy::new(row.get(0)?,row.get(1)?))
            })?;

            for galaxy in galaxies {
                let g = galaxy.unwrap();
                g.load(database);
                universe.inject_galaxy(g);
            }
            Ok(())
        }

        match sub(self, database) {
            Ok(_) => (),
            Err(err) => {
                println!("Database error: {}", err);
                panic!("Shutting down");
            }
        }
    }
}
