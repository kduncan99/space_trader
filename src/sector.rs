use crate::planet::{Planet, PlanetId};
use crate::port::{Port, PortId};

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, Result};

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
        Sector { sector_id, planet: None, port: None, links: HashSet::new() }
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
        let mut result: Vec<String> = Vec::new();
        result.push(format!("Sector {}", self.sector_id));
        let string_numbers: Vec<String> = self.links.iter()
            .map(|&n| n.to_string())
            .collect();
        result.push(format!("  Links to: {}", string_numbers.join(" ")));
        if self.planet.is_some() {
            result.push(format!("  Port: {}", self.get_planet().get_planet_name()));
        }
        if self.port.is_some() {
            let port_name: &String = self.port.as_ref().unwrap().get_port_name();
            result.push(format!("  Port: {}", port_name));
        }
        result
    }

    // for use by Galaxy::load_galaxies
    // Creates a map of all sectors in the universe - Used at game startup.
    pub fn load_sectors(database: &Connection,
                        planets: &mut HashMap<PlanetId, Planet>,
                        ports: &mut HashMap<PortId, Port>) -> Result<HashMap<SectorId, Sector>> {
        // initial load of all sectors
        let mut sector_ids: Vec<SectorId> = vec![];
        let select_sql = "SELECT sectorId FROM sectors ORDER BY sectorId";
        _ = database.prepare(select_sql)?.query_map([], |row| {
            let sector_id : SectorId = row.get(0)?;
            sector_ids.push(sector_id);
            NEXT_SECTOR_ID.store(sector_id, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        // Iterate over the sectors to load links, planets, and ports
        let mut sectors: HashMap<SectorId, Sector> = HashMap::new();
        for sector_id in sector_ids {
            let mut links: HashSet<SectorId> = HashSet::new();
            let select_sql = "SELECT toSectorId FROM sectors_to_planets WHERE sectorId = :sectorId";
            _ = database.prepare(select_sql)?.query_map(&[(":sectorId", &sector_id.to_string())], |row| {
                links.insert(row.get(0)?);
                Ok(())
            });

            let mut planet: Option<Planet> = None;
            let select_sql = "SELECT planetId FROM sectors_to_planets WHERE sectorId = :sectorId";
            _ = database.prepare(select_sql)?.query_map(&[(":sectorId", &sector_id.to_string())], |row| {
                let planet_id : PlanetId = row.get(0)?;
                planet = planets.remove(&planet_id);
                Ok(())
            });

            let mut port: Option<Port> = None;
            let select_sql = "SELECT portId FROM sectors_to_ports WHERE sectorId = :sectorId";
            _ = database.prepare(select_sql)?.query_map(&[(":sectorId", &sector_id.to_string())], |row| {
                let port_id : PortId = row.get(0)?;
                port = ports.remove(&port_id);
                Ok(())
            });

            sectors.insert(sector_id, Sector{sector_id, planet, port, links});
        }

        Ok(sectors)
    }

    /// Writes information about this sector to the database.
    /// To be used when the sector is first created.
    pub fn persist(&self, database: &Connection) -> Result<()> {
        let statement = "INSERT INTO sectors (sectorId) VALUES (?1);";
        let params = params![self.sector_id];
        database.execute(statement, params)?;

        for link in self.links.iter() {
            let statement = "INSERT INTO sector_links (fromSectorId, toSectorId) VALUES (?1, ?2);";
            let params = params![self.sector_id, *link];
            database.execute(statement, params)?;
        }

        if self.has_planet() {
            self.get_planet().persist(database)?;

            let statement = "INSERT INTO sectors_to_planets (sectorId, planetId) VALUES (?1, ?2);";
            let params = params![self.sector_id, &self.get_planet().get_planet_id()];
            database.execute(statement, params)?;
        }

        if self.has_port() {
            self.get_port().persist(database)?;

            let statement = "INSERT INTO sectors_to_ports (sectorId, portId) VALUES (?1, ?2);";
            let params = params![self.sector_id, &self.get_port().get_port_id()];
            database.execute(statement, params)?;
        }
        
        Ok(())
    }
}
