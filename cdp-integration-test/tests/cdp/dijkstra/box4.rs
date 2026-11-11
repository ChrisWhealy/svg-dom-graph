//! A rectangle as the Dijkstra panel's page reports it.

use serde::Deserialize;

/// A rectangle as the page reports it.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Box4 {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl Box4 {
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.left < other.left + other.width
            && self.left + self.width > other.left
            && self.top < other.top + other.height
            && self.top + self.height > other.top
    }

    pub(crate) fn centre(self) -> (f64, f64) {
        (self.left + self.width / 2.0, self.top + self.height / 2.0)
    }
}
