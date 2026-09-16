use crate::model::content::DataNodeContent;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's semantic content holding either the plain text label of an ordinary node or the typed numeric values of a
/// data node.
///
/// The graph model retains this regardless of how a node was rendered. So a reader need not parse the generated
/// SVG to recover the node's actual content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeContent {
    Label(String),
    Data(DataNodeContent),
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
