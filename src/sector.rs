use crate::planet::PlanetId;
use crate::port::PortId;

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use rusqlite::{params, Connection};
use crate::{planet, port};

pub type SectorId = usize;

static NEXT_SECTOR_ID: LazyLock<Mutex<SectorId>> = LazyLock::new(|| Mutex::new(1));
static SECTORS: LazyLock<Mutex<HashMap<SectorId, Sector>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
    pub sector_links: HashSet<SectorId>,
}

pub fn create_sector(database: &Connection) -> Result<SectorId, String> {
    let mut next_sector_id = NEXT_SECTOR_ID.lock().unwrap();
    let sector_id = *next_sector_id;
    *next_sector_id += 1;

    let sector = Sector { sector_id, planet_id: None, port_id: None, sector_links: HashSet::new() };
    match sector.persist(database) {
        Ok(_) => (),
        Err(e) => { return Err(e.to_string()); },
    }

    SECTORS.lock().unwrap().insert(sector_id, sector);
    Ok(sector_id)
}

pub fn get_sector(sector_id: SectorId) -> Option<Sector> {
    let lock = SECTORS.lock().unwrap();
    let sector = lock.get(&sector_id);
    if sector.is_none() {
        None
    } else {
        Some(sector.unwrap().clone())
    }
}

pub fn link_sectors(sector_id1: SectorId, sector_id2: SectorId) {
    println!("{}<->{}", sector_id1, sector_id2);//TODO remove
    if sector_id1 == sector_id2 {
        panic!("Attempt to link sector {} to itself", sector_id1);
    }

    if !SECTORS.lock().unwrap().contains_key(&sector_id1) {
        panic!("Attempt to link unknown sector {}", sector_id1);
    }

    if !SECTORS.lock().unwrap().contains_key(&sector_id2) {
        panic!("Attempt to link unknown sector {}", sector_id2);
    }

    let mut lock = SECTORS.lock().unwrap();
    {
        let sector1 = lock.get(&sector_id1).unwrap();
        if sector1.has_max_links() {
            panic!("Attempt to link sector {} which has max links", sector_id1);
        }

        let sector2 = lock.get(&sector_id2).unwrap();
        if sector2.has_max_links() {
            panic!("Attempt to link sector {} which has max links", sector_id2);
        }
    }

    lock.get_mut(&sector_id1).unwrap().insert_link_to(sector_id2);
    lock.get_mut(&sector_id2).unwrap().insert_link_to(sector_id1);
}

// for use by Galaxy::load_galaxies
// Creates a map of all sectors in the universe - Used at game startup.
pub fn load_sectors(database: &Connection) -> Result<(), String> {
    // initial load of all sectors
    SECTORS.lock().unwrap().clear();

    match || -> rusqlite::Result<()> {
        // build sectors
        let mut stmt = database.prepare("SELECT sectorId FROM sectors ORDER BY sectorId")?;
        let sector_iter = stmt.query_map([], |row| {
            Ok(Sector { sector_id: row.get(0)?, sector_links: HashSet::new(), port_id: None, planet_id: None })
        })?;

        // Iterate over the sectors to load links, planets, and ports
        let mut highest_sector_id = 0;
        for result_sector in sector_iter {
            let mut sector = result_sector?;
            highest_sector_id = sector.sector_id;

            // links to other sectors
            let mut stmt = database.prepare("SELECT toSectorId FROM sector_links WHERE fromSectorId = :sectorId")?;
            let link_iter = stmt.query_map(&[(":sectorId", &sector.sector_id)], |row| {
                Ok(row.get(0)?)
            })?;

            for link in link_iter {
                sector.sector_links.insert(link?);
            }

            // link to planet (if any - there should be one at most)
            let mut stmt = database.prepare("SELECT planetId FROM sectors_to_planets WHERE sectorId = :sectorId")?;
            let planet_id_iter = stmt.query_map(&[(":sectorId", &sector.sector_id)], |row| {
                Ok(row.get(0)?)
            })?;

            for planet_id in planet_id_iter {
                sector.planet_id.replace(planet_id?);
            }

            // link to port (if any - there should be one at most)
            let mut stmt = database.prepare("SELECT portId FROM sectors_to_ports WHERE sectorId = :sectorId")?;
            let port_id_iter = stmt.query_map(&[(":sectorId", &sector.sector_id)], |row| {
                Ok(row.get(0)?)
            })?;

            for port_id in port_id_iter {
                sector.port_id.replace(port_id?);
            }

            SECTORS.lock().unwrap().insert(sector.sector_id, sector);
        }

        *NEXT_SECTOR_ID.lock().unwrap() = highest_sector_id;
        println!("Loaded {} sectors", SECTORS.lock().unwrap().len());
        Ok(())
    }() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Cannot load sectors:{}", e)),
    }
}

// Stores a PlanetId in this sector. Used only during loading, so we don't need to update the database
pub fn set_planet_id(sector_id: SectorId, planet_id: PlanetId) {
    SECTORS.lock().unwrap().get_mut(&sector_id).unwrap().planet_id.replace(planet_id);
}

// Stores a PortId in this sector. Used only during loading, so we don't need to update the database
pub fn set_sector_port_id(sector_id: SectorId, port_id: PortId) {
    SECTORS.lock().unwrap().get_mut(&sector_id).unwrap().port_id.replace(port_id);
}

impl Sector {
    pub fn clone(&self) -> Sector {
        Sector{ sector_id: self.sector_id,
                planet_id: self.planet_id,
                port_id: self.port_id,
                sector_links: self.sector_links.clone() }
    }

    /// Creates a vector of strings to be sent to a user, describing the sector
    pub fn get_description(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        result.push(format!("Sector {}", self.sector_id));
        let string_numbers: Vec<String> = self.sector_links.iter()
            .map(|&n| n.to_string())
            .collect();
        result.push(format!("  Links to: {}", string_numbers.join(" ")));
        if self.planet_id.is_some() {
            let planet_name = planet::get_planet(self.planet_id.unwrap()).unwrap().planet_name;
            result.push(format!("  Planet: {}", planet_name));
        }
        if self.port_id.is_some() {
            let port_name = port::get_port(self.port_id.unwrap()).unwrap().port_name;
            result.push(format!("  Port: {}", port_name));
        }
        result
    }

    pub fn get_link_count(&self) -> usize {
        SECTORS.lock().unwrap().len()
    }

    pub fn has_max_links(&self) -> bool {
        self.sector_links.len() >= 6
    }

    pub fn has_planet(&self) -> bool {
        self.planet_id.is_some()
    }

    pub fn has_port(&self) -> bool {
        self.port_id.is_some()
    }

    pub(crate) fn insert_link_to(&mut self, sector_id: SectorId) {
        self.sector_links.insert(sector_id);
    }

    /// Writes information about this sector to the database.
    /// To be used when the sector is first created, and only after ports and planets are all persisted.
    pub fn persist(&self, database: &Connection) -> rusqlite::Result<(), String> {
        match || -> rusqlite::Result<()> {
            let statement = "INSERT INTO sectors (sectorId) VALUES (?1);";
            let params = params![self.sector_id];
            database.execute(statement, params)?;

            for link in self.sector_links.iter() {
                let statement = "INSERT INTO sector_links (fromSectorId, toSectorId) VALUES (?1, ?2);";
                let params = params![self.sector_id, *link];
                database.execute(statement, params)?;
            }

            if self.has_planet() {
                let statement = "INSERT INTO sectors_to_planets (sectorId, planetId) VALUES (?1, ?2);";
                let params = params![self.sector_id, &self.planet_id.unwrap()];
                database.execute(statement, params)?;
            }

            if self.has_port() {
                let statement = "INSERT INTO sectors_to_ports (sectorId, portId) VALUES (?1, ?2);";
                let params = params![self.sector_id, &self.port_id.unwrap()];
                database.execute(statement, params)?;
            }

            Ok(())
        }() {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("Cannot persist sector:{}", e)),
        }
    }
}
