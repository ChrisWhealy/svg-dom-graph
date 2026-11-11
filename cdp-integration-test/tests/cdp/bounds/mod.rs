//! A real, CDP-driven mouse drag past the visible view box clamps to the edge — and the clamped node stays
//! real-hit-testable, not clipped and unclickable.
//!
//! Uses the fixture's `bounded` node (`#diagram > g.svg-dom-graph-content > g:nth-of-type(7)`), whose
//! `DragOptions::bounds` matches the diagram's own viewBox, `(0, 0, 500, 400)`. See `cdp-test-fixture/src/lib.rs`'s own
//! module doc comment for why it sits in the top-right corner, far from every other node.
//!
//! `wasm-bindgen-test`'s browser suite already proves the clamp math itself (`tests/drag/bounds.rs`, run via `wasm-pack
//! test`). It also proves that a clamped node's own listeners still respond to a synthetic pointer sequence dispatched
//! straight at it. What it cannot prove is the actual bug this feature exists to fix. A node dragged outside its own
//! `<svg>`'s visible area renders clipped. A *real* click can never hit-test it there at all. Only a real
//! `Input.dispatchMouseEvent` sequence, through the browser's own hit-testing, can catch that — see this crate's own
//! module doc comment for why `wasm-bindgen-test`'s synthetic dispatch bypasses it entirely.
//!
//! This test does both halves in one sequence. First it drags `bounded` further into the view box's own top-right
//! corner, past both the right and top edges, and releases it. Then it drags it again, back toward the centre. That
//! second drag starts from a fresh `get_midpoint()` on the node's own now-clamped, on-screen position, not any
//! coordinate computed ahead of time. The second drag can only succeed if the first drop left `bounded` somewhere real
//! hit-testing can still find.
//!
//! Both drags deliberately land clear of every other fixture node — `CollisionPolicy::PushClear`'s own corrective push
//! is proved separately, against a controlled setup, by `tests/drag/bounds.rs`'s own
//! `collision_pushback_near_an_edge_stays_within_bounds`. Landing near a node here (the bottom-right corner sits right
//! next to `branch_b`) would let that same push run unexamined, and silently change what this test is actually
//! demonstrating.

use crate::common::{drag, group_translate, new_tab};
use std::time::Duration;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bounded`'s current world-space origin: its own `<g>`'s `transform`, not its `<rect>`'s local coordinates. Those are
/// drawn once, at `(0, 0)`, and never rewritten as the node moves. See `svg_dom_graph::scene::node::draw_box`'s own doc
/// comment.
fn rect_origin(tab: &headless_chrome::Tab) -> Result<(f64, f64), String> {
    let group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(7)")
        .map_err(|e| format!("could not find bounded's <g>: {e}"))?;
    group_translate(&group)
}

mod unit_tests;
