use crate::sector::{SectorId, Sector};
use crate::planet::{PlanetId, Planet};
use crate::port::{PortId, Port};

use rand::Rng;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::cmp::{max, min};

pub type GalaxyId = usize;

pub struct Galaxy {
    pub galaxy_id: GalaxyId,
    pub galaxy_name: String,
    pub sectors: HashMap<SectorId, Sector>,
    pub planets: HashMap<PlanetId, Planet>,
    pub ports: HashMap<PortId, Port>,
}

impl Galaxy {
    pub fn new(galaxy_id: GalaxyId,
           galaxy_name: String,
    ) -> Galaxy {
        Galaxy{galaxy_id, galaxy_name, sectors: Default::default(), planets: Default::default(), ports: Default::default() }
    }

    /// Creates a legacy Galaxy, and incorporates it into the universe.
    /// Such a galaxy has a fixed number of sector. It has a root sector, at sector ID 1.
    /// 1) Create the root sector
    /// 2) For each sector from 1 to n, we randomly choose a linking sector within 10 sector ids
    /// (inclusive) of that sector's id, and link the two sectors.
    /// 3) Find disjoint sectors (sectors which have to path to the root sector) and un-disjoint them.
    /// Note that connecting one disjoint sector may indirectly connect multiple other disjoint sectors.
    /// 4) Locate any sectors which are greater than some fixed distance from sector 1,
    /// and link them one-way to sector 1 (this ensuring that any ship in the galaxy is no further
    /// than this distance from sector 1, which will contain a port with fuel).
    ///
    /// # Arguments
    /// * `galaxy_id` a unique GalaxyId to identify this galaxy
    /// * `galaxy_name` admin-supplied galaxy name. Be creative.
    /// * `first_sector_id` since sectors across the universe have unique ids, we need to know the first usable sector id
    /// * `first_port_id` as above, but for ports
    /// * `sector_count` number of sectors to be created for this galaxy
    pub fn new_conventional_galaxy(galaxy_id: GalaxyId,
                                   galaxy_name: String,
                                   first_sector_id: SectorId,
                                   first_port_id: PortId,
                                   sector_count: usize) -> Galaxy {
        let mut galaxy = Galaxy::new(galaxy_id, galaxy_name);

        // create all the sectors first.
        println!("Creating {} sectors...", sector_count);
        let mut sector_id = first_sector_id;
        for _ in 0..sector_count {
            galaxy.sectors.insert(sector_id, Sector::new(sector_id));
            sector_id += 1;
        }

        // now do initial random linking.
        println!("Linking sectors...");
        let mut rng = rand::rng();
        for sector_id in 1..(sector_count + 1) as SectorId {
            if galaxy.sectors.get(&sector_id).unwrap().links.len() < 6 {
                let mut target_id = sector_id;
                while target_id == sector_id || galaxy.sectors.get(&target_id).unwrap().links.len() == 6 {
                    let range_low = max((sector_id as isize) - 10, 1) as usize;
                    let range_high = min(sector_id + 10, sector_count as SectorId);
                    target_id = rng.random_range(range_low..range_high + 1);
                }
                galaxy.sectors.get_mut(&sector_id).unwrap().links.insert(target_id);
                galaxy.sectors.get_mut(&target_id).unwrap().links.insert(sector_id);
            }
        }

        // Create a map of sectors and their distance from the root sector.
        // Note that we're really interested in the distance from that sector to the root,
        // not vice versa. At this point however, these two values are the same.
        // This should really be a closure since rust functions cannot see things in their containing scope...
        // but closures in rust cannot recurse, and we need to do that.
        fn distance_func(sector_map: &HashMap<SectorId, Sector>, distances: &mut HashMap<SectorId, isize>, base_id: SectorId, base_distance: isize) {
            distances.insert(base_id, base_distance);
            for link_id in sector_map.get(&base_id).unwrap().links.iter() {
                if !distances.contains_key(link_id) {
                    distance_func(sector_map, distances, *link_id, base_distance + 1)
                }
            }
        }

        println!("Cross-linking disjoint globs...");
        let mut distances = HashMap::<SectorId, isize>::new();
        distance_func(&galaxy.sectors, &mut distances, 1, 0);

        // Now look for sectors for which we do not have a distance - this is a disjoint sector,
        // and we need to link it somewhere into the non-disjoint group, then calculate distances again.
        for sector_id in 1..(sector_count + 1) as SectorId {
            if !distances.contains_key(&sector_id) {
                let mut target_id = sector_id;
                while !distances.contains_key(&target_id) {
                    target_id = rng.random_range((1 as SectorId)..((sector_count + 1) as SectorId));
                }
                galaxy.sectors.get_mut(&sector_id).unwrap().links.insert(target_id);
                galaxy.sectors.get_mut(&target_id).unwrap().links.insert(sector_id);
                println!("  Linking sectors {} and {}", sector_id, target_id);

                let new_distance = distances.get(&target_id).unwrap() + 1;
                distance_func(&galaxy.sectors, &mut distances, sector_id, new_distance);
            }
        }

        // Finally, look at all the distances. As we find sectors which are too far from the root sector,
        // link them one-way thereto, then recalculate distances for proximate sectors so we don't
        // link more than we have to. This should also be a closure, but...
        fn distance_recalculate_func(sector_map: &HashMap<SectorId, Sector>, distances: &mut HashMap<SectorId, isize>, base_id: SectorId) {
            // The recursion here is self-limiting - we cannot recurse into places we've already been
            // because they will have a smaller distance than we are looking for, for recursing.
            let our_distance = distances.get(&base_id).unwrap();
            let new_distance = our_distance + 1;
            for link_id in sector_map.get(&base_id).unwrap().links.iter() {
                if *distances.get_mut(link_id).unwrap() > new_distance {
                    distances.insert(*link_id, new_distance);
                    distance_recalculate_func(sector_map, distances, *link_id);
                }
            }
        }

        println!("Creating one-way links back to root sector...");
        const DISTANCE_LIMIT: isize = 20;
        for sector_id in 1..(sector_count + 1) as SectorId {
            if distances[&sector_id] > DISTANCE_LIMIT {
                galaxy.sectors.get_mut(&sector_id).unwrap().links.insert(1);
                println!("  Linking sector {} to sector 1", sector_id);
                distances.insert(sector_id, 1);
                distance_recalculate_func(&galaxy.sectors, &mut distances, sector_id);
            }
        }

        // Create some ports. We create 1 port per 15 sectors,
        // so a galaxy of 1000 sectors would contain 66 ports.
        // Ports are randomly assign to sectors according to the following rules:
        // * the sector must be at least 3 sectors from the root
        // * a sector can have at most one port.
        println!("Creating ports...");
        let mut remaining = sector_count / 15;
        let mut port_id = first_port_id;
        while remaining > 0 {
            let sector_id = rng.random_range((1 as SectorId)..((sector_count + 1) as SectorId));
            let sector = galaxy.sectors.get_mut(&sector_id).unwrap();
            if sector.port_id.is_none() && distances[&sector_id] >= 3 {
                let port = Port::new(port_id);
                println!("Port {} ({}) is at sector {}", port_id, port.port_name(), sector_id);
                galaxy.ports.insert(port_id, port);
                port_id += 1;
                remaining -= 1;
            }
        }

        // All done.
        galaxy
    }

    /// Creates a tree-oriented Galaxy, and incorporates it into the universe.
    /// The galaxy has a root sector, and each sector including the root sector will have a fixed
    /// number of branches to child sectors (see branch_count), excepting the final sectors
    /// at the conceptual edge of the galaxy. Each sector excluding the root sector will also
    /// have a link back to its root branch, for a total number of n+1 links, where n is branch_count.
    /// We guarantee at least sector_count sectors, but we may create a few additional sectors.
    ///
    /// # Arguments
    /// * `galaxy_id` a unique GalaxyId to identify this galaxy
    /// * `galaxy_name` admin-supplied galaxy name. Be creative.
    /// * `first_sector_id` since sectors across the universe have unique ids, we need to know the first usable sector id
    /// * `first_port_id` as above, but for ports
    /// * `branch_count` number of branches per sector
    /// * `sector_count` least number of sectors to be created for this galaxy
    pub fn new_tree_galaxy(galaxy_id: GalaxyId,
                           galaxy_name: String,
                           first_sector_id: SectorId,
                           first_port_id: PortId,
                           branch_count: usize,
                           sector_count: usize) -> Galaxy {
        let mut galaxy = Galaxy::new(galaxy_id, galaxy_name);

        // create all the sectors first.
        let mut sector_id = first_sector_id;
        for _ in 0..sector_count {
            galaxy.sectors.insert(sector_id, Sector::new(sector_id));
            sector_id += 1;
        }

        // now link the tree.
        let mut base_id: SectorId = 1;
        let mut target_id: SectorId = 2;
        while target_id <= sector_count as SectorId {
            galaxy.sectors.get_mut(&base_id).unwrap().links.insert(target_id);
            galaxy.sectors.get_mut(&target_id).unwrap().links.insert(base_id);
            target_id += 1;
            if galaxy.sectors.get_mut(&base_id).unwrap().links.len() > branch_count as usize {
                base_id += 1;
            }
        }

        // TODO create ports

        galaxy
    }

    // only for debugging purposes
    pub fn dump(&self) {
        for sector in self.sectors.values() {
            let mut str: String = format!("{} ->", sector.sector_id); //"".to_owned();
            for link in sector.links.iter() {
                let sub_str = format!(" {}", link);
                str.push_str(&sub_str);
            }
            println!("{}", str);
        }
    }

    /// Convenience function to create a bidirectional link between two sectors
    /// in this galaxy - if the sectors do not exist, or if the same sector is
    /// presented twice, we do nothing.
    pub fn link_sectors(&mut self, sector_id_1: SectorId, sector_id_2: SectorId) {
        if sector_id_1 != sector_id_2
            && self.sectors.contains_key(&sector_id_1)
            && self.sectors.contains_key(&sector_id_2) {
            self.sectors.get_mut(&sector_id_1).unwrap().links.insert(sector_id_2);
            self.sectors.get_mut(&sector_id_2).unwrap().links.insert(sector_id_1);
        }
    }

    /// Finds the shortest path from one sector to another sector.
    /// The result will contain an ordered list of SectorId values indicating the path
    /// from the first sector (not inclusive), to the targeted sector (inclusive).
    /// If the result is empty, there is no such path available.
    ///
    /// # Arguments
    /// * `from` sector id of the starting sector
    /// * `to` sector id of the sector we're trying to reach
    pub fn find_shortest_path(&self, from: SectorId, to: SectorId) -> Vec<SectorId> {
        self.find_shortest_path_avoiding(from, to, &HashSet::new())
    }

    /// Finds the shortest path from this sector to the indicated sector.
    /// This version observes a provided list of sectors to be avoided.
    /// The result will contain an ordered list of SectorId values indicating the path
    /// from this sector (not inclusive), to the targeted sector (inclusive).
    /// If the result is empty, there is no such path available.
    /// If either sector ID is not found in this galaxy, or if from and to are the same,
    /// the result is empty.
    ///
    /// # Arguments
    /// * `from` sector id of the starting sector
    /// * `to` sector id of the sector we're trying to reach
    /// * `avoiding` set of sector ids of sectors we do not wish to traverse, or which we have
    /// * already traversed (so that we don't infinitely loop)
    pub fn find_shortest_path_avoiding(&self, from: SectorId, to: SectorId, avoiding: &HashSet<SectorId>) -> Vec<SectorId> {
        let mut result: Vec<SectorId> = Vec::new();

        let sector1 = &self.sectors.get(&from);
        let sector2 = &self.sectors.get(&to);
        if sector1.is_some() && sector2.is_some() {
            let sector1 = sector1.unwrap();

            // First loop - look for the short completion
            for sector_id in sector1.links.iter() {
                if *sector_id == to {
                    result.push(to);
                    break;
                }
            }

            if result.is_empty() {
                // Second loop - recurse over links which are not in the avoid list.
                let mut sub_avoiding: HashSet<SectorId> = avoiding.clone();
                let mut pending_result: Vec<SectorId> = Vec::new();
                sub_avoiding.insert(from);

                for sector_id in sector1.links.iter() {
                    let sub_result = self.find_shortest_path_avoiding(*sector_id, to, &sub_avoiding);
                    if !sub_result.is_empty() {
                        if pending_result.is_empty() || sub_result.is_empty() {
                            pending_result = sub_result;
                        }
                    }
                }
            }
        }

        result
    }

    /// Loads the indicated galaxy from the database connection.
    pub fn load(&self, database: &Connection) {
        println!("Loading galaxy {}:{}", self.galaxy_id, self.galaxy_name);
        //TODO
        // SELECT sectors.sectorId FROM sectors JOIN galaxies_to_sectors WHERE sectors.sectorId == galaxies_to_sectors.sectorId;
        //   iterate over result to create Sector objects and store them in galaxy
        //   SELECT planets.planet_id planets.planet_name FROM planets JOIN sectors_to_planets WHERE sector_id == {};
        //   if any (there will be one at most)
        //     insert the planet-ids into the sector
        //     insert the planets into the galaxy
        //   SELECT ports.port_id ports.port_name_index FROM ports JOIN sectors_to_ports WHERE sector_id == {};
        //   if any (there will be one at most)
        //     insert the ports into the galaxy
        //     insert the port-ids into the sector
    }

    /// Invoked by the initializer to store everything to the database...
    /// Not intended for use during engine processing, since all persistence during execution is piecemeal.
    pub fn persist(&self, database: &Connection) {
        let statement = "INSERT INTO galaxies (galaxyId, galaxyName) VALUES (?1, ?2);";
        let params = params![self.galaxy_id, self.galaxy_name];
        match database.execute(statement, params) {
            Ok(_) => (),
            Err(err) => {
                println!("Database error: {}", err);
                println!("{}", statement);
                panic!("Shutting down");
            }
        }

        for sector in self.sectors.values() {
            let statement = "INSERT INTO galaxies_to_sectors (galaxyId, sectorId) VALUES (?1, ?2);";
            let params = params![self.galaxy_id, sector.sector_id];
            match database.execute(statement, params) {
                Ok(_) => (),
                Err(err) => {
                    println!("Database error: {}", err);
                    println!("{}", statement);
                    panic!("Shutting down");
                }
            }

            sector.persist(database);
        }

        for planet in self.planets.values() {
            planet.persist(database);
        }

        for port in self.ports.values() {
            port.persist(database);
        }
    }
}
