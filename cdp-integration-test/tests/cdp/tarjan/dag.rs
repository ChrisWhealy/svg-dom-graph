//! The condensation of the Tarjan panel, as the page draws it.

#[derive(serde::Deserialize)]
pub(crate) struct Dag {
    pub(crate) nodes: Vec<Vec<f64>>,
    pub(crate) arcs: Vec<(usize, usize)>,
    pub(crate) state: Option<String>,
}

impl Dag {
    /// How much the lengths of the arrows differ from each other, as a fraction of their mean.
    pub(crate) fn spread(&self) -> f64 {
        let lengths: Vec<f64> = self
            .arcs
            .iter()
            .map(|&(a, b)| {
                (self.nodes[a - 1][0] - self.nodes[b - 1][0]).hypot(self.nodes[a - 1][1] - self.nodes[b - 1][1])
            })
            .collect();
        let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
        (lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lengths.len() as f64).sqrt() / mean
    }
}
