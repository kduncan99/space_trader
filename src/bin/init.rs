use std::sync::{LazyLock, Mutex};
use rusqlite::{Connection, OpenFlags};

use space_trader::universe::Universe;

pub const DB_BUILD_STATEMENTS: &'static [&'static str] = &[
    "DROP TABLE IF EXISTS galaxies_to_sectors;",
    "DROP TABLE IF EXISTS sector_links;",
    "DROP TABLE IF EXISTS sectors_to_planets;",
    "DROP TABLE IF EXISTS sectors_to_ports;",
    "DROP TABLE IF EXISTS galaxies;",
    "DROP TABLE IF EXISTS sectors;",
    "DROP TABLE IF EXISTS planets;",
    "DROP TABLE IF EXISTS ports;",
    "DROP TABLE IF EXISTS users;",

    "CREATE TABLE galaxies ( \
                galaxyId INTEGER PRIMARY KEY NOT NULL, \
                galaxyName TEXT NOT NULL);",

    "CREATE TABLE sectors ( \
                sectorId INTEGER PRIMARY KEY NOT NULL);",

    "CREATE TABLE planets ( \
                portId INTEGER NOT NULL,\
                portNameIndex INTEGER);",

    "CREATE TABLE ports ( \
                portId INTEGER NOT NULL,\
                portName STRING);",

    "CREATE TABLE sector_links ( \
                fromSectorId INTEGER REFERENCES sectors(sectorId), \
                toSectorId INTEGER REFERENCES sectors(sectorId), \
                PRIMARY KEY (fromSectorId, toSectorId));",

    "CREATE TABLE sectors_to_planets ( \
                sectorId INTEGER REFERENCES sectors(sectorId), \
                planetId INTEGER REFERENCES planets(planetId), \
                PRIMARY KEY (sectorId, planetId));",

    "CREATE TABLE sectors_to_ports ( \
                sectorId INTEGER REFERENCES sectors(sectorId), \
                portId INTEGER REFERENCES ports(portId), \
                PRIMARY KEY (sectorId, PortId));",

    "CREATE TABLE galaxies_to_sectors ( \
                galaxyId INTEGER REFERENCES galaxy(galaxyId), \
                sectorId INTEGER REFERENCES sectors(sectorId), \
                PRIMARY KEY (galaxyId, SectorId));",
];

static UNIVERSE: LazyLock<Mutex<Universe>> = LazyLock::new(|| Mutex::new(Universe::new()));

fn main() {
    println!("Space Trader - initializer");

    let database = match Connection::open_with_flags("space-trader.db",
                                                     OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE) {
        Ok(connection) => connection,
        Err(err) => {
            panic!("Failed to open database: {}", err);
        }
    };

    build_database(&database);
    UNIVERSE.lock().unwrap().initialize(database);
}

fn build_database(database: &Connection) -> bool {
    let mut result = true;
    for statement in DB_BUILD_STATEMENTS {
        match database.execute(statement, ()) {
            Ok(_) => (),
            Err(err) => {
                println!("Failed to build database: {}", err);
                println!("{}", statement);
                result = false;
                break;
            }
        }
    }

    result
}
