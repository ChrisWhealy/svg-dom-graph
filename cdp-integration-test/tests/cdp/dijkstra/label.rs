//! One edge label of the Dijkstra panel, as the page draws it.

use crate::dijkstra::box4::Box4;
use serde::Deserialize;

/// One edge label as the page draws it: its text, its box, and its index.
#[derive(Deserialize, Debug)]
pub(crate) struct Label {
    pub(crate) index: usize,
    pub(crate) text: String,
    #[serde(flatten)]
    pub(crate) area: Box4,
    pub(crate) name: String,
}
