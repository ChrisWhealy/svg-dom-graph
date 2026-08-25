//! Dropping a node onto another node pushes it back to the expected clear position, via real, CDP-driven mouse
//! input.
//!
//! Drags the fixture's `mover` node (`#diagram > g:nth-of-type(3)`) onto `blocker` (`#diagram > g:nth-of-type(2)`, not
//! draggable, fixed in place), then checks that `mover`'s final position is the one the documented overlap-resolution
//! rule predicted: pushed back along a straight line from its own pre-drag centre through `blocker`'s centre, stopping
//! just outside `blocker`'s boundary (inflated by half of `mover`'s own size, so `mover`'s rectangle — not just its
//! centre — clears the overlap) plus a small padding gap.
//!
//! # Expected position, worked by hand
//!
//! Fixture positions (`cdp-test-fixture/src/lib.rs`): `mover` starts at `(20, 150)`, size `(80, 40)` — centre
//! `(60, 170)`. `blocker` sits at `(300, 150)`, same size — centre `(340, 170)`.
//!
//! `blocker` inflated by half of `mover`'s size on every side: origin `(260, 130)`, size `(160, 80)` — spans
//! `x: [260, 420]`, `y: [130, 210]`, same centre `(340, 170)`.
//!
//! The approach line from `mover`'s pre-drag centre `(60, 170)` through `blocker`'s centre `(340, 170)` is purely
//! horizontal (`dy = 0`), so it crosses the inflated rectangle's boundary at its left edge: `x = 260`, `y = 170`.
//!
//! Padding (`6.0`, matching `scene::OVERLAP_RESOLUTION_PADDING`, which is private) pushes that another 6 units further
//! from `blocker`'s centre, along the same leftward direction: `(254, 170)`.
//!
//! `mover`'s final origin is that centre minus half its own size: `(254 - 40, 170 - 20) = (214, 150)`.

use crate::common::{drag, new_tab};
use std::time::Duration;

#[test]
fn dropping_mover_onto_blocker_lands_at_the_expected_clear_position() -> Result<(), String> {
    let tab = new_tab()?;

    let mover_rect = tab
        .find_element("#diagram > g:nth-of-type(3) rect")
        .map_err(|e| format!("could not find mover's <rect>: {e}"))?;
    let mover_midpoint = mover_rect
        .get_midpoint()
        .map_err(|e| format!("could not get mover's midpoint: {e}"))?;

    let blocker_rect = tab
        .find_element("#diagram > g:nth-of-type(2) rect")
        .map_err(|e| format!("could not find blocker's <rect>: {e}"))?;
    let blocker_midpoint = blocker_rect
        .get_midpoint()
        .map_err(|e| format!("could not get blocker's midpoint: {e}"))?;

    // Drags mover's centre directly onto blocker's centre — a real, multi-step drag, not a single instantaneous
    // jump, so it exercises the same pointermove sequence a real user's mouse would produce.
    let steps = 4;
    let mut waypoints = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        waypoints.push((
            mover_midpoint.x + (blocker_midpoint.x - mover_midpoint.x) * t,
            mover_midpoint.y + (blocker_midpoint.y - mover_midpoint.y) * t,
        ));
    }
    drag(&tab, &waypoints)?;

    // Give the page a moment to process the dispatched events, including the corrective move_node call
    // resolve_overlap triggers on pointerup, before reading the result back.
    std::thread::sleep(Duration::from_millis(100));

    let mover_rect = tab
        .find_element("#diagram > g:nth-of-type(3) rect")
        .map_err(|e| format!("could not re-find mover's <rect> after the drag: {e}"))?;
    let after_x: f64 = mover_rect
        .get_attribute_value("x")
        .map_err(|e| format!("{e}"))?
        .ok_or("mover's <rect> has no x attribute after the drag")?
        .parse()
        .map_err(|e| format!("x did not parse as f64: {e}"))?;
    let after_y: f64 = mover_rect
        .get_attribute_value("y")
        .map_err(|e| format!("{e}"))?
        .ok_or("mover's <rect> has no y attribute after the drag")?
        .parse()
        .map_err(|e| format!("y did not parse as f64: {e}"))?;

    let (expected_x, expected_y) = (214.0, 150.0);
    let close = |got: f64, expected: f64| (got - expected).abs() <= 1.0;

    if !close(after_x, expected_x) || !close(after_y, expected_y) {
        return Err(format!(
            "expected mover's <rect> to land at approximately ({expected_x}, {expected_y}), got ({after_x}, {after_y})"
        ));
    }

    let blocker_rect = tab
        .find_element("#diagram > g:nth-of-type(2) rect")
        .map_err(|e| format!("could not re-find blocker's <rect>: {e}"))?;
    let blocker_x: f64 = blocker_rect
        .get_attribute_value("x")
        .map_err(|e| format!("{e}"))?
        .ok_or("blocker's <rect> has no x attribute")?
        .parse()
        .map_err(|e| format!("x did not parse as f64: {e}"))?;
    let blocker_y: f64 = blocker_rect
        .get_attribute_value("y")
        .map_err(|e| format!("{e}"))?
        .ok_or("blocker's <rect> has no y attribute")?
        .parse()
        .map_err(|e| format!("y did not parse as f64: {e}"))?;
    // mover: (80, 40) at (after_x, after_y); blocker: (80, 40) at (blocker_x, blocker_y).
    let overlaps = after_x < blocker_x + 80.0
        && after_x + 80.0 > blocker_x
        && after_y < blocker_y + 40.0
        && after_y + 40.0 > blocker_y;
    if overlaps {
        return Err(format!(
            "mover ({after_x}, {after_y}) still overlaps blocker ({blocker_x}, {blocker_y}) after the drop"
        ));
    }

    Ok(())
}
