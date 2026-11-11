//! An elbowed connector's real rendered `<path>` matches its hand-worked route, both as first drawn and after a real,
//! CDP-driven drag moves one of its endpoints.
//!
//! Uses the fixture's two `solo`-to-`blocker` connectors. The first (`#diagram > g.svg-dom-graph-content >
//! path:nth-of-type(1)`) has sharp corners. The second (`#diagram > g.svg-dom-graph-content > path:nth-of-type(2)`) has
//! `corner_radius: 8.0`. See `cdp-test-fixture/src/lib.rs`'s own module doc comment for why this pair, out of the
//! fixture's three nodes, is the one that actually bends.
//!
//! # Expected path, worked by hand
//!
//! Fixture positions: `solo` at `(20, 20)`, size `(80, 40)` — centre `(60, 40)`. `blocker` at `(300, 150)`, same size —
//! centre `(340, 170)`.
//!
//! From `solo` toward `blocker`'s centre: `dx = 280`, `dy = 130`. Scaled by each axis's half-extent, `40 / 280 ≈ 0.143`
//! is smaller than `20 / 130 ≈ 0.154`, so `solo` anchors on its east side: `(100, 40)`.
//!
//! From `blocker` toward `solo`'s centre, the same comparison (magnitudes are shared, only the sign flips) anchors
//! `blocker` on its west side: `(300, 170)`.
//!
//! Both anchors leave horizontally and do not share a y coordinate, so the route jogs across the midpoint between them:
//! `mid_x = (100 + 300) / 2 = 200`. The sharp route is `(100, 40) → (200, 40) → (200, 170) → (300, 170)`.
//!
//! Rounding each corner to radius `8.0` shrinks neither corner. Both adjacent segments (100 and 130 units) are far
//! longer than `16.0`, so the full radius applies at both corners.
//!
//! `dragging_solo_reroutes_both_connectors_and_keeps_their_own_corner_radius` repeats this same calculation after
//! dragging `solo` by `(40, 25)`. Its new centre is `(100, 65)`. Only `solo`'s own anchor and the jog's `mid_x` change
//! — `blocker` did not move, so its own anchor `(300, 170)` stays exactly as computed above.

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
