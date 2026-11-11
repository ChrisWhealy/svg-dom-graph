//! A place of the Petri net panel, as the page measures it.

#[derive(serde::Deserialize)]
pub(crate) struct Hub {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) r: f64,
}
