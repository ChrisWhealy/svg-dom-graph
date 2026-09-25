//! WASM fixture for `cdp-integration-test`'s Chrome-DevTools-Protocol integration tests.
//!
//! Builds a small `Scene` with known, fixed node positions, so the driving CDP tests can compute expected drag results
//! independently and compare them against real, browser-driven mouse input. This is something `wasm-bindgen-test`'s
//! synthetic `dispatchEvent` calls cannot exercise, since they go straight to a target element rather than through the
//! browser's own hit-testing, pointer capture, and default-action machinery.
//!
//! `index.html`'s `<svg id="diagram">` uses a `viewBox` matching its pixel size 1:1, so a CSS-pixel mouse delta is the
//! same size in this scene's user-space units. This removes the need for the tests to care about scaling.
//!
//! Nodes, in add order (`#diagram > g.svg-dom-graph-content > g:nth-of-type(N)`):
//!
//! 1. `solo` — draggable, far from every other node. Used to prove an ordinary drag with no overlap involved.
//! 2. `blocker` — not draggable, fixed in place. The node `mover` is dragged onto.
//! 3. `mover` — draggable, starts far from `blocker`. Used to prove overlap resolution on drop.
//! 4. `hub` — not draggable, fixed in place. Its own `EdgeAnchors(3)` offers three fixing points along each side.
//! 5. `branch_a` — draggable, connected to `hub`. Used to prove a live drag re-snaps to a different fixing point.
//! 6. `branch_b` — not draggable, connected to `hub`. Stays put, so its own connector isolates `branch_a`'s drag as the
//!    only thing that moved.
//! 7. `bounded` — draggable with `DragOptions::bounds` set to the diagram's own viewBox, `(0, 0, 500, 400)`. Placed in
//!    the top-right corner, far from every other node, so a drag past the viewBox's own right/bottom edge has nothing
//!    else to collide with on the way. Used to prove a real mouse drag past the visible area clamps to the edge, and
//!    that the clamped node stays real-hit-testable — see `bounds.rs`.
//! 8. `named_value` — not draggable, a named single-value data node (`"B: u64 = AB CD EF 01 23 45 67 89"`). Used by
//!    `accessibility_tree.rs` to query the real, browser-computed accessibility tree via CDP's own `Accessibility`
//!    domain, not just the rendered DOM `wasm-pack test`'s own suite already checks.
//!
//! Panning and wheel zoom are switched on (`InputMode::On`), with no toolbar, so dragging empty background pans the whole
//! scene and Ctrl plus the wheel zooms it. The
//! background is clear of every node and connector at, for example, `(450, 200)` and `(150, 90)`.
//!
//! Connectors, in add order (`#diagram > g.svg-dom-graph-content > path:nth-of-type(N)`):
//!
//! 1. `solo` to `blocker`, sharp corners (`Scene::add_edge`'s default). `solo` and `blocker` sit at a diagonal offset,
//!    so this connector bends — see `connectors.rs` for the hand-worked path.
//! 2. `solo` to `blocker` again, rounded corners (`Scene::add_edge_with`, `corner_radius: 8.0`). Same route as
//!    connector 1, so the two isolate corner rounding as the only difference between them.
//! 3. `hub` to `branch_a`, sharp corners. `hub`'s own `EdgeAnchors(3)` snaps this connector's `hub`-end onto one of
//!    three fixing points along `hub`'s south side — see `edge_anchors.rs` for the hand-worked path.
//! 4. `hub` to `branch_b`, sharp corners. Shares `hub`'s same three fixing points, but lands on a different one —
//!    proving sibling edges sharing one node do not all crowd onto the same spot.
//!
//! `Scene` is a cheap handle around an `Rc`-shared state, and its own listener closures deliberately hold only `Weak`
//! references back to it (so a dropped `Scene` cannot leak the whole page's DOM forever). That means a `Scene` built
//! and then simply let go out of scope, which is the natural shape of a `#[wasm_bindgen(start)]` function, drops before
//! the user ever gets a chance to click anything. This in turn, silently kills every listener with no panic and no
//! console output.
//!
//! `SCENE` below keeps this fixture's only `Scene` handle alive for the page's whole lifetime, the same pattern
//! `demo-app` now uses.

use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    Error,
    scene::{
        ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, DragOptions, EdgeAnchors, InputMode, NodeOptions,
        NodeValues, Scene,
    },
};
use wasm_bindgen::prelude::*;

thread_local! {
    // Keeps this fixture's `Scene` alive for as long as the page lives — see the module doc comment above.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

fn build() -> Result<(), Error> {
    let svg = SvgRoot::attach("diagram")?;
    let scene = Scene::new(svg)?;
    let box_size = Size::new(80.0, 40.0);

    // Isolated: nothing else is anywhere near it, at any point in either test.
    let solo = scene.add_node(Point::new(20.0, 20.0), box_size, "solo")?;
    scene.make_draggable(solo)?;

    // Not draggable — `mover` is dragged onto this one.
    let blocker = scene.add_node(Point::new(300.0, 150.0), box_size, "blocker")?;

    let mover = scene.add_node(Point::new(20.0, 150.0), box_size, "mover")?;
    scene.make_draggable(mover)?;

    scene.add_edge(solo, blocker)?;
    scene.add_edge_with(
        solo,
        blocker,
        ConnectorOptions::default().with_connector_type(ConnectorType::Elbow { corner_radius: 8.0 }),
    )?;

    let hub_options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3)));
    let hub = scene.add_node_with(Point::new(210.0, 220.0), box_size, "hub", hub_options)?;
    let branch_a = scene.add_node(Point::new(60.0, 340.0), box_size, "branch_a")?;
    let branch_b = scene.add_node(Point::new(360.0, 340.0), box_size, "branch_b")?;
    scene.make_draggable(branch_a)?;

    scene.add_edge(hub, branch_a)?;
    scene.add_edge(hub, branch_b)?;

    // Far from every other node, in the top-right corner — see this module's own doc comment for why.
    let view_box = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(500.0, 400.0),
    };
    let bounded = scene.add_node(Point::new(400.0, 20.0), box_size, "bounded")?;
    scene.make_draggable_with(bounded, DragOptions::default().with_bounds(Some(view_box)))?;

    // Not draggable — this one exists purely for `accessibility_tree.rs` to query, never for hit-testing or
    // dragging. Same value `tests/drag/data_node.rs`'s own
    // `a_named_single_value_data_nodes_own_aria_label_includes_the_name_and_the_real_value` test already checks at
    // the DOM level, so the two tests cover the same content at two different layers.
    //
    // Placed well outside the diagram's own `(0, 0, 500, 400)` viewBox, rather than merely far from every other
    // node within it. `Scene::make_draggable`'s own `PushClear` collision resolution checks every node's rect for
    // overlap on drop, this one included, regardless of on-screen position within the viewBox. A spot far outside
    // it can never overlap another node's landing rect, whatever that test's own drag distance turns out to be.
    // This node plays no visual role, so being off-canvas costs nothing. `Accessibility.getPartialAXTree` reads
    // the accessibility tree, not the painted picture, so clipping outside the viewBox does not affect it either.
    scene.add_named_data_node(
        Point::new(2000.0, 2000.0),
        "B",
        DataNodeContent::new(NodeValues::U64(vec![0xABCD_EF01_2345_6789]), DataFormat::Hexadecimal),
    )?;

    // Panning on, with no toolbar. Dragging empty background then pans the content, so `pan.rs` can drive it with real
    // mouse input and prove it never interferes with a node drag, and vice versa. It also proves panning works without
    // a toolbar, since that is what `InputMode::On` is for.
    scene.set_pan_mode(InputMode::On)?;
    // Wheel zoom on as well, so `pan.rs` can zoom with a real wheel in the middle of a real drag.
    scene.set_wheel_zoom_mode(InputMode::On)?;

    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
