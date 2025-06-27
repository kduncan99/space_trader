use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use crate::action::ActionResolution;

pub struct Clock {
    registry: Mutex<HashMap<ActionResolution, Vec<Box<dyn Any>>>>,   
    join_handle: Option<JoinHandle<()>>,
    stop_flag: bool,
}

impl Clock {

    pub fn new() -> Clock {
        Clock{
            registry: Mutex::new(HashMap::new()),
            join_handle: None,
            stop_flag: false,
        }
    }

    pub fn register(&mut self, resolution: ActionResolution, actor: Box<dyn Any>) {
        let reg = self.registry.lock().unwrap().get_mut(&resolution).unwrap().push(actor);
    }

    pub fn start(&mut self) {
        if self.join_handle.is_some() {
            panic!("Clock already started!");
        }

        self.join_handle = Some(thread::spawn(Self::thread_func));
        while !self.stop_flag {
            sleep(Duration::from_secs(5));
        }
    }

    pub fn stop(&mut self) {
        let jh = self.join_handle.take();
        if !jh.is_some() {
            panic!("Clock already stopped or never started!");
        }

        self.stop_flag = true;
        jh.unwrap().join();
    }

    fn thread_func() {
        
    }
}
