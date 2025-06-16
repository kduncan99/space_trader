use rusqlite::{params, Connection};

pub type PlanetId = usize;

pub struct Planet {
    pub planet_id: PlanetId,
    pub planet_name: String,
}

impl Planet {
    fn new<'a>(
        planet_id: PlanetId,
        planet_name: String,
    ) -> Planet {
        Planet{planet_id, planet_name}
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
