use std::cmp::max;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, Result};

pub type PlanetId = usize;

lazy_static! {
    static ref NEXT_PLANET_ID: Mutex<PlanetId> = Mutex::new(1);
}

pub struct Planet {
    planet_id: PlanetId,
    planet_name: String,
}

impl Planet {
    fn new(planet_name: String) -> Planet {
        let mut next_planet_id = crate::planet::NEXT_PLANET_ID.lock().unwrap();
        let planet_id = *next_planet_id;
        *next_planet_id += 1;
        Planet{planet_id, planet_name}
    }

    pub fn get_planet_id(&self) -> PlanetId {
        self.planet_id
    }    

    pub fn get_planet_name(&self) -> &String {
        &self.planet_name
    }

    /// Creates a planet map describing all the ports in the universe - Used when a game starts up.
    pub fn load_planets(database: &Connection) -> rusqlite::Result<HashMap<PlanetId, Planet>> {
        let mut stmt = database.prepare("SELECT planetId, planetName FROM planets")?;
        let mapped_planets = stmt.query_map([], |row| {
            Ok(Planet{planet_id: row.get(0)?, planet_name: row.get(1)?})
        })?;

        let mut planet_map : HashMap<PlanetId, Planet> = HashMap::new();
        let mut next_planet_id = NEXT_PLANET_ID.lock().unwrap();
        for planet_result in mapped_planets {
            let planet = planet_result?;
            *next_planet_id = max(*next_planet_id, planet.planet_id + 1);
            planet_map.insert(planet.planet_id + 1, planet);
        }

        Ok(planet_map)
    }

    /// Writes information about this planet to the database.
    /// To be used when the planet is first created.
    pub fn persist(&self, database: &Connection) -> Result<()> {
        // TODO resource columns
        let statement = "INSERT INTO planets (planetId, planetName) VALUES (?1, ?2);";
        let params = params![self.planet_id, self.planet_name];
        database.execute(statement, params)?;
        Ok(())
    }
}
