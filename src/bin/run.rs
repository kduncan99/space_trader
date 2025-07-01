use std::{time::Duration};
use std::sync::{LazyLock, Mutex};
use crossbeam_channel::{select, tick, Receiver};
use rusqlite::{Connection, OpenFlags, Result};

use space_trader::{planet, port};
use space_trader::{server, universe};

pub static DATABASE: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(|| Mutex::new(None));

fn main() {
    println!("Space Trader");

    match setup() {
        Ok(connection) => {
            process();
        },
        Err(err) => {
            panic!("Failed to open or load database: {}", err);
        }
    }

}

fn ctrl_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = crossbeam_channel::bounded(100);
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;

    Ok(receiver)
}

fn process() {
    let chan = match ctrl_channel() {
        Ok(receiver) => receiver,
        Err(_) => { panic!("Could not set up channel for process loop.") },
    };

    let ticks = tick(Duration::from_secs(1));
    loop {
        select! {
        recv(ticks) -> _ => println!("working!"),
        recv(chan) -> _ => break,
        }
    }

    println!("\nSystem shutting down...\n");
    server::stop();
    println!("\nDone");
}

fn setup() -> Result<()> {
    let database = Connection::open_with_flags("space-trader.db", OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    planet::load_planets(&database)?;
    port::load_ports(&database)?;
    universe::load(database)?;
    server::start();
    Ok(())
}
