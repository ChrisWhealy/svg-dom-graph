use crate::model::content::DataNodeContent;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's semantic content holds one of:
///  - the plain text label of an ordinary node,
///  - the typed numeric values of a data node,
///  - the label of a container node that owns a nested `Scene`.
///
/// The graph model retains this regardless of how a node was rendered. So a reader need not parse the generated SVG to
/// recover the node's actual content.
///
/// `Container` holds only the label, never the nested `Scene` itself: a `Scene`/`SceneInner` is DOM/wasm state, and
/// this module is kept free of that (see [`super::super::graph::Graph`]'s own doc comment) so it stays testable with a
/// plain `cargo test`. The nested `Scene` handle instead lives on `BoxHandles::child`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeContent {
    Label(String),
    Data(DataNodeContent),
    Container(String),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl From<String> for NodeContent {
    fn from(label: String) -> Self {
        Self::Label(label)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl From<&str> for NodeContent {
    fn from(label: &str) -> Self {
        Self::Label(label.to_string())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl From<DataNodeContent> for NodeContent {
    fn from(data: DataNodeContent) -> Self {
        Self::Data(data)
    }
}
