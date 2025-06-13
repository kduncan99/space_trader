use std::collections::HashSet;
use rusqlite::Connection;
use crate::port::PortId;

pub type SectorId = u32;

pub struct Sector {
    pub sector_id: SectorId,
    pub links: HashSet<SectorId>,
    pub port_id: Option<PortId>,
}

impl Sector {
    /// Invoked by Galaxy during creation of a galaxy, and nowhere else
    pub fn new(sector_id: SectorId) -> Sector {
        Sector { sector_id, links: HashSet::new(), port_id: None }
    }

    /// Invoked by Galaxy after full creation of itself, so that we can populate entries in
    /// both the sector and the sector_to_ports tables.
    pub fn persist(&self, database: &Connection) {
        let (statement, result) = {
            if self.port_id.is_some() {
                let statement = "INSERT INTO sectors (sectorId, portId) VALUES (?1, ?2);";
                let params = [self.sector_id, self.port_id.unwrap()];
                (statement, database.execute(statement, params))
            } else {
                let statement = "INSERT INTO sectors (sectorId) VALUES (?1);";
                let params = [self.sector_id];
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
            let params = [self.sector_id, *link];
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
