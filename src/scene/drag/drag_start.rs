use crate::geometry::view::ViewTransform;
use svg_dom::root::utils::{Matrix2D, Point, Size};

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
    /// The dragged box's own size, read from the same `Rect` `box_origin` came from, at drag start.
    ///
    /// A node's size never changes while it is being dragged, so this is read once here rather than re-fetched as this
    /// would incur another graph lookup and `RefCell` borrow on every bounded `pointermove` / collision-correcting
    /// `pointerup` event.
    pub(super) box_size: Size,
    /// The dragged group's screen CTM, inverted once at pointerdown and reused for the duration of this drag.
    ///
    /// `SvgNode::screen_ctm()` may force a synchronous layout, so this is captured once per drag rather than on every
    /// pointermove. It reflects the scene's view *as it was at pointerdown* — see [`view`](Self::view) — and the dragged
    /// group's own translation at that moment, which is why it is never re-read: that translation changes on every move.
    ///
    /// The view can change during a drag, so this matrix alone is not enough to place the pointer once it has.
    pub(super) inverse_ctm: Matrix2D,
    /// The scene's view — its zoom and pan — that [`inverse_ctm`](Self::inverse_ctm) was taken under.
    ///
    /// A drag cannot assume the view stays as it was. The wheel, the keyboard, a toolbar button, or the application
    /// itself can all change it mid-drag. Each pointermove compares this with the view as it is now, and re-reads the
    /// pointer's content position under that, so the node goes on following the pointer.
    pub(super) view: ViewTransform,
}
