use std::sync::{LazyLock, Mutex};
use rusqlite::{Connection, OpenFlags, Result};

use space_trader::universe::Universe;

static UNIVERSE: LazyLock<Mutex<Universe>> = LazyLock::new(|| Mutex::new(Universe::new()));

fn main() {
    println!("Space Trader - engine");

    match process() {
        Ok(_) => println! ("Space Trader exited successfully"),
        Err(err) => {
            panic!("Failed to open database: {}", err);
        }
    }
}

fn process() -> Result<()> {
    let database = Connection::open_with_flags("space-trader.db", OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    UNIVERSE.lock().unwrap().load(&database)?;
    Ok(())
}