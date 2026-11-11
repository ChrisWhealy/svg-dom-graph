//! Everything that the Petri net panel's measurement script reads from the page.

use crate::petri_net::{arc::Arc, bar::Bar, hub::Hub};

#[derive(serde::Deserialize)]
pub(crate) struct Measured {
    pub(crate) bars: std::collections::HashMap<String, Bar>,
    pub(crate) hubs: std::collections::HashMap<String, Hub>,
    pub(crate) arcs: Vec<Arc>,
}
