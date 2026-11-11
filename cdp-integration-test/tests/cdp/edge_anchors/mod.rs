//! `hub`'s two connectors render at the expected, distinct `EdgeAnchors(3)` fixing points. That holds both as first
//! drawn and after a real, CDP-driven drag moves `branch_a` far enough to re-snap onto a different one.
//!
//! Uses the fixture's `hub`-to-`branch_a` and `hub`-to-`branch_b` connectors (`#diagram > g.svg-dom-graph-content >
//! path:nth-of-type(3)` and `#diagram > g.svg-dom-graph-content > path:nth-of-type(4)`). See
//! `cdp-test-fixture/src/lib.rs`'s own module doc comment for this pair's place among the fixture's six nodes and four
//! connectors.
//!
//! # Expected paths, worked by hand
//!
//! Fixture positions: `hub` at `(210, 220)`, size `(80, 40)` — centre `(250, 240)`, half-extents `(40, 20)`. `branch_a`
//! at `(60, 340)`, same size — centre `(100, 360)`. `branch_b` at `(360, 340)`, same size — centre `(400, 360)`.
//!
//! From `hub` toward `branch_a`: `dx = -150`, `dy = 120`. `20 / 120 ≈ 0.167` is smaller than `40 / 150 ≈ 0.267`, so the
//! ray leaves through `hub`'s south side, crossing it at `x = 225`. `hub`'s own `EdgeAnchors(3)` offers three
//! candidates spaced a quarter of the side's width apart — `x = 230, 250, 270` — and `225` snaps to the nearest, `230`.
//!
//! From `hub` toward `branch_b`, the same comparison (magnitudes shared, `dx` sign flipped) again leaves through the
//! south side, crossing at `x = 275`, which snaps to `270`.
//!
//! `branch_a`'s own anchor, back toward `hub`, follows its own default rule (`edge_anchors: None`): the exact midpoint
//! of whichever side the ray crosses — its north side's own midpoint, `(100, 340)`. Both anchors leave vertically and
//! do not share an x coordinate, so the route jogs across their own midpoint: `mid_y = (260 + 340) / 2 = 300`. The
//! sharp route is `(230, 260) -> (230, 300) -> (100, 300) -> (100, 340)`.
//!
//! `branch_b`'s own anchor follows the same default rule: `(400, 340)`. Its route is `(270, 260) -> (270, 300) -> (400,
//! 300) -> (400, 340)`.
//!
//! `dragging_branch_a_re_snaps_its_connector_onto_a_different_fixing_point` repeats the `hub`-to-`branch_a` calculation
//! after dragging `branch_a` by `(100, 0)`. Its new centre is `(200, 360)`. From `hub` toward this new centre: `dx =
//! -50`, `dy = 120`, crossing the south side at `x ≈ 241.67`. That now snaps to the middle candidate, `250`, a
//! different fixing point from the original `230`. `branch_a`'s own anchor also moves, to `(200, 340)`. The new route
//! is `(250, 260) -> (250, 300) -> (200, 300) -> (200, 340)`. `branch_b` did not move, so its own connector's path is
//! unchanged.

use crate::common::{drag, new_tab};
use std::time::Duration;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn connector_d(tab: &headless_chrome::Tab, nth: u32) -> Result<String, String> {
    let selector = format!("#diagram > g.svg-dom-graph-content > path:nth-of-type({nth})");
    let path = tab
        .find_element(&selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?;
    path.get_attribute_value("d")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no d attribute"))
}

mod unit_tests;
