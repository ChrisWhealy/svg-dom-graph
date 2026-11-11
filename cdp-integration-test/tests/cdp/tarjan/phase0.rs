//! What the Tarjan panel's phase 0 measurement found.

use crate::tarjan::offending::Offending;

#[derive(serde::Deserialize)]
pub(crate) struct Phase0 {
    pub(crate) offending: Vec<Offending>,
    pub(crate) coincident: usize,
    pub(crate) separation: std::collections::BTreeMap<String, f64>,
}
