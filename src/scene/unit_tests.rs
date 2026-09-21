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
/// `move_node`'s own no-op fast path: an unchanged origin must return `Ok` without reaching the incident-edge
/// redraw loop at all — not merely without changing the rendered result.
///
/// Proved here by corrupting `edge_handles` out from under a real, still-incident edge after the first move: a
/// redraw of that edge can now only fail with `Error::UnknownEdge`. A call with the same origin as the node's
/// current one must still succeed, since it returns before ever reaching that loop. A call with a genuinely new
/// origin must fail, since that same loop is exactly what would need to run.
#[wasm_bindgen_test]
fn move_node_to_the_same_origin_skips_the_edge_redraw_loop_entirely() -> Result<(), String> {
    let svg = make_svg("move-node-no-op");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let origin = Point::new(10.0, 10.0);
    let a = scene.add_node(origin, Size::new(40.0, 20.0), "a").map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(100.0, 0.0), Size::new(40.0, 20.0), "b")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?;

    // Removing the edge's own handle, while leaving it incident on `a` in the graph, reproduces exactly the
    // inconsistency `redraw_edge` reports as `Error::UnknownEdge` — the one and only way to observe from here
    // whether `move_node`'s edge-redraw loop actually ran.
    scene.inner.borrow_mut().remove_edge_handle(edge);

    let mut scratch = String::new();
    let same_origin_result = scene.inner.borrow_mut().move_node(a, origin, &mut scratch);
    check(
        same_origin_result.is_ok(),
        &format!(
            "moving to the same origin should skip the redraw loop and return Ok, instead got {same_origin_result:?}"
        ),
    )?;

    let moved_result = scene.inner.borrow_mut().move_node(a, Point::new(20.0, 20.0), &mut scratch);
    check(
        matches!(moved_result, Err(Error::UnknownEdge(id)) if id == edge),
        &format!(
            "moving to a genuinely new origin should reach the redraw loop and raise UnknownEdge, instead got {moved_result:?}"
        ),
    )
}
