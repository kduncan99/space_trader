use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use rusqlite::{params, Connection};

pub type PlanetId = usize;

static NEXT_PLANET_ID: LazyLock<Mutex<PlanetId>> = LazyLock::new(|| Mutex::new(1));
static PLANETS: LazyLock<Mutex<HashMap<PlanetId, Planet>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct Planet {
    pub planet_id: PlanetId,
    pub planet_name: String,
}

// Creates a planet and persists it to the database
fn create_planet(database: &Connection, planet_name: String) -> Result<PlanetId, String> {
    let mut next_planet_id = NEXT_PLANET_ID.lock().unwrap();
    let planet_id = *next_planet_id;
    *next_planet_id += 1;

    let planet = Planet { planet_id, planet_name };
    match planet.persist(database) {
        Ok(_) => (),
        Err(e) => { return Err(e.to_string()); },
    }

    PLANETS.lock().unwrap().insert(planet_id, planet);
    Ok(planet_id)
}

// Gets a clone of a planet - only for looking at information, not for changing it
pub fn get_planet(planet_id: PlanetId) -> Option<Planet> {
    for planet in PLANETS.lock().unwrap().values() {
        if planet.planet_id == planet_id {
            return Some(planet.clone());
        }
    }
    None
}

/// Creates a planet map describing all the ports in the universe - Used when a game starts up.
pub fn load_planets(database: &Connection) -> Result<(), String> {
    PLANETS.lock().unwrap().clear();

    match || -> rusqlite::Result<()> {
        let mut stmt = database.prepare("SELECT planetId, planetName FROM planets ORDER BY planetId")?;
        let planet_iter = stmt.query_map([], |row| {
            Ok(Planet { planet_id: row.get(0)?, planet_name: row.get(1)? })
        })?;

        let mut highest_planet_id: PlanetId = 0;
        for planet_result in planet_iter {
            let planet = planet_result?;
            highest_planet_id = planet.planet_id;
            println!("Loaded planet: {}", planet.planet_name);
            PLANETS.lock().unwrap().insert(planet.planet_id, planet);
        }

        *NEXT_PLANET_ID.lock().unwrap() = highest_planet_id + 1;
        Ok(())
    }() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Cannot load planets:{}", e)),
    }
}

impl Planet {
    pub fn clone(&self) -> Planet {
        Planet {planet_id: self.planet_id, planet_name: self.planet_name.clone()}
    }

    /// Writes information about this planet to the database.
    /// To be used when the planet is first created.
    pub fn persist(&self, database: &Connection) -> Result<(), String> {
        match || -> rusqlite::Result<()> {
            // TODO resource columns
            let statement = "INSERT INTO planets (planetId, planetName) VALUES (?1, ?2);";
            let params = params![self.planet_id, self.planet_name];
            database.execute(statement, params)?;
            Ok(())
        }() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Cannot persist planet:{}", e)),
        }
    }
}
