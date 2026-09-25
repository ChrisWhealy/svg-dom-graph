//! `hub`'s two connectors render at the expected, distinct `EdgeAnchors(3)` fixing points, both as first drawn and
//! after a real, CDP-driven drag moves `branch_a` far enough to re-snap onto a different one.
//!
//! Uses the fixture's `hub`-to-`branch_a` and `hub`-to-`branch_b` connectors (`#diagram > g.svg-dom-graph-content > path:nth-of-type(3)` and
//! `#diagram > g.svg-dom-graph-content > path:nth-of-type(4)`). See `cdp-test-fixture/src/lib.rs`'s own module doc comment for this pair's place
//! among the fixture's six nodes and four connectors.
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
//! -50`, `dy = 120`, crossing the south side at `x ≈ 241.67`, which now snaps to the middle candidate, `250` — a
//! different fixing point from the original `230`. `branch_a`'s own anchor also moves, to `(200, 340)`. The new route
//! is `(250, 260) -> (250, 300) -> (200, 300) -> (200, 340)`. `branch_b` did not move, so its own connector's path
//! is unchanged.

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn hubs_two_connectors_land_on_distinct_fixing_points() -> Result<(), String> {
    let tab = new_tab()?;

    let to_branch_a = connector_d(&tab, 3)?;
    let expected_to_branch_a = "M 230 260 L 230 300 L 100 300 L 100 340";
    if to_branch_a != expected_to_branch_a {
        return Err(format!(
            "expected the hub-to-branch_a connector's d to be {expected_to_branch_a:?}, got {to_branch_a:?}"
        ));
    }

    let to_branch_b = connector_d(&tab, 4)?;
    let expected_to_branch_b = "M 270 260 L 270 300 L 400 300 L 400 340";
    if to_branch_b != expected_to_branch_b {
        return Err(format!(
            "expected the hub-to-branch_b connector's d to be {expected_to_branch_b:?}, got {to_branch_b:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging `branch_a` far enough moves its `hub`-side anchor onto a different one of `hub`'s three fixing points —
/// proving the snapped candidate is recomputed live on every redraw, not fixed once at `add_edge` time.
#[test]
fn dragging_branch_a_re_snaps_its_connector_onto_a_different_fixing_point() -> Result<(), String> {
    let tab = new_tab()?;

    let branch_a_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(5) rect")
        .map_err(|e| format!("could not find branch_a's <rect>: {e}"))?;
    let midpoint = branch_a_rect
        .get_midpoint()
        .map_err(|e| format!("could not get branch_a's midpoint: {e}"))?;

    let dx = 100.0;
    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + dx / 2.0, midpoint.y),
            (midpoint.x + dx, midpoint.y),
        ],
    )?;

    std::thread::sleep(Duration::from_millis(100));

    let to_branch_a = connector_d(&tab, 3)?;
    let expected_to_branch_a = "M 250 260 L 250 300 L 200 300 L 200 340";
    if to_branch_a != expected_to_branch_a {
        return Err(format!(
            "expected the hub-to-branch_a connector's d after the drag to be {expected_to_branch_a:?}, got {to_branch_a:?}"
        ));
    }

    let to_branch_b = connector_d(&tab, 4)?;
    let expected_to_branch_b = "M 270 260 L 270 300 L 400 300 L 400 340";
    if to_branch_b != expected_to_branch_b {
        return Err(format!(
            "expected the untouched hub-to-branch_b connector's d to still be {expected_to_branch_b:?}, got {to_branch_b:?}"
        ));
    }

    Ok(())
}
