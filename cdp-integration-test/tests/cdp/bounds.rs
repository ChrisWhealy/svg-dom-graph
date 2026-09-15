//! A real, CDP-driven mouse drag past the visible view box clamps to the edge — and the clamped node stays
//! real-hit-testable, not clipped and unclickable.
//!
//! Uses the fixture's `bounded` node (`#diagram > g:nth-of-type(7)`), whose `DragOptions::bounds` matches the
//! diagram's own viewBox, `(0, 0, 500, 400)` — see `cdp-test-fixture/src/lib.rs`'s own module doc comment for why
//! it sits in the top-right corner, far from every other node.
//!
//! `wasm-bindgen-test`'s browser suite already proves the clamp math itself (`tests/drag/bounds.rs`, run via
//! `wasm-pack test`), including that a clamped node's own listeners still respond to a synthetic pointer sequence
//! dispatched straight at it. What it cannot prove is the actual bug this feature exists to fix: a node dragged
//! outside its own `<svg>`'s visible area renders clipped, and a *real* click can never hit-test it there at all.
//! Only a real `Input.dispatchMouseEvent` sequence, through the browser's own hit-testing, can catch that — see
//! this crate's own module doc comment for why `wasm-bindgen-test`'s synthetic dispatch bypasses it entirely.
//!
//! This test does both halves in one sequence: drag `bounded` further into the view box's own top-right corner —
//! past both the right and top edges — and release it, then, starting from a fresh `get_midpoint()` on the node's
//! own now-clamped, on-screen position (not any coordinate computed ahead of time), drag it again, back toward
//! the centre. The second drag can only succeed if the first drop left `bounded` somewhere real hit-testing can
//! still find.
//!
//! Both drags deliberately land clear of every other fixture node — `CollisionPolicy::PushClear`'s own corrective
//! push is proved separately, against a controlled setup, by `tests/drag/bounds.rs`'s own
//! `collision_pushback_near_an_edge_stays_within_bounds`. Landing near a node here (the bottom-right corner sits
//! right next to `branch_b`) would let that same push run unexamined, and silently change what this test is
//! actually demonstrating.

use crate::common::{drag, group_translate, new_tab};
use std::time::Duration;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bounded`'s current world-space origin — its own `<g>`'s `transform`, not its `<rect>`'s local coordinates (which
/// are drawn once, at `(0, 0)`, and never rewritten as the node moves — see `svg_dom_graph::scene::node::draw_box`'s
/// own doc comment).
fn rect_origin(tab: &headless_chrome::Tab) -> Result<(f64, f64), String> {
    let group = tab
        .find_element("#diagram > g:nth-of-type(7)")
        .map_err(|e| format!("could not find bounded's <g>: {e}"))?;
    group_translate(&group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_node_dragged_past_the_view_box_clamps_and_stays_real_clickable() -> Result<(), String> {
    let tab = new_tab()?;

    let rect = tab
        .find_element("#diagram > g:nth-of-type(7) rect")
        .map_err(|e| format!("could not find bounded's <rect>: {e}"))?;
    let start = rect
        .get_midpoint()
        .map_err(|e| format!("could not get bounded's midpoint: {e}"))?;

    // A real drag, well past the view box's own right edge (500) and above its own top edge (0) — `bounded`
    // starts at (400, 20), size (80, 40), so a (300, -60) move overshoots the right edge by a wide margin and the
    // top edge comfortably. Kept well inside `launch_browser`'s own generously sized window, rather than an even
    // larger delta that risks landing at some unrelated point past the window's actual edge instead of the view
    // box's.
    drag(
        &tab,
        &[
            (start.x, start.y),
            (start.x + 150.0, start.y - 30.0),
            (start.x + 300.0, start.y - 60.0),
        ],
    )?;
    std::thread::sleep(Duration::from_millis(100));

    // Clamped to the view box's own top-right corner: x = 500 - 80 = 420, y = 0.
    let (clamped_x, clamped_y) = rect_origin(&tab)?;
    let close = |got: f64, expected: f64| (got - expected).abs() <= 1.0;
    if !close(clamped_x, 420.0) || !close(clamped_y, 0.0) {
        return Err(format!(
            "expected bounded's <g> to clamp to approximately (420, 0), got ({clamped_x}, {clamped_y})"
        ));
    }

    // The crux of this test: re-find the element and ask for its own midpoint again, fresh, from wherever it is
    // now. If the first drag had left it outside the view box's own clipped rendering area, a real browser could
    // never resolve this to a paintable point at all — this call is standing in for a user's own mouse cursor
    // finding something to click.
    let clamped_rect = tab
        .find_element("#diagram > g:nth-of-type(7) rect")
        .map_err(|e| format!("could not re-find bounded's <rect> after the first drag: {e}"))?;
    let clamped_midpoint = clamped_rect
        .get_midpoint()
        .map_err(|e| format!("could not get bounded's midpoint after it was clamped to the edge: {e}"))?;

    // Drags it back toward the centre of the view box, landing clear of every other fixture node (see this
    // module's own doc comment for why that matters here) — comfortably clear of both edges too, so this second
    // drag is not itself reclamped, and its own success is unambiguous.
    let (dx, dy) = (-250.0, 150.0);
    drag(
        &tab,
        &[
            (clamped_midpoint.x, clamped_midpoint.y),
            (clamped_midpoint.x + dx / 2.0, clamped_midpoint.y + dy / 2.0),
            (clamped_midpoint.x + dx, clamped_midpoint.y + dy),
        ],
    )?;
    std::thread::sleep(Duration::from_millis(100));

    let (final_x, final_y) = rect_origin(&tab)?;
    let expected_x = clamped_x + dx;
    let expected_y = clamped_y + dy;
    if !close(final_x, expected_x) || !close(final_y, expected_y) {
        return Err(format!(
            "expected bounded's second drag to land at approximately ({expected_x}, {expected_y}), got \
             ({final_x}, {final_y}) — the node clamped to the edge by the first drag did not respond to a real, \
             separately hit-tested second drag"
        ));
    }

    Ok(())
}
