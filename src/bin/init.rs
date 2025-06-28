use rusqlite::{Connection, OpenFlags, Result};

use space_trader::universe::UNIVERSE;

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

    "CREATE TABLE users (\
                userId INTEGER PRIMARY KEY NOT NULL, \
                userName TEXT NOT NULL UNIQUE, \
                password TEXT, \
                gameName TEXT, \
                isDisabled INTEGER NOT NULL,\
                requestsPerDay INTEGER, \
                requestsRemaining INTEGER);",

    "CREATE TABLE galaxies ( \
                galaxyId INTEGER PRIMARY KEY NOT NULL, \
                galaxyName TEXT NOT NULL);",

    "CREATE TABLE sectors ( \
                sectorId INTEGER PRIMARY KEY NOT NULL);",

    "CREATE TABLE planets ( \
                planetId INTEGER NOT NULL,\
                planetName STRING);",

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

fn main() {
    println!("Space Trader - initializer");

    match build_database() {
        Ok(_) => println!("Successfully initialized database"),
        Err(msg) => panic!("Failed to initialize database:{msg}"),
    }
}

fn build_database() -> Result<()> {
    let database = Connection::open_with_flags("space-trader.db",
                                               OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    for statement in DB_BUILD_STATEMENTS {
        database.execute(statement, ())?;
    }

    UNIVERSE.lock().unwrap().initialize(database)?;

    Ok(())
}
