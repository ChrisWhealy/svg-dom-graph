//! A connector of the Tarjan panel that passes through a vertex.

#[derive(serde::Deserialize)]
pub(crate) struct Offending {
    pub(crate) e: usize,
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) hit: Vec<usize>,
    pub(crate) kind: String,
}
