//! `make_draggable`'s `pointerdown` and `pointermove` handlers must call `prevent_default()`, so a mouse drag across a
//! node's label does not fall through to the browser's native text-selection gesture.
//!
//! This is checked by reading back `Event::defaultPrevented`, not by checking `window.getSelection()` after the drag. A
//! first attempt did check the selection directly, driven by real, CDP-dispatched mouse input — the same technique used
//! by `small_drag` and `overlap_resolution`. However, a control experiment showed headless Chrome never starts a text
//! selection over that exact shape to begin with. The experiment was a bare `<rect>` painted immediately before a
//! `<text>` in one `<g>`, with no listeners and no `svg-dom-graph` involved.
//!
//! `svg-dom-graph`'s own box+label markup is exactly that shape, so the selection outcome can't distinguish a working
//! `prevent_default()` from a missing one here since it would pass either way. Reading `defaultPrevented` instead
//! checks the mechanism `svg-dom-graph`'s code is actually responsible for, independent of that browser/headless-mode
//! quirk.

use crate::common::{drag, new_tab};
use std::time::Duration;

mod unit_tests;
