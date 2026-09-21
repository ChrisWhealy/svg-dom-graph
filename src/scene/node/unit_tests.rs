use super::*;
use crate::test_support::check;
use svg_dom::root::utils::Size;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A fresh `SvgRoot` in its own container, with a unique `id` so parallel tests in this binary do not collide.
fn make_svg(id: &str) -> SvgRoot {
    let container_id = format!("{id}-container");
    let el = document().create_element("div").unwrap();
    el.set_id(&container_id);
    document().query_selector("body").unwrap().unwrap().append_child(&el).unwrap();

    let svg = SvgRoot::create_in(&container_id, Size::new(200.0, 200.0)).unwrap();
    svg.root.set_id(id);
    svg
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed `RenderGuard` removes both `group` and every element tracked via `track` from the document.
/// `draw_box`/`draw_content_box` can be left in exactly that partial state. This happens when a later fallible step
/// fails, and the `?` operator drops the guard on the way out.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_removes_the_group_and_every_loose_element() -> Result<(), String> {
    let svg = make_svg("render-guard-rollback");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::with_capacity(group.clone(), 0);
    guard.track(loose.clone());
    drop(guard);

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed RenderGuard dropped",
    )?;
    check(
        loose.as_element().parent_node().is_none(),
        "the loosely tracked element was still attached after an armed RenderGuard dropped",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed guard leaves `group` and every tracked element attached. So a
/// successful `draw_box`/`draw_content_box` call is not accidentally rolled back by its own cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_guard_leaves_the_group_and_every_loose_element_attached() -> Result<(), String> {
    let svg = make_svg("render-guard-disarm");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::with_capacity(group.clone(), 0);
    guard.track(loose.clone());
    guard.disarm();

    check(
        group.as_element().parent_node().is_some(),
        "group was removed even though the guard was disarmed",
    )?;
    check(
        loose.as_element().parent_node().is_some(),
        "the loosely tracked element was removed even though the guard was disarmed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed guard that tracked nothing beyond `group` itself is still safe. `group.remove()` is a harmless
/// no-op on an already-empty group.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_with_no_loose_elements_only_removes_the_group() -> Result<(), String> {
    let svg = make_svg("render-guard-empty");
    let group = svg.group().unwrap();

    drop(RenderGuard::with_capacity(group.clone(), 0));

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed, otherwise-empty RenderGuard dropped",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `release` stops tracking the most recently tracked node, so an armed guard dropping afterward leaves it exactly
/// where it was — not one of the elements `draw_content_box`'s own per-cell loop has already appended into `group`
/// (whose own removal would remove it anyway), but proved here directly: even a node never appended anywhere must
/// survive, once released, purely because tracking it stopped.
#[wasm_bindgen_test]
fn release_stops_tracking_the_most_recently_tracked_node() -> Result<(), String> {
    let svg = make_svg("render-guard-release");
    let group = svg.group().unwrap();
    let released = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::with_capacity(group.clone(), 0);
    guard.track(released.clone());
    guard.release();
    drop(guard);

    check(
        group.as_element().parent_node().is_none(),
        "group should still be removed on drop — release only affects the tracked node, not group itself",
    )?;
    check(
        released.as_element().parent_node().is_some(),
        "release should have stopped tracking the node, but dropping the still-armed guard removed it anyway",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed `OperatorConstructionGuard` removes every edge tracked via `track_edge` — its rendered path,
/// its `edge_handles` entry, and its place in the graph — then the node itself, the same partial state a failure
/// drawing an operator's own auto-wired input edge would otherwise leave behind.
#[wasm_bindgen_test]
fn dropping_an_armed_construction_guard_removes_the_node_and_every_tracked_edge() -> Result<(), String> {
    let svg = make_svg("construction-guard-rollback");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let input = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "input")
        .map_err(|e| e.to_string())?;
    let operator = scene
        .add_node(Point::new(100.0, 0.0), Size::new(40.0, 20.0), "operator")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(input, operator).map_err(|e| e.to_string())?;

    let (node_group, edge_path) = {
        let inner = scene.inner.borrow();
        (
            inner.node_handle(operator).unwrap().group.clone(),
            inner.edge_handle(edge).unwrap().path.clone(),
        )
    };

    let mut guard = OperatorConstructionGuard::new(scene.clone(), operator);
    guard.track_edge(edge);
    drop(guard);

    check(
        node_group.as_element().parent_node().is_none(),
        "the operator node's own group was still attached after an armed OperatorConstructionGuard dropped",
    )?;
    check(
        edge_path.as_element().parent_node().is_none(),
        "the tracked edge's own path was still attached after an armed OperatorConstructionGuard dropped",
    )?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(operator).is_none(),
        "node_handles still held the removed operator node",
    )?;
    check(inner.edge_handle(edge).is_none(), "edge_handles still held the removed edge")?;
    check(
        inner.graph.node(operator).is_none(),
        "the graph still held the removed operator node",
    )?;
    check(inner.graph.edge(edge).is_none(), "the graph still held the removed edge")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed `OperatorConstructionGuard` leaves the node and every tracked
/// edge exactly as they were. So a fully successful operator creation is not accidentally rolled back by its own
/// cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_construction_guard_leaves_the_node_and_every_tracked_edge_in_place() -> Result<(), String> {
    let svg = make_svg("construction-guard-disarm");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let input = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "input")
        .map_err(|e| e.to_string())?;
    let operator = scene
        .add_node(Point::new(100.0, 0.0), Size::new(40.0, 20.0), "operator")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(input, operator).map_err(|e| e.to_string())?;

    let mut guard = OperatorConstructionGuard::new(scene.clone(), operator);
    guard.track_edge(edge);
    guard.disarm();

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(operator).is_some(),
        "node_handles lost the operator node despite a disarmed guard",
    )?;
    check(
        inner.edge_handle(edge).is_some(),
        "edge_handles lost the edge despite a disarmed guard",
    )?;
    check(
        inner.graph.node(operator).is_some(),
        "the graph lost the operator node despite a disarmed guard",
    )?;
    check(
        inner.graph.edge(edge).is_some(),
        "the graph lost the edge despite a disarmed guard",
    )
}
