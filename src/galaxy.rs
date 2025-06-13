use std::cmp::{max, min};
pub use crate::sector::*;

use std::collections::{HashMap, HashSet};
use rand::Rng;
use rusqlite::Connection;

pub type GalaxyId = u16;

pub struct Galaxy {
    pub galaxy_id: GalaxyId,
    pub name: String,
    pub sectors: HashMap<SectorId, Sector>,
}

impl Galaxy {
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
    /// * `branch_count` number of branches per sector
    /// * `sector_count` least number of sectors to be created for this galaxy
    pub fn new_conventional_galaxy(galaxy_id: GalaxyId, name: String, sector_count: isize) -> Galaxy {
        let mut sector_map = HashMap::<SectorId, Sector>::new();

        // create all the sectors first.
        for sector_id in 1..(sector_count + 1) as SectorId {
            sector_map.insert(sector_id, Sector::new(sector_id));
        }

        // now do initial random linking.
        let mut rng = rand::rng();
        for sector_id in 1..(sector_count + 1) as SectorId {
            if sector_map.get(&sector_id).unwrap().links.len() < 6 {
                let mut target_id = sector_id;
                while target_id == sector_id || sector_map.get(&sector_id).unwrap().links.len() == 6 {
                    let range_low = max((sector_id as i32) - 10, 1) as SectorId;
                    let range_high = min(sector_id + 10, sector_count as SectorId);
                    target_id = rng.random_range(range_low..range_high + 1);
                }
                sector_map.get_mut(&sector_id).unwrap().links.insert(target_id);
                sector_map.get_mut(&target_id).unwrap().links.insert(sector_id);
            }
        }

        // Create a map of sectors and their distance from the root sector.
        // Note that we're really interested in the distance from that sector to the root,
        // not vice versa. At this point however, these two values are the same.
        fn distance_func(sector_map: &HashMap<SectorId, Sector>, distances: &mut HashMap<SectorId, isize>, base_id: SectorId, base_distance: isize) {
            distances.insert(base_id, base_distance);
            for link_id in sector_map.get(&base_id).unwrap().links.iter() {
                if !distances.contains_key(link_id) {
                    distance_func(sector_map, distances, *link_id, base_distance + 1)
                }
            }
        }

        let mut distances = HashMap::<SectorId, isize>::new();
        distance_func(&sector_map, &mut distances, 1, 0);

        // Now look for sectors for which we do not have a distance - this is a disjoint sector,
        // and we need to link it somewhere into the non-disjoint group, then calculate distances again.
        for sector_id in 1..(sector_count + 1) as SectorId {
            if !distances.contains_key(&sector_id) {
                let mut target_id = sector_id;
                while !distances.contains_key(&target_id) {
                    target_id = rng.random_range((1 as SectorId)..(sector_count as SectorId));
                }
                sector_map.get_mut(&sector_id).unwrap().links.insert(target_id);
                sector_map.get_mut(&target_id).unwrap().links.insert(sector_id);

                let new_distance = distances.get(&target_id).unwrap() + 1;
                distance_func(&sector_map, &mut distances, sector_id, new_distance);
            }
        }

        // Finally, look at all the distances. As we find sectors which are too far from the root sector,
        // link them one-way thereto, then recalculate distances for proximate sectors so we don't
        // link more than we have to.
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

        const DISTANCE_LIMIT: isize = 20;
        for sector_id in 1..(sector_count + 1) as SectorId {
            if distances[&sector_id] > DISTANCE_LIMIT {
                sector_map.get_mut(&sector_id).unwrap().links.insert(1);
                distances.insert(sector_id, 1);
                distance_recalculate_func(&sector_map, &mut distances, sector_id);
            }
        }

        // All done.
        Galaxy { galaxy_id, name, sectors: sector_map }
    }

    /// Creates a tree-oriented Galaxy, and incorporates it into the universe.
    /// The galaxy has a root sector, and each sector including the root sector will have a fixed
    /// number of branches to child sectors (see branch_count), excepting the final sectors
    /// at the conceptual edge of the galaxy. Each sector excluding the root sector will also
    /// have a link back to its root branch, for a total number of n+1 links, where n is branch_count.
    /// We guarantee at least sector_count sectors, but we may create a few additional sectors.
    ///
    /// # Arguments
    /// * `branch_count` number of branches per sector
    /// * `sector_count` least number of sectors to be created for this galaxy
    pub fn new_tree_galaxy(galaxy_id: GalaxyId, name: String, branch_count: isize, sector_count: isize) -> Galaxy {
        let mut sector_map = HashMap::<SectorId, Sector>::new();

        // create all the sectors first.
        for sector_id in 1..(sector_count + 1) as SectorId {
            sector_map.insert(sector_id, Sector::new(sector_id));
        }

        // now link the tree.
        let mut base_id: SectorId = 1;
        let mut target_id: SectorId = 2;
        while target_id <= sector_count as SectorId {
            sector_map.get_mut(&base_id).unwrap().links.insert(target_id);
            sector_map.get_mut(&target_id).unwrap().links.insert(base_id);
            target_id += 1;
            if sector_map.get_mut(&base_id).unwrap().links.len() > branch_count as usize {
                base_id += 1;
            }
        }

        Galaxy { galaxy_id, name, sectors: sector_map }
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

    /// Invoked by the initializer to store everything to the database...
    /// Not intended for use during engine processing, since all persistence during execution is
    /// piecemeal.
    pub fn persist(&self, database: &Connection) {
        for sector in self.sectors.values() {
            sector.persist(database);
        }
    }
}
