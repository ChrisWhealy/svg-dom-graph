use super::ToolbarAction;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One rendered toolbar button.
pub(crate) struct ToolbarButton {
    pub action: ToolbarAction,
    pub group: SvgNode,
    pub rect: SvgNode,
    pub label: SvgNode,
}
