use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, Result};
use crate::port::{Port, PortId};

pub type PlanetId = usize;

lazy_static! {
    static ref NEXT_PLANET_ID: AtomicUsize = AtomicUsize::new(1);
}

pub struct Planet {
    planet_id: PlanetId,
    planet_name: String,
}

impl Planet {
    fn new(planet_name: String) -> Planet {
        let planet_id = NEXT_PLANET_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        for planet_result in mapped_planets {
            let mut planet = planet_result?;
            crate::planet::NEXT_PLANET_ID.store(planet.planet_id, std::sync::atomic::Ordering::SeqCst);
            planet_map.insert(planet.planet_id, planet);
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
