//! An elbowed connector's real rendered `<path>` matches its hand-worked route, both as first drawn and after a
//! real, CDP-driven drag moves one of its endpoints.
//!
//! Uses the fixture's two `solo`-to-`blocker` connectors. The first (`#diagram > path:nth-of-type(1)`) has sharp
//! corners. The second (`#diagram > path:nth-of-type(2)`) has `corner_radius: 8.0`. See
//! `cdp-test-fixture/src/lib.rs`'s own module doc comment for why this pair, out of the fixture's three nodes, is
//! the one that actually bends.
//!
//! # Expected path, worked by hand
//!
//! Fixture positions: `solo` at `(20, 20)`, size `(80, 40)` — centre `(60, 40)`. `blocker` at `(300, 150)`, same
//! size — centre `(340, 170)`.
//!
//! From `solo` toward `blocker`'s centre: `dx = 280`, `dy = 130`. Scaled by each axis's half-extent, `40 / 280 ≈
//! 0.143` is smaller than `20 / 130 ≈ 0.154`, so `solo` anchors on its east side: `(100, 40)`.
//!
//! From `blocker` toward `solo`'s centre, the same comparison (magnitudes are shared, only the sign flips) anchors
//! `blocker` on its west side: `(300, 170)`.
//!
//! Both anchors leave horizontally and do not share a y coordinate, so the route jogs across the midpoint between
//! them: `mid_x = (100 + 300) / 2 = 200`. The sharp route is `(100, 40) → (200, 40) → (200, 170) → (300, 170)`.
//!
//! Rounding each corner to radius `8.0` shrinks neither corner. Both adjacent segments (100 and 130 units) are far
//! longer than `16.0`, so the full radius applies at both corners.
//!
//! `dragging_solo_reroutes_both_connectors_and_keeps_their_own_corner_radius` repeats this same calculation after
//! dragging `solo` by `(40, 25)`. Its new centre is `(100, 65)`. Only `solo`'s own anchor and the jog's `mid_x`
//! change — `blocker` did not move, so its own anchor `(300, 170)` stays exactly as computed above.

use crate::common::{drag, new_tab};
use std::time::Duration;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn connector_d(tab: &headless_chrome::Tab, nth: u32) -> Result<String, String> {
    let selector = format!("#diagram > path:nth-of-type({nth})");
    let path = tab
        .find_element(&selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?;
    path.get_attribute_value("d")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no d attribute"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_two_connectors_render_the_expected_sharp_and_rounded_elbow_routes() -> Result<(), String> {
    let tab = new_tab()?;

    let sharp = connector_d(&tab, 1)?;
    let expected_sharp = "M 100 40 L 200 40 L 200 170 L 300 170";
    if sharp != expected_sharp {
        return Err(format!(
            "expected the sharp connector's d to be {expected_sharp:?}, got {sharp:?}"
        ));
    }

    let rounded = connector_d(&tab, 2)?;
    let expected_rounded = "M 100 40 L 192 40 A 8 8 0 0 1 200 48 L 200 162 A 8 8 0 0 0 208 170 L 300 170";
    if rounded != expected_rounded {
        return Err(format!(
            "expected the rounded connector's d to be {expected_rounded:?}, got {rounded:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector's rendered `<path>` must never fill. An elbow route bends back on itself, and the SVG default fill
/// is black. An unset `fill` would paint a solid black wedge across every corner.
#[test]
fn a_connector_path_does_not_fill_and_carries_the_arrow_marker() -> Result<(), String> {
    let tab = new_tab()?;

    let selector = "#diagram > path:nth-of-type(1)";
    let path = tab
        .find_element(selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?;

    let fill = path
        .get_attribute_value("fill")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no fill attribute"))?;
    if fill != "none" {
        return Err(format!("expected {selector}'s fill to be \"none\", got {fill:?}"));
    }

    let marker_end = path
        .get_attribute_value("marker-end")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no marker-end attribute"))?;
    if !marker_end.starts_with("url(#") {
        return Err(format!(
            "expected {selector}'s marker-end to reference a marker, got {marker_end:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging `solo` reroutes both connectors' `solo`-end. The rounded connector stays rounded afterwards — proving
/// `corner_radius` survives a redraw, not just the initial draw.
#[test]
fn dragging_solo_reroutes_both_connectors_and_keeps_their_own_corner_radius() -> Result<(), String> {
    let tab = new_tab()?;

    let solo_rect = tab
        .find_element("#diagram > g:nth-of-type(1) rect")
        .map_err(|e| format!("could not find solo's <rect>: {e}"))?;
    let midpoint = solo_rect
        .get_midpoint()
        .map_err(|e| format!("could not get solo's midpoint: {e}"))?;

    let (dx, dy) = (40.0, 25.0);
    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + dx / 2.0, midpoint.y + dy / 2.0),
            (midpoint.x + dx, midpoint.y + dy),
        ],
    )?;

    std::thread::sleep(Duration::from_millis(100));

    let sharp = connector_d(&tab, 1)?;
    let expected_sharp = "M 140 65 L 220 65 L 220 170 L 300 170";
    if sharp != expected_sharp {
        return Err(format!(
            "expected the sharp connector's d after the drag to be {expected_sharp:?}, got {sharp:?}"
        ));
    }

    let rounded = connector_d(&tab, 2)?;
    let expected_rounded = "M 140 65 L 212 65 A 8 8 0 0 1 220 73 L 220 162 A 8 8 0 0 0 228 170 L 300 170";
    if rounded != expected_rounded {
        return Err(format!(
            "expected the rounded connector's d after the drag to be {expected_rounded:?}, got {rounded:?}"
        ));
    }

    Ok(())
}
