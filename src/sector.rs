use crate::planet::Planet;
use crate::port::Port;

use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use rusqlite::{params, Connection};

pub type SectorId = usize;

lazy_static! {
    static ref NEXT_SECTOR_ID: AtomicUsize = AtomicUsize::new(1);
}

/// Represents a location in space, conceptually contained within a galaxy.
/// Sectors have no names, being identified only by their integer id.
/// A sector may contain a port.
/// A sector may contain a planet.
/// Various space-craft (including ships, missiles, fighters, etc.) may be temporarily located in a sector,
/// but the Sector struct has no knowledge of this.
pub struct Sector {
    sector_id: SectorId,
    planet: Option<Planet>,
    port: Option<Port>,
    links: HashSet<SectorId>,
}

impl Sector {
    pub fn new() -> Sector {
        let sector_id = NEXT_SECTOR_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Sector{sector_id, planet: None, port: None, links: HashSet::new()}
    }

    pub fn get_link_count(&self) -> usize {
        self.links.len()
    }

    pub fn get_links(&self) -> HashSet<SectorId> {
        self.links.clone()
    }

    pub fn get_sector_id(&self) -> SectorId {
        self.sector_id
    }

    pub fn get_planet(&self) -> &Planet {
        self.planet.as_ref().unwrap()
    }

    pub fn get_port(&self) -> &Port {
        self.port.as_ref().unwrap()
    }

    pub fn has_max_links(&self) -> bool {
        self.links.len() >= 6
    }

    pub fn has_planet(&self) -> bool {
        self.planet.is_some()
    }

    pub fn has_port(&self) -> bool {
        self.port.is_some()
    }

    pub fn insert_link_to(&mut self, sector_id: SectorId) {
        self.links.insert(sector_id);
    }

    pub fn set_port(&mut self, port: Port) {
        self.port = Some(port);
    }

    /// Creates a vector of strings to be sent to a user, describing the sector
    pub fn description(&self) -> Vec<String> {
        let mut result : Vec<String> = Vec::new();
        result.push(format!("Sector {}", self.sector_id));
        let string_numbers: Vec<String> = self.links.iter()
            .map(|&n| n.to_string())
            .collect();
        result.push(format!("  Links to: {}", string_numbers.join(" ")));
        if self.planet.is_some() {
            result.push(format!("  Port: {}", self.get_planet().get_planet_name()));
        }
        if self.port.is_some() {
            let port_name : &String = self.port.as_ref().unwrap().get_port_name();
            result.push(format!("  Port: {}", port_name));
        }
        result
    }

    /// Writes information about this sector to the database.
    /// To be used when the sector is first created.
    pub fn persist(&self, database: &Connection) {
        let (statement, result) = {
            let statement = "INSERT INTO sectors (sectorId) VALUES (?1);";
            let params = params![self.sector_id];
            (statement, database.execute(statement, params))
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

        if self.has_planet() {
            self.get_planet().persist(database);

            let statement = "INSERT INTO sectors_to_planets (sectorId, planetId) VALUES (?1, ?2);";
            let params = params![self.sector_id, &self.get_planet().get_planet_id()];
            match database.execute(statement, params) {
                Ok(_) => (),
                Err(err) => {
                    println!("Database error: {}", err);
                    println!("{}", statement);
                    panic!("Shutting down");
                }
            }
        }
        
        if self.has_port() {
            self.get_port().persist(database);

            let statement = "INSERT INTO sectors_to_ports (sectorId, portId) VALUES (?1, ?2);";
            let params = params![self.sector_id, &self.get_port().get_port_id()];
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
