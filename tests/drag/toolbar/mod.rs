//! `Scene`'s toolbar and zoom: showing, hiding, and moving the button bar, keeping it a fixed size while the
//! content zooms, the zoom buttons themselves (by click and by keyboard), and dragging under zoom.
//!
//! Every test uses a viewport and `viewBox` of `400 x 300`, 1:1, so client pixels and user-space units coincide,
//! unless a scenario's own module doc comment says otherwise.
//!
//! - [`basics`] — showing/hiding the bar, sibling ordering, edge placement, buttons by click and by keyboard,
//!   disabled state at the zoom limits, and option validation.
//! - [`gestures`] — pan and wheel-zoom mechanics: dragging a node under zoom, the pan surface, following/
//!   releasing/cancelling a pan, ctrl/cmd-plus-wheel, wheel direction and amount, coalescing a burst into one
//!   frame, and a toolbar button winning over a pending frame.
//! - [`input_modes`] — pan and wheel zoom independent of the toolbar via `InputMode`: `WithToolbar`, `On`, `Off`,
//!   and forcing/switching combinations of the two.
//! - [`responsive_layout`] — the toolbar and pan surface laid out in the `<svg>`'s own user space, an
//!   attribute-less CSS-sized `<svg>`, and `refresh_layout`.
//! - [`viewbox_geometry`] — non-zero `viewBox` origins, `meet`/`slice` aspect-ratio mismatches, and CSS scaling.
//! - [`accessibility`] — the toolbar buttons' own role/name/tab-stop, the keyboard focus target and its pan/zoom
//!   keys, and quiet, non-disruptive state updates.
//! - [`performance`] — zoom and pan write only the content layer's own `transform`, at most once per frame, with
//!   every node position and connector path left untouched.
//! - [`lifecycle`] — showing and hiding the toolbar, and toggling input modes, repeatedly, install no duplicate
//!   handlers; a dropped `Scene` answers no input at all.
//! - [`mid_gesture_changes`] — a node drag or a pan that carries on correctly through a wheel zoom, an
//!   application-initiated zoom, or a toolbar button pressed while the gesture is still in progress.
//! - [`frame_teardown`] — the toolbar hidden, a mode switched, or the `Scene` dropped, between an input and the
//!   animation frame that would have drawn it.
//! - [`host_svg_attributes`] — the application's own `<svg>` may carry its own accessibility attributes, and the
//!   scene must never touch them.
//! - [`per_frame_reads`] — a pan or zoom frame writes the view without reading any attribute back from the DOM.
//! - [`failure_injection`] — a view change is a transaction: `Element.setAttribute` failures, injected on purpose,
//!   must never leave `zoom_scale()` disagreeing with what is actually drawn.

mod accessibility;
mod basics;
mod failure_injection;
mod frame_teardown;
mod gestures;
mod host_svg_attributes;
mod input_modes;
mod lifecycle;
mod mid_gesture_changes;
mod per_frame_reads;
mod performance;
mod responsive_layout;
mod support;
mod viewbox_geometry;
