//! A transition's bar in the Petri net panel, as the page measures it.

#[derive(serde::Deserialize)]
pub(crate) struct Bar {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}
