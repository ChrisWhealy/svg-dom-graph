use super::ToolbarAction;
use std::cell::Cell;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One rendered toolbar button.
pub(crate) struct ToolbarButton {
    pub action: ToolbarAction,
    pub group: SvgNode,
    pub rect: SvgNode,
    pub label: SvgNode,
    /// Whether the button was last drawn enabled, or `None` if it has not been drawn yet.
    ///
    /// Remembered here, on the Rust side, so a frame that leaves it as it was can see so without asking the DOM. Reading
    /// an attribute back crosses the WASM and JavaScript boundary and allocates a `String` for the answer, which is too
    /// much to pay on every animation frame of a pan to learn that nothing changed.
    pub enabled: Cell<Option<bool>>,
}
