pub use crate::galaxy::*;

use std::collections::HashMap;

pub struct Universe {
    pub galaxies: HashMap<GalaxyId, Galaxy>,
}
