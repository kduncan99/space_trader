#[derive(Eq, Hash, PartialEq)]
pub enum ActionResolution {
    Fine,   // sub-second
    Coarse, // approximately 1Hz
    Daily,  // acts as soon after midnight as is practical
}

pub trait Actor {
    fn act(&self);
    fn is_finished(&self) -> bool;
}
