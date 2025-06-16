use std::sync::{LazyLock, Mutex};
use rusqlite::{Connection, OpenFlags};

use space_trader::universe::Universe;

static UNIVERSE: LazyLock<Mutex<Universe>> = LazyLock::new(|| Mutex::new(Universe::new()));

fn main() {
    println!("Space Trader - engine");

    let database = match Connection::open_with_flags("space-trader.db",
                                                     OpenFlags::SQLITE_OPEN_READ_WRITE) {
        Ok(connection) => connection,
        Err(err) => {
            panic!("Failed to open database: {}", err);
        }
    };

    UNIVERSE.lock().unwrap().load(&database);
}
