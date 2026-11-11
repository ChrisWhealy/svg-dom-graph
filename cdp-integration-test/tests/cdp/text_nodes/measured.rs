//! What the Markdown node's measurement script reads from the page: the box, and where each line of text is inside it.

#[derive(serde::Deserialize)]
pub(crate) struct Measured {
    pub(crate) r#box: (f64, f64),
    pub(crate) lines: Vec<(String, f64, f64, f64, f64)>,
}
