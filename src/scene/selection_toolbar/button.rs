use super::SelectionToolbarAction;
use std::cell::Cell;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One rendered selection toolbar button.
pub(crate) struct SelectionToolbarButton {
    pub action: SelectionToolbarAction,
    pub group: SvgNode,
    pub rect: SvgNode,
    pub label: SvgNode,
    /// The button's own width: its natural width, or wider if its label's rendered text needs more room. The label of a
    /// stride button is the caller's own text, in any script, so no fixed width is certain to hold it.
    pub width: f64,
    /// Whether the button was last drawn enabled, or `None` if it has not been drawn yet.
    ///
    /// Same reasoning as [`crate::scene::toolbar`]'s own `ToolbarButton::enabled`: remembered here so a
    /// `Scene::set_selection` call that leaves every button as it was writes nothing to the DOM.
    pub enabled: Cell<Option<bool>>,
}
