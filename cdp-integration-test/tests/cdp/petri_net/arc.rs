//! An arc of the Petri net panel, as the page measures it.

#[derive(serde::Deserialize)]
pub(crate) struct Arc {
    pub(crate) i: usize,
    pub(crate) n: Vec<f64>,
}
