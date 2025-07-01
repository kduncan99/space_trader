use std::cmp::max;
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

pub fn get_planet(planet_id: PlanetId) -> Option<Planet> {
    let lock = PLANETS.lock().unwrap();
    let planet = lock.get(&planet_id);
    if planet.is_none() {
        None
    } else {
        Some(planet.unwrap().clone())
    }
}

/// Creates a planet map describing all the ports in the universe - Used when a game starts up.
pub fn load_planets(database: &Connection) -> rusqlite::Result<()> {
    PLANETS.lock().unwrap().clear();

    let mut stmt = database.prepare("SELECT planetId, planetName FROM planets")?;
    let mapped_planets = stmt.query_map([], |row| {
        Ok(Planet { planet_id: row.get(0)?, planet_name: row.get(1)? })
    })?;

    let mut next_planet_id = NEXT_PLANET_ID.lock().unwrap();
    for planet_result in mapped_planets {
        let planet = planet_result?;
        *next_planet_id = max(*next_planet_id, planet.planet_id + 1);
        PLANETS.lock().unwrap().insert(planet.planet_id + 1, planet);
    }

    Ok(())
}

fn create_planet(planet_name: String) -> PlanetId {
    let mut next_planet_id = NEXT_PLANET_ID.lock().unwrap();
    let planet_id = *next_planet_id;
    *next_planet_id += 1;
    PLANETS.lock().unwrap().insert(planet_id, Planet { planet_id, planet_name });
    planet_id
}

impl Planet {
    pub fn clone(&self) -> Planet {
        Planet {planet_id: self.planet_id, planet_name: self.planet_name.clone()}
    }

    /// Writes information about this planet to the database.
    /// To be used when the planet is first created.
    pub fn persist(&self, database: &Connection) -> rusqlite::Result<()> {
        // TODO resource columns
        let statement = "INSERT INTO planets (planetId, planetName) VALUES (?1, ?2);";
        let params = params![self.planet_id, self.planet_name];
        database.execute(statement, params)?;
        Ok(())
    }
}
