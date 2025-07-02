use std::{time::Duration};
use std::sync::atomic::Ordering;
use crossbeam_channel::{select, tick, Receiver};
use rusqlite::{Connection, OpenFlags};

use space_trader::{galaxy, message, planet, port, sector, server, user};

fn main() {
    println!("Space Trader");

    match setup() {
        Ok(connection) => process(),
        Err(err) => panic!("Failed to open or load database: {}", err),
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
        recv(ticks) -> _ =>
            if !server::IS_ACTIVE.load(Ordering::SeqCst) {
                break;
            },
        recv(chan) -> _ => break,
        }
    }

    println!("\nSystem shutting down...\n");
    server::stop();
    // TODO shut down anything else which might still be spinning
    println!("\nDone");
}

fn setup() -> Result<(), String> {
    let database = match Connection::open_with_flags("space-trader.db", OpenFlags::SQLITE_OPEN_READ_WRITE) {
        Ok(database) => database,
        Err(err) => return Err(err.to_string()),
    };

    // Order might matter here, so don't change it.
    user::load_users(&database)?;
    message::load_messages(&database)?;
    planet::load_planets(&database)?;
    port::load_ports(&database)?;
    sector::load_sectors(&database)?;
    galaxy::load_galaxies(&database)?;

    server::start();
    Ok(())
}
