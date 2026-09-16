use svg_dom::root::utils::{Matrix2D, Point};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer position and box origin recorded when a drag starts.
///
/// The delta between the pointer's current position and `pointer` describes how far to move `box_origin`. Both are in
/// the dragged box's own user-space coordinates, not viewport CSS pixels — see `inverse_ctm`.
#[derive(Clone, Copy)]
pub(super) struct DragStart {
    /// The pointer that started this drag.
    ///
    /// A pointer's own `pointerdown` grants it exclusive capture (see `set_pointer_capture` below), but a `pointermove`
    /// `pointerup` or `pointercancel` for a *different*, unrelated pointer can still reach this same listener. For
    /// example a second finger touching the same element mid-drag.
    ///
    /// Checking this field against each event's own id avoids the case in which a different pointer attempts to drive
    /// or end another pointer's drag event.
    pub(super) pointer_id: i32,
    pub(super) pointer: Point,
    pub(super) box_origin: Point,
    /// The dragged group's screen CTM, inverted once at pointerdown and reused for the duration of this drag event.
    ///
    /// `SvgNode::screen_ctm()` may force a synchronous layout, so this is captured once per drag rather than on every
    /// pointermove. Caching it here (rather than recomputing it per drag event) assumes that neither the group's own
    /// transform nor any ancestor transform up to the viewport, changes mid-drag.
    ///
    /// The inverse CTM is captured at drag start so all pointer deltas remain expressed in a stable drag-start
    /// coordinate system. Ancestor transforms must remain unchanged during the drag; the node group's translation
    /// itself is expected to change.
    pub(super) inverse_ctm: Matrix2D,
}
