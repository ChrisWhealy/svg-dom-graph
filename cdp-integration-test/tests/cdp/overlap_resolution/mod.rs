//! Dropping a node onto another node pushes it back to the expected clear position, via real, CDP-driven mouse input.
//!
//! Drags the fixture's `mover` node (`#diagram > g.svg-dom-graph-content > g:nth-of-type(3)`) onto `blocker`
//! (`#diagram > g.svg-dom-graph-content > g:nth-of-type(2)`, not draggable, fixed in place). It then checks that
//! `mover`'s final position is the one the documented overlap-resolution rule predicted. That rule pushes it back along a
//! straight line from its own pre-drag centre through `blocker`'s centre. It stops just outside `blocker`'s boundary,
//! plus a small padding gap. The boundary is inflated by half of `mover`'s own size, so `mover`'s rectangle, not just its
//! centre, clears the overlap.
//!
//! # Expected position, worked by hand
//!
//! Fixture positions (`cdp-test-fixture/src/lib.rs`): `mover` starts at `(20, 150)`, size `(80, 40)` — centre `(60,
//! 170)`. `blocker` sits at `(300, 150)`, same size — centre `(340, 170)`.
//!
//! `blocker` inflated by half of `mover`'s size on every side: origin `(260, 130)`, size `(160, 80)` — spans `x: [260,
//! 420]`, `y: [130, 210]`, same centre `(340, 170)`.
//!
//! The approach line from `mover`'s pre-drag centre `(60, 170)` through `blocker`'s centre `(340, 170)` is purely
//! horizontal (`dy = 0`), so it crosses the inflated rectangle's boundary at its left edge: `x = 260`, `y = 170`.
//!
//! Padding (`6.0`, matching `scene::OVERLAP_RESOLUTION_PADDING`, which is private) pushes that another 6 units further
//! from `blocker`'s centre, along the same leftward direction: `(254, 170)`.
//!
//! `mover`'s final origin is that centre minus half its own size: `(254 - 40, 170 - 20) = (214, 150)`.

use crate::common::{drag, group_translate, new_tab};
use std::time::Duration;

mod unit_tests;
