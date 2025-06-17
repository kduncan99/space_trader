use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use rusqlite::{params, Connection};

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

    /// Writes information about this planet to the database.
    /// To be used when the planet is first created.
    pub fn persist(&self, database: &Connection) {
        // TODO resource columns
        let statement = "INSERT INTO planets (planetId, planetName) VALUES (?1, ?2);";
        let params = params![self.planet_id, self.planet_name];
        match database.execute(statement, params) {
            Ok(_) => (),
            Err(err) => {
                println!("Database error: {}", err);
                println!("{}", statement);
                panic!("Shutting down");
            }
        }
    }
}
