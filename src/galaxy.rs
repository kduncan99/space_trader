use crate::sector::{SectorId, Sector};
use crate::port::Port;

use rand::Rng;
use rusqlite::{params, Connection, Result};
use std::collections::{HashMap, HashSet, LinkedList};
use rusqlite::fallible_iterator::FallibleIterator;
use crate::planet::Planet;

pub type GalaxyId = usize;

pub struct Galaxy {
    galaxy_id: GalaxyId,
    galaxy_name: String,
    sectors: HashMap<SectorId, Sector>,
}

impl Galaxy {
    pub fn new(galaxy_id: GalaxyId,
           galaxy_name: String,
    ) -> Galaxy {
        Galaxy{galaxy_id, galaxy_name, sectors: Default::default()}
    }

    pub fn get_galaxy_id(&self) -> GalaxyId {
        self.galaxy_id
    }

    pub fn get_galaxy_name(&self) -> &String {
        &self.galaxy_name
    }

    pub fn get_sector(&self, sector_id: SectorId) -> &Sector {
        self.sectors.get(&sector_id).unwrap()
    }

    pub fn get_mut_sector(&mut self, sector_id: SectorId) -> &mut Sector {
        self.sectors.get_mut(&sector_id).unwrap()
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
    /// * `sector_count` number of sectors to be created for this galaxy
    pub fn new_conventional_galaxy(galaxy_id: GalaxyId,
                                   galaxy_name: String,
                                   sector_count: usize) -> Galaxy {
        let mut galaxy = Galaxy::new(galaxy_id, galaxy_name);

        // create all the sectors first.
        // start with root sector, then do all the rest.
        println!("Creating {} sectors...", sector_count);
        let root_sector = Sector::new();
        let root_sector_id = root_sector.get_sector_id();
        galaxy.sectors.insert(root_sector_id, root_sector);

        let mut last_sector_id = root_sector_id;
        for _ in 1..sector_count {
            let sector = Sector::new();
            last_sector_id = sector.get_sector_id();
            galaxy.sectors.insert(sector.get_sector_id(), sector);
        }

        // Now do initial random-ish linking. For each sector, link to another sector with a
        // sector-id within 10 (inclusive) above or below. Don't allow any sector to have
        // more than the max allowable number of links.
        println!("Linking sectors...");
        let highest_sector_id = (root_sector_id + sector_count - 1) as SectorId;
        let mut rng = rand::rng();
        for sector_id in root_sector_id..(last_sector_id + 1) as SectorId {
            if !galaxy.get_sector(sector_id).has_max_links() {
                loop {
                    // Set the range such that it allows values too low or too high.
                    // If the result is indeed out of range, bend it up or down as necessary.
                    // This gives us interesting artifacts near the root sector and its
                    // opposite ending sector, which we want for a legacy layout.
                    // This is a bit messy as we do want to play with potentially negative values
                    // (albeit briefly), and sector-ids are unsigned.
                    let offset = rng.random_range(0u32..21u32) as isize - 10;
                    if offset != 0 { // don't link to ourselves
                        let mut target_id = sector_id as isize + offset;
                        if target_id < root_sector_id as isize{
                            target_id = root_sector_id as isize;
                        } else if target_id > highest_sector_id as isize {
                            target_id = highest_sector_id as isize;
                        }

                        if !galaxy.get_sector(target_id as SectorId).has_max_links() {
                            galaxy.link_sectors(sector_id, target_id as SectorId);
                            break;
                        }
                    }
                }
            }
        }

        // Separate the mess into disjoint graphs, then link the disjoint parts so that we
        // have one completely connected graph.
        println!("Cross-linking disjoint globs...");
        let mut disjoint_sets = galaxy.create_disjoint_sector_sets(root_sector_id);
        /* TODO remove this later
        for set in &mut disjoint_sets {
            let mut str = format!("Set[{}]", set.len());
            for sector_id in set.iter() {
                str = format!("{} {}", str, sector_id);
            }
            println!("{}", str);
        }
        */

        // Note that we don't need to merge these globs - they're going away almost immediately.
        // We only use the globs as a means of choosing sectors to be linked, in order to
        // un-disjoint the globs. However, we get better link distribution if we *do* merge them.
        let mut main_glob = disjoint_sets.pop_front().unwrap();
        while !disjoint_sets.is_empty() {
            let disjoint_glob = disjoint_sets.pop_front().unwrap();

            // choose a random sector from the base glob and the disjoint glob and link them.
            let mut ix = rng.random_range(0..main_glob.len());
            while main_glob[ix] == root_sector_id {
                ix = rng.random_range(0..main_glob.len());
            }
            let sector_id1 = main_glob[ix];
            let iy = rng.random_range(0..disjoint_glob.len());
            let sector_id2 = disjoint_glob[iy];
            println!("  Linking disjoint {} to {}", sector_id2, sector_id1);
            galaxy.link_sectors(sector_id1, sector_id2);
            
            for disjoint_sector_id in disjoint_glob {
                main_glob.push(disjoint_sector_id);
            }
        }

        // Find all the dead-ends. We like dead-ends because things can hide there.
        // But we don't want too many of them.
        println!("Cross-linking excess dead ends...");
        let mut dead_ends = LinkedList::<SectorId>::new();
        for sector in galaxy.sectors.iter() {
            if sector.1.get_link_count() == 1 {
                dead_ends.push_back(*sector.0);
            }
        }

        while dead_ends.len() > sector_count / 10 {
            let sector_id1 = dead_ends.pop_front().unwrap();
            let sector_id2 = dead_ends.pop_front().unwrap();
            galaxy.link_sectors(sector_id1, sector_id2);
        }

        // breadth-first recursion - external caller should invoke with base_id set to root sector id,
        // and the distances map pre-populated with key of root sector id having value of zero.
        // We iterate over the neighbors setting them in the map with the distance one more than
        // that of the given base sector, then iterate again, recursing.
        fn distance_func(sector_map: &HashMap<SectorId, Sector>, distances: &mut HashMap<SectorId, isize>, base_id: SectorId) {
            let neighbor_distance = distances.get(&base_id).unwrap() + 1;
            let mut neighbors_to_visit: HashSet<SectorId> = HashSet::new();
            for neighbor_sector_id in sector_map.get(&base_id).unwrap().get_links() {
                if !distances.contains_key(&neighbor_sector_id) {
                    neighbors_to_visit.insert(neighbor_sector_id);
                    distances.insert(neighbor_sector_id, neighbor_distance);
                }
            }

            for neighbor_sector_id in neighbors_to_visit.iter() {
                distance_func(sector_map, distances, *neighbor_sector_id);
            }
        }

        fn distance_recalculate_func(galaxy: &Galaxy, distances: &mut HashMap<SectorId, isize>, base_id: SectorId) {
            // The recursion here is self-limiting - we cannot recurse into places we've already been
            // because they will have a smaller distance than we are looking for, for recursing.
            // We should do breadth-first, but this will work.
            let our_distance = distances.get(&base_id).unwrap();
            let new_distance = our_distance + 1;
            for link_id in galaxy.get_sector(base_id).get_links().iter() {
                if *distances.get_mut(link_id).unwrap() > new_distance {
                    distances.insert(*link_id, new_distance);
                    distance_recalculate_func(galaxy, distances, *link_id);
                }
            }
        }

        // Create a map of sectors and their distance from the root sector.
        // Note that we're really interested in the distance from that sector to the root,
        // not vice versa. At this point however, these two values are the same.
        // This should really be a closure since rust functions cannot see things in their containing scope...
        // but closures in rust cannot recurse, and we need to do that.
        let mut distances = HashMap::<SectorId, isize>::new();
        distances.insert(root_sector_id, 0);
        distance_func(&galaxy.sectors, &mut distances, root_sector_id);

        // Look at the distances. As we find sectors which are too far from the root sector,
        // link them one-way thereto, then recalculate distances for proximate sectors so we don't
        // link more than we have to. This should also be a closure, but...
        println!("Creating one-way links back to root sector if/as necessary...");
        const DISTANCE_LIMIT: isize = 20;
        for sector_id in root_sector_id..(last_sector_id + 1) as SectorId {
            if distances[&sector_id] > DISTANCE_LIMIT {
                println!("  Linking sector {} to root sector", sector_id);
                galaxy.get_mut_sector(sector_id).insert_link_to(root_sector_id);
                distances.insert(sector_id, 1);
                distance_recalculate_func(&galaxy, &mut distances, sector_id);
            }
        }

        // Create some ports. We create 1 port per 15 sectors,
        // so a galaxy of 1000 sectors would contain 66 ports.
        // Ports are randomly assigned to sectors according to the following rules:
        // * the sector must be at least 3 sectors from the root
        // * a sector can have at most one port.
        println!("Creating ports...");
        let mut remaining = sector_count / 15;
        while remaining > 0 {
            let sector_id = rng.random_range(root_sector_id..(last_sector_id + 1) as SectorId);
            let sector = galaxy.get_mut_sector(sector_id);
            if !sector.has_port() && distances[&sector_id] >= 3 {
                let port = Port::new();
                println!("Port {} ({}) is at sector {}", port.get_port_id(), port.get_port_name(), sector_id);
                sector.set_port(port);
                remaining -= 1;
            }
        }

        // All done.
        galaxy
    }

    // segregates the sector map into disjoint globs.
    // The glob containing the root sector id will be the first in the vector.
    pub fn create_disjoint_sector_sets(&self, root_sector_id: SectorId) -> LinkedList<Vec<SectorId>> {
        let mut disjoint_sector_sets = LinkedList::<Vec<SectorId>>::new();
        let mut unassigned_sectors = self.sectors.keys().cloned().collect::<HashSet<_>>();

        fn move_sector_id(sector_map: &HashMap<SectorId, Sector>,
                          sector_id: SectorId,
                          from_catalog: &mut HashSet<SectorId>,
                          to_set: &mut Vec<SectorId>) {
            if from_catalog.contains(&sector_id) {
                to_set.push(sector_id);
                from_catalog.remove(&sector_id);
                let sector = sector_map.get(&sector_id).unwrap();
                for neighbor_sector_id in sector.get_links() {
                    move_sector_id(sector_map, neighbor_sector_id, from_catalog, to_set);
                }
            }
        }

        let mut disjoint_set: Vec<SectorId> = Vec::new();
        move_sector_id(&self.sectors, root_sector_id, &mut unassigned_sectors, &mut disjoint_set);
        disjoint_sector_sets.push_back(disjoint_set);
        
        loop {
            let entry = { unassigned_sectors.iter().next() };
            if entry.is_none() {
                break
            }

            let sector_id = entry.unwrap();
            let mut disjoint_set: Vec<SectorId> = Vec::new();
            move_sector_id(&self.sectors, *sector_id, &mut unassigned_sectors, &mut disjoint_set);
            disjoint_sector_sets.push_back(disjoint_set);
        }

        disjoint_sector_sets
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
    /// * `branch_count` number of branches per sector
    /// * `sector_count` least number of sectors to be created for this galaxy
    pub fn new_tree_galaxy(galaxy_id: GalaxyId,
                           galaxy_name: String,
                           branch_count: usize,
                           sector_count: usize) -> Galaxy {
        let mut galaxy = Galaxy::new(galaxy_id, galaxy_name);

        println!("Creating ~{} sectors...", sector_count);
        let root_sector = Sector::new();
        let mut base_sector_id = root_sector.get_sector_id();
        galaxy.sectors.insert(root_sector.get_sector_id(), root_sector);

        let mut last_sector_id = base_sector_id;
        for _ in 1..sector_count {
            for _ in 0..branch_count {
                let sector = Sector::new();
                galaxy.link_sectors(base_sector_id, sector.get_sector_id());
            }

            last_sector_id = base_sector_id;
            base_sector_id += 1;
        }

        // Create some ports. We create 1 port per 15 sectors,
        // so a galaxy of 1000 sectors would contain 66 ports.
        // Ports are randomly assign to sectors according to the following rules:
        // * the sector must be at least 3 sectors from the root sector.
        // * a sector can have at most one port.
        /*
            branch count 1:  1  2  3                          -> 4 = 1**2 + 3
            branch count 2:  1  2,3  4,5,6,7                  -> 8 = 2**2 + 4
            branch count 3:  1  2,3,4  5,6,7,8,9,10,11,12,13  -> 14 = 3**2 + 5
            branch count 4:  1  2,3,4,5  6,7,8,9,10,11,12,13,14,15,16,17,18,29,20,21  -> 22 = 4**2 + 6
         */
        println!("Creating ports...");
        let mut rng = rand::rng();
        let lowest_target_sector_id = base_sector_id + (branch_count * branch_count) + branch_count + 2;
        let mut remaining = sector_count / 15;
        while remaining > 0 {
            let sector_id = rng.random_range(lowest_target_sector_id..(last_sector_id + 1) as SectorId);
            let sector = galaxy.get_mut_sector(sector_id);
            if !sector.has_port() {
                let port = Port::new();
                println!("Port {} ({}) is at sector {}", port.get_port_id(), port.get_port_name(), sector_id);
                sector.set_port(port);
                remaining -= 1;
            }
        }

        galaxy
    }

    // only for debugging purposes
    pub fn dump(&self) {
        for sector in self.sectors.values() {
            let mut str: String = format!(":{} ->", sector.get_sector_id()); //"".to_owned();
            for link in sector.get_links().iter() {
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
            self.sectors.get_mut(&sector_id_1).unwrap().insert_link_to(sector_id_2);
            self.sectors.get_mut(&sector_id_2).unwrap().insert_link_to(sector_id_1);
        }
    }

    pub fn link_sector_to(&mut self, from_sector_id: SectorId, to_sector_id: SectorId) {
        if from_sector_id != to_sector_id
            && self.sectors.contains_key(&from_sector_id)
            && self.sectors.contains_key(&to_sector_id) {
            self.sectors.get_mut(&from_sector_id).unwrap().insert_link_to(to_sector_id);
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

    /// Finds the length of the shortest path from one sector to another.
    ///
    /// # Arguments
    /// * `from` sector id of the starting sector
    /// * `to` sector id of the sector we're trying to reach
    pub fn find_shortest_path_len(&self, from: SectorId, to: SectorId) -> usize {
        self.find_shortest_path_avoiding(from, to, &HashSet::new()).len()
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
            for sector_id in sector1.get_links().iter() {
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

                for sector_id in sector1.get_links().iter() {
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

    /// Loads all the galaxies from the given database connection.
    ///
    /// # Arguments
    /// * `database` open database connection, containing a valid game database.
    pub fn load_galaxies(database: &Connection) -> Result<Vec<Galaxy>> {
        let mut planet_map = Planet::load_planets(database)?;
        println!("Loaded {} planets", planet_map.len());
        let mut port_map = Port::load_ports(database)?;
        println!("Loaded {} ports", port_map.len());
        let mut sector_map= Sector::load_sectors(database, &mut planet_map, &mut port_map)?;
        println!("Loaded {} sectors", sector_map.len());

        let mut stmt = database.prepare("SELECT galaxyId, galaxyName FROM galaxies;")?;
        let mapped_galaxies = stmt.query_map([], |row| {
            Ok(Galaxy{ galaxy_id: row.get(0)?, galaxy_name: row.get(1)?, sectors: Default::default() })
        })?;

        let mut galaxies: Vec<Galaxy> = Vec::new();
        for galaxy_result in mapped_galaxies {
            let mut galaxy = galaxy_result?;
            let mut galaxy_sector_map: HashMap<SectorId, Sector> = HashMap::new();
            
            let mut stmt =
                database.prepare("SELECT sectors.sectorId FROM sectors \
                    JOIN galaxies_to_sectors \
                    WHERE sectors.sectorId = galaxies_to_sectors.sectorId \
                    AND galaxies_to_sectors.galaxyId = :galaxyId")?;
            let mapped_sectors =
                stmt.query_map(&[(":galaxyId", &galaxy.galaxy_id.to_string())], |row| {
                    Ok(row.get(0)?)
            })?;

            for result_sector in mapped_sectors {
                let sector_id: SectorId = result_sector?;
                let sector = sector_map.remove(&sector_id).unwrap();
                galaxy_sector_map.insert(sector_id, sector);
            }

            println!("Loaded galaxy {}:{} with {} sectors", galaxy.galaxy_id, galaxy.galaxy_name, galaxy_sector_map.len());
            galaxy.sectors = galaxy_sector_map;
            galaxies.push(galaxy);
        }

        Ok(galaxies)
    }

    /// Invoked by the initializer to store everything to the database...
    /// Not intended for use during engine processing, since all persistence during execution is piecemeal.
    pub fn persist(&self, database: &Connection) -> Result<()> {
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
            let params = params![self.galaxy_id, sector.get_sector_id()];
            match database.execute(statement, params) {
                Ok(_) => (),
                Err(err) => {
                    println!("Database error: {}", err);
                    println!("{}", statement);
                    panic!("Shutting down");
                }
            }

            sector.persist(database)?;
        }
        
        Ok(())
    }
}
