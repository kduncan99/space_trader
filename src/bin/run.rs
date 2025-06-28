use rusqlite::{Connection, OpenFlags, Result};

use space_trader::server::Server;
use space_trader::universe::UNIVERSE;

fn main() {
    println!("Space Trader");

    match setup() {
        Ok(connection) => {
            let s = Server::new(connection);
            s.process();
            println!("Terminated")
        },
        Err(err) => {
            panic!("Failed to open or load database: {}", err);
        }
    }
}

fn setup() -> Result<(Connection)> {
    let database = Connection::open_with_flags("space-trader.db", OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    UNIVERSE.lock().unwrap().load(&database)?;
    
    Ok(database)
}
