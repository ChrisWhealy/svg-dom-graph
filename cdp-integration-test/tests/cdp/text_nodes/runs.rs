//! The runs of text that the Markdown node draws as bold, italic and code, and the size of each line.

#[derive(serde::Deserialize)]
pub(crate) struct Runs {
    pub(crate) bold: Vec<String>,
    pub(crate) italic: Vec<String>,
    pub(crate) code: Vec<String>,
    pub(crate) sizes: Vec<(String, String)>,
}
