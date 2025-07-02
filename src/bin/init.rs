use rusqlite::{Connection, OpenFlags};
use space_trader::galaxy;
use space_trader::user;

pub const DB_BUILD_STATEMENTS: &'static [&'static str] = &[
    "DROP TABLE IF EXISTS galaxies_to_sectors;",
    "DROP TABLE IF EXISTS sector_links;",
    "DROP TABLE IF EXISTS sectors_to_planets;",
    "DROP TABLE IF EXISTS sectors_to_ports;",
    "DROP TABLE IF EXISTS sectors;",
    "DROP TABLE IF EXISTS planets;",
    "DROP TABLE IF EXISTS ports;",
    "DROP TABLE IF EXISTS galaxies;",
    "DROP TABLE IF EXISTS messages;",
    "DROP TABLE IF EXISTS users;",

    "CREATE TABLE users ( \
                userId INTEGER PRIMARY KEY NOT NULL, \
                userName TEXT NOT NULL UNIQUE, \
                password TEXT, \
                gameName TEXT, \
                lastLoginTimeStamp INTEGER, \
                isDisabled INTEGER NOT NULL,\
                requestsPerDay INTEGER, \
                requestsRemaining INTEGER);",

    "CREATE TABLE messages ( \
                messageId INTEGER PRIMARY KEY NOT NULL, \
                fromUserId INTEGER REFERENCES users(userId), \
                toUserId INTEGER NOT NULL REFERENCES users(userId), \
                timeStamp INTEGER NOT NULL, \
                text STRING);",

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

fn build_database() -> Result<(), String> {
    let database = match || -> rusqlite::Result<Connection> {
        let database = Connection::open_with_flags("space-trader.db",
                                                   OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        for statement in DB_BUILD_STATEMENTS {
            database.execute(statement, ())?;
        }

        Ok(database)
    }() {
        Ok(database) => database,
        Err(msg) => return Err(format!("Failed to build database:{msg}")),
    };

    _ = user::create_admin_user(&database)?;
    _ = user::create_normal_user(&database, "Neo".to_string(), "anderson".to_string(), "The One".to_string());
    _ = galaxy::create_conventional_galaxy(&database, "Kronos".to_string(), 500)?;

    Ok(())
}
