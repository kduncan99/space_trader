use rusqlite::{Connection, OpenFlags};
use space_trader::galaxy::Galaxy;

pub const BUILD_STATEMENTS: &'static [&'static str] = &[
    "DROP TABLE IF EXISTS galaxies_to_sectors;",
    "DROP TABLE IF EXISTS sector_links;",
    "DROP TABLE IF EXISTS galaxies;",
    "DROP TABLE IF EXISTS sectors;",
    "DROP TABLE IF EXISTS ports;",
    "DROP TABLE IF EXISTS users;",

    "CREATE TABLE galaxies ( \
                galaxyId INTEGER PRIMARY KEY NOT NULL, \
                galaxyName TEXT NOT NULL);",

    "CREATE TABLE sectors ( \
                sectorId INTEGER PRIMARY KEY NOT NULL, \
                portId INTEGER);",

    "CREATE TABLE ports ( \
                portId INTEGER NOT NULL,\
                portNameIndex INTEGER);",

    "CREATE TABLE galaxies_to_sectors ( \
                galaxyId INTEGER REFERENCES galaxy(galaxyId), \
                sectorId INTEGER REFERENCES sectors(sectorId), \
                PRIMARY KEY (galaxyId, SectorId));",

    "CREATE TABLE sector_links ( \
                fromSectorId INTEGER REFERENCES sectors(sectorId), \
                toSectorId INTEGER REFERENCES sectors(sectorId), \
                PRIMARY KEY (fromSectorId, toSectorId));",
];

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
    let g = Galaxy::new_conventional_galaxy(1, String::from("Kronos"), 50);
    g.dump();
    g.persist(&database);
}

fn build_database(database: &Connection) -> bool {
    let mut result = true;
    for statement in BUILD_STATEMENTS {
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
