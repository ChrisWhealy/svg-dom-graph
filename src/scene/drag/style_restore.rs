use super::{DragState, GRAB_STYLE};
use web_sys::Element;

/// Puts a node's idle style back if the scene is dropped in the middle of a drag.
///
/// A drag sets the grabbing cursor on the node, and the pointer-up or pointer-cancel handler is what sets it back. If the last
/// `Scene` handle goes between the two, those handlers go with it and nothing is left to do that. The node's `<g>` stays in the
/// document for as long as the host does, so it would keep showing a grabbing cursor for a drag that has ended.
///
/// The pointer-down handler owns one of these, so it is dropped when the node's listeners are released. It holds the DOM
/// element itself and not the node, because the node is already gone by the time the handler is dropped. The element is a
/// browser object that nothing in Rust owns, so holding it makes no ownership cycle.
pub(super) struct DragStyleRestore {
    element: Element,
    drag_start: DragState,
}

impl DragStyleRestore {
    pub(super) fn new(element: Element, drag_start: DragState) -> Self {
        Self { element, drag_start }
    }
}

impl Drop for DragStyleRestore {
    fn drop(&mut self) {
        // No drag in progress means the pointer-up or pointer-cancel handler has already put the style back.
        let Some(start) = self.drag_start.get() else { return };
        // Nowhere to report an error to, and nothing left to keep consistent.
        let _ = self.element.release_pointer_capture(start.pointer_id);
        let _ = self.element.set_attribute("style", GRAB_STYLE);
    }
}
