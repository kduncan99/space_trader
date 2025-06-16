use crate::galaxy::Galaxy;
use crate::planet::PlanetId;
use crate::port::PortId;

use std::collections::HashSet;
use rusqlite::{params, Connection};

pub type SectorId = usize;

/// Represents a location in space, conceptually contained within a galaxy.
/// Sectors have no names, being identified only by their integer id.
/// A sector may contain a port.
/// A sector may contain a planet.
/// Various space-craft (including ships, missiles, fighters, etc.) may be temporarily located in a sector,
/// but the Sector struct has no knowledge of this.
pub struct Sector {
    pub sector_id: SectorId,
    pub planet_id: Option<PlanetId>,
    pub port_id: Option<PortId>,
    pub links: HashSet<SectorId>,
}

impl Sector {
    pub fn new(sector_id: SectorId) -> Sector {
        Sector{sector_id, planet_id: None, port_id: None, links: HashSet::new()}
    }

    /// Creates a vector of strings to be sent to a user, describing the sector
    ///
    /// # Arguments
    /// * `galaxy` the galaxy which contains this sector - needed in order to look up actual port and planet struct by id.
    pub fn description(&self, galaxy: &Galaxy) -> Vec<String> {
        let mut result : Vec<String> = Vec::new();
        result.push(format!("Sector {}", self.sector_id));
        let string_numbers: Vec<String> = self.links.iter()
            .map(|&n| n.to_string())
            .collect();
        result.push(format!("  Links to: {}", string_numbers.join(" ")));
        if self.planet_id.is_some() {
            result.push(format!("  Planet: {}", self.planet_id.as_ref().unwrap()));
        }
        if self.port_id.is_some() {
            let port = galaxy.ports.get(&self.port_id.unwrap()).unwrap();
            result.push(format!("  Port: {}", port.port_name()));
        }
        result
    }

    /// Writes information about this sector to the database.
    /// To be used when the sector is first created.
    pub fn persist(&self, database: &Connection) {
        let (statement, result) = {
            if self.port_id.is_some() {
                let statement = "INSERT INTO sectors (sectorId, portId) VALUES (?1, ?2);";
                let params = params![self.sector_id, self.port_id.unwrap()];
                (statement, database.execute(statement, params))
            } else {
                let statement = "INSERT INTO sectors (sectorId) VALUES (?1);";
                let params = params![self.sector_id];
                (statement, database.execute(statement, params))
            }
        };

        match result {
            Ok(_) => (),
            Err(err) => {
                println!("Database error: {}", err);
                println!("{}", statement);
                panic!("Shutting down");
            }
        }

        for link in self.links.iter() {
            let statement = "INSERT INTO sector_links (fromSectorId, toSectorId) VALUES (?1, ?2);";
            let params = params![self.sector_id, *link];
            match database.execute(statement, params) {
                Ok(_) => (),
                Err(err) => {
                    println!("Database error: {}", err);
                    println!("{}", statement);
                    panic!("Shutting down");
                }
            }
        }

        if self.planet_id.is_some() {
            let statement = "INSERT INTO sectors_to_planets (sectorId, planetId) VALUES (?1, ?2);";
            let params = params![self.sector_id, self.planet_id.unwrap()];
            match database.execute(statement, params) {
                Ok(_) => (),
                Err(err) => {
                    println!("Database error: {}", err);
                    println!("{}", statement);
                    panic!("Shutting down");
                }
            }
        }
        
        if self.port_id.is_some() {
            let statement = "INSERT INTO sectors_to_ports (sectorId, portId) VALUES (?1, ?2);";
            let params = params![self.sector_id, self.port_id.unwrap()];
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
}
