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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Lifetime. A `Scene` is a cheap handle to `Rc`-shared state, and every listener it installs — node dragging, toolbar
// buttons, the pan surface, wheel zoom, keyboard control — holds only a `Weak` reference back to that state. If any of
// them held a strong one, the state would sit in a cycle (`SceneInner` -> DOM node -> listener -> `SceneInner`) and never
// be freed, still responding to input after every `Scene` handle had gone.
//
// The proof is a `Weak` taken from the shared state: once every handle is dropped, it must no longer upgrade.

/// A scene with everything switched on: several nodes and connectors, draggable nodes, the toolbar, and both gestures.
fn busy_scene(id: &str) -> Result<Scene, String> {
    let scene = Scene::new(make_svg(id)).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(10.0, 10.0), Size::new(40.0, 20.0), "a")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(100.0, 10.0), Size::new(40.0, 20.0), "b")
        .map_err(|e| e.to_string())?;
    let c = scene
        .add_node(Point::new(100.0, 100.0), Size::new(40.0, 20.0), "c")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.add_edge(b, c).map_err(|e| e.to_string())?;
    for node in [a, b, c] {
        scene.make_draggable(node).map_err(|e| e.to_string())?;
    }
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    Ok(scene)
}

/// Dispatches a cancelable ctrl+wheel at the scene's pan surface, which schedules a frame.
fn ctrl_wheel_at_surface(id: &str) -> Result<(), String> {
    let surface = document()
        .query_selector(&format!("#{id} > rect"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no pan surface")?;
    let init = web_sys::WheelEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_ctrl_key(true);
    init.set_delta_y(-100.0);
    let event = web_sys::WheelEvent::new_with_event_init_dict("wheel", &init).map_err(|e| format!("{e:?}"))?;
    surface.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn dropping_every_handle_of_a_busy_scene_frees_its_shared_state() -> Result<(), String> {
    let scene = busy_scene("lifetime-busy")?;
    let state = Rc::downgrade(&scene.inner);

    check(
        state.upgrade().is_some(),
        "test setup: the state is already gone while a handle is held",
    )?;
    drop(scene);
    check(
        state.upgrade().is_none(),
        "the shared state outlived every Scene handle: a listener holds a strong reference to it",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A clone is a real handle: the state lives while any one of them does, and is freed when the last goes.
#[wasm_bindgen_test]
fn the_shared_state_lives_until_the_last_clone_is_dropped() -> Result<(), String> {
    let scene = busy_scene("lifetime-clones")?;
    let clone = scene.clone();
    let state = Rc::downgrade(&scene.inner);

    drop(scene);
    check(state.upgrade().is_some(), "the state was freed while a clone was still held")?;
    drop(clone);
    check(state.upgrade().is_none(), "the state outlived the last handle")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The reviewer's sequence, and then some: repeated showing and hiding, and every mode switched back and forth. Each
/// show builds listeners afresh, and each hide takes them away again, so none may be left holding the state.
#[wasm_bindgen_test]
fn repeated_show_hide_and_mode_changes_leave_nothing_holding_the_state() -> Result<(), String> {
    let scene = busy_scene("lifetime-cycles")?;
    let state = Rc::downgrade(&scene.inner);

    for _ in 0..5 {
        scene.hide_toolbar();
        scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    }
    for mode in [
        InputMode::Off,
        InputMode::WithToolbar,
        InputMode::On,
        InputMode::Off,
        InputMode::On,
    ] {
        scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
        scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
    }
    scene.hide_toolbar();
    scene
        .show_toolbar(ToolbarOptions::new(Side::South))
        .map_err(|e| e.to_string())?;

    drop(scene);
    check(
        state.upgrade().is_none(),
        "a listener left by an earlier show, hide, or mode change holds the state",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A frame can be pending when the last handle goes: a wheel event schedules one, and it has not run yet. The scheduled
/// frame holds the state weakly too, so it neither keeps it alive nor breaks when it finds it gone.
#[wasm_bindgen_test]
fn a_scheduled_frame_does_not_keep_the_state_alive() -> Result<(), String> {
    let scene = busy_scene("lifetime-frame")?;
    let state = Rc::downgrade(&scene.inner);

    ctrl_wheel_at_surface("lifetime-frame")?;
    drop(scene);
    check(
        state.upgrade().is_none(),
        "a pending animation frame keeps the shared state alive",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A toolbar that was never shown, and one shown and then hidden, are freed just the same.
#[wasm_bindgen_test]
fn a_scene_that_hid_its_toolbar_is_freed_too() -> Result<(), String> {
    let scene = busy_scene("lifetime-hidden")?;
    scene.hide_toolbar();
    scene.set_pan_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    let state = Rc::downgrade(&scene.inner);

    drop(scene);
    check(
        state.upgrade().is_none(),
        "the shared state outlived a scene whose toolbar was hidden",
    )
}
