//! Browser tests for the drag-to-reroute pipeline: pointerdown, pointer capture, pointermove, model update, rect
//! move, label move, edge reroute, pointerup.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state.
//! That proves the whole pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state
//! changed correctly.

mod common;

use common::{
    attr_f64, check, check_close, dispatch_pointer_event, dispatch_pointer_event_with_button, line_count, make_svg,
    marker_ids, nth_group, the_connector,
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    Error,
    geometry::boundary_point,
    scene::{CollisionPolicy, DragOptions, Scene},
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// The centre point of a box's rectangle.
/// Mirrors `scene`'s own private `box_centre`, so the expected connector position can be computed independently,
/// from outside the crate.
fn centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a node moves its rendered `<rect>` and `<text>` label, and reroutes its connector's far end.
/// The `<svg>`'s `viewBox` matches its pixel size 1:1 here, so a client-pixel drag is a same-sized user-space move.
#[wasm_bindgen_test]
fn dragging_a_node_moves_its_rect_label_and_reroutes_its_edge() -> Result<(), String> {
    let svg = make_svg("drag-1to1", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let a_rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(90.0, 50.0),
    };
    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene.add_node(a_rect.origin, a_rect.size, "A").map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("drag-1to1", 1)?; // B was added second.
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;
    let label_b = group_b
        .query_selector("text")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <text> in B's group")?;
    let connector = the_connector("drag-1to1")?;

    // pointerdown -> pointer capture -> pointermove: drag by 50 client-pixels right, 30 down.
    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 150, 130, 1)?;

    let b_rect_after = Rect {
        origin: Point::new(b_rect_before.origin.x + 50.0, b_rect_before.origin.y + 30.0),
        size: b_rect_before.size,
    };

    // The rect moved by exactly the drag delta (1:1 client-pixel-to-user-space here).
    check_close(attr_f64(&rect_b, "x")?, b_rect_after.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_after.origin.y)?;

    // The label re-centred on the rect's new position.
    let new_centre = centre(b_rect_after);
    check_close(attr_f64(&label_b, "x")?, new_centre.x)?;
    check_close(attr_f64(&label_b, "y")?, new_centre.y)?;

    // The connector's B-end rerouted to B's new boundary point, computed independently via the same public
    // boundary-routing function the crate itself uses.
    let expected_end = boundary_point(b_rect_after, centre(a_rect));
    check_close(attr_f64(&connector, "x2")?, expected_end.x)?;
    check_close(attr_f64(&connector, "y2")?, expected_end.y)?;

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same drag, but with a `viewBox` half the `<svg>`'s rendered pixel size — so one user-space unit spans two
/// CSS pixels.
///
/// Proves the pointer-coordinate conversion, not just that a drag moves something.
/// If the crate still assumed client pixels equalled user-space units, the observed move would come out twice the
/// expected size.
#[wasm_bindgen_test]
fn dragging_under_a_scaled_view_box_converts_client_pixels_to_user_space() -> Result<(), String> {
    let svg = make_svg("drag-scaled", Size::new(400.0, 260.0), Size::new(200.0, 130.0));

    let b_rect_before = Rect {
        origin: Point::new(100.0, 75.0),
        size: Size::new(45.0, 25.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(45.0, 25.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("drag-scaled", 1)?;
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;

    // 100 x 60 CLIENT pixels of movement...
    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 200, 160, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 200, 160, 1)?;

    // ...must land as 50 x 30 USER-SPACE units, since 1 user unit = 2 client pixels under this viewBox.
    check_close(attr_f64(&rect_b, "x")?, b_rect_before.origin.x + 50.0)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_before.origin.y + 30.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `make_draggable`'s listener closures must not keep the scene's internal state alive on their own.
///
/// `Scene` is a cheap handle around shared internal state, so an external test cannot hold a `Weak` reference the
/// way it could when callers had to wrap `Scene` in `Rc<RefCell<_>>` themselves — that internal sharing strategy is
/// no longer observable from outside the crate. Instead, this checks the same property behaviourally: registers
/// draggable handlers, then drops every `Scene` handle this test holds before dragging. If a listener closure
/// captured a strong reference to the scene's internal state (rather than a `Weak` one), the closure's
/// `Weak::upgrade()` would still succeed and the drag would still move the box; a correctly weak-captured closure
/// finds nothing left to upgrade to, and the drag becomes a silent no-op.
#[wasm_bindgen_test]
fn dropping_every_scene_handle_makes_dragging_a_silent_no_op() -> Result<(), String> {
    let svg = make_svg("scene-lifetime", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(200.0, 150.0), Size::new(90.0, 50.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    scene.make_draggable(a).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("scene-lifetime", 1)?;
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;
    let x_before = attr_f64(&rect_b, "x")?;
    let y_before = attr_f64(&rect_b, "y")?;

    drop(scene); // every handle this test ever held

    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 150, 130, 1)?;

    let leaked = "dragging still moved the box after every Scene handle was dropped, meaning a listener closure \
        leaked a strong reference to the scene's internal state";
    check_close(attr_f64(&rect_b, "x")?, x_before).map_err(|e| format!("{e} — {leaked}"))?;
    check_close(attr_f64(&rect_b, "y")?, y_before).map_err(|e| format!("{e} — {leaked}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A second pointer touching the same element mid-drag must not steal the drag, drive it, or end it by lifting off.
///
/// Sends a real `pointerdown` for the second pointer too, not just a stray `pointermove`/`pointerup` — a `pointerdown`
/// that doesn't check for an already-active drag would silently overwrite the first pointer's `DragStart`, after which
/// pointer 2's own `pointermove` would then legitimately match and drive the box, since the `pointer_id` guards defined
/// elsewhere only check against whichever `DragStart` is currently stored.
#[wasm_bindgen_test]
fn a_second_pointer_cannot_steal_drive_or_end_another_pointers_drag() -> Result<(), String> {
    let svg = make_svg("multi-pointer", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("multi-pointer", 1)?;
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;

    // Pointer 1 starts the drag.
    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;

    // Pointer 2 touches down on the same element, then moves. Must not steal or drive pointer 1's drag.
    dispatch_pointer_event(&group_b, "pointerdown", 500, 500, 2)?;
    dispatch_pointer_event(&group_b, "pointermove", 550, 550, 2)?;
    check_close(attr_f64(&rect_b, "x")?, b_rect_before.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_before.origin.y)?;

    // Pointer 2 lifts off. Must not end pointer 1's still-active drag.
    dispatch_pointer_event(&group_b, "pointerup", 550, 550, 2)?;

    // Pointer 1 keeps moving and finishes the drag. This only works if pointer 2 never gained control of it.
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 150, 130, 1)?;

    let b_rect_after = Rect {
        origin: Point::new(b_rect_before.origin.x + 50.0, b_rect_before.origin.y + 30.0),
        size: b_rect_before.size,
    };
    check_close(attr_f64(&rect_b, "x")?, b_rect_after.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_after.origin.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A non-primary pointerdown (a right or middle mouse button, `button() != 0`) must not start a drag.
///
/// Pointer Events use `0` for the primary button — left mouse, touch, or ordinary pen contact — `1` for the middle
/// mouse button, and `2` for the right mouse button or a pen's barrel button.
/// Right-clicking a node (for example, to open a context menu) must not put it into drag mode.
#[wasm_bindgen_test]
fn a_non_primary_button_pointerdown_does_not_start_a_drag() -> Result<(), String> {
    let svg = make_svg("non-primary-button", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("non-primary-button", 1)?;
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;

    // A right-click (button 2) followed by a move under the same pointer_id must not move the box.
    dispatch_pointer_event_with_button(&group_b, "pointerdown", 100, 100, 1, 2)?;
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 150, 130, 1)?;

    check_close(attr_f64(&rect_b, "x")?, b_rect_before.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_before.origin.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An unrelated pointer's `pointercancel` must not end another pointer's active drag either.
///
/// Same property as the `pointerup` half of the test above, checked separately since `pointercancel` is a distinct
/// listener with its own `pointer_id` guard.
#[wasm_bindgen_test]
fn an_unrelated_pointers_pointercancel_does_not_end_the_active_drag() -> Result<(), String> {
    let svg = make_svg("multi-pointer-cancel", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("multi-pointer-cancel", 1)?;
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;

    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_b, "pointercancel", 0, 0, 2)?; // an unrelated pointer

    // Pointer 1's drag must still be active.
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 150, 130, 1)?;

    let b_rect_after = Rect {
        origin: Point::new(b_rect_before.origin.x + 50.0, b_rect_before.origin.y + 30.0),
        size: b_rect_before.size,
    };
    check_close(attr_f64(&rect_b, "x")?, b_rect_after.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_after.origin.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_edge(node, node)` must be rejected, not silently accepted as a zero-length connector.
///
/// Both endpoints of a self-edge share one rectangle, so `boundary_point` (which returns a rectangle's own centre
/// when the target point is already the centre) would compute the same point for both ends — a real `<line>`
/// element with no visible extent, not a meaningful connector. Checks both that the call returns `Err` and that it
/// drew nothing, since a caller could otherwise still end up with invisible connector elements accumulating in the
/// DOM.
#[wasm_bindgen_test]
fn add_edge_rejects_a_self_loop() -> Result<(), String> {
    let svg = make_svg("self-loop", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;

    let result = scene.add_edge(a, a);
    check(result.is_err(), "add_edge(node, node) unexpectedly succeeded")?;
    let message = result.unwrap_err().to_string();
    check(
        message.contains("not yet supported"),
        &format!("unexpected error message for a self-loop: {message:?}"),
    )?;

    check(
        line_count("self-loop")? == 0,
        "add_edge(node, node) drew a connector despite returning Err",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `NodeId` from a different `Scene` must be rejected with an error, not silently treated as one of this
/// `Scene`'s own nodes.
///
/// Both `Scene`s' graphs number their nodes from zero, so `foreign`'s `NodeId` and this scene's own first `NodeId`
/// share the same internal sequence position — exactly the case a numbering scheme not scoped to its owning
/// `Scene` would confuse.
#[wasm_bindgen_test]
fn a_node_id_from_a_different_scene_is_rejected_not_silently_mismatched() -> Result<(), String> {
    let foreign_svg = make_svg("foreign-scene", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let svg = make_svg("local-scene", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let local = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Local")
        .map_err(|e| e.to_string())?;

    check(
        scene.add_edge(local, foreign).is_err(),
        "add_edge silently accepted a NodeId from a different Scene",
    )?;

    check(
        scene.make_draggable(foreign).is_err(),
        "make_draggable silently accepted a NodeId from a different Scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_edge(foreign, foreign)` — the same foreign id used for both endpoints — must be reported as an unknown
/// node, not a self-loop.
///
/// `foreign` names no node at all in `scene`, so `UnknownNode` is the correct diagnosis even though `from == to`.
/// Checking membership before the self-loop comparison is what makes this so: reversing that order would report
/// `SelfLoopUnsupported` instead, which is misleading — the id does not belong to this scene at all, let alone
/// name a node connected to itself.
#[wasm_bindgen_test]
fn add_edge_with_the_same_foreign_id_twice_is_reported_as_unknown_not_a_self_loop() -> Result<(), String> {
    let foreign_svg = make_svg("foreign-self-pair", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let svg = make_svg("local-only", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let result = scene.add_edge(foreign, foreign);
    check(result.is_err(), "add_edge(foreign, foreign) unexpectedly succeeded")?;
    let message = result.unwrap_err().to_string();
    check(
        message.contains("does not belong to this Scene"),
        &format!("expected an UnknownNode-style message, got {message:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two `Scene`s sharing one `<svg>` must not collide on their arrow marker's id.
///
/// A hardcoded id such as `"arrow"` would make the second `Scene::new` either fail outright or silently produce a
/// second `<marker id="arrow">`, leaving both `Scene`s' connectors pointing at whichever one the browser resolves
/// `url(#arrow)` to.
#[wasm_bindgen_test]
fn scenes_sharing_one_svg_get_distinct_arrow_marker_ids() -> Result<(), String> {
    let svg1 = make_svg("shared-svg", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    // A second, independent `SvgRoot` handle attached to the same live `<svg id="shared-svg">` — exactly the
    // "two Scenes exist in one SVG" scenario the marker id needs to survive.
    let svg2 = SvgRoot::attach("shared-svg").map_err(|e| e.to_string())?;

    let _scene1 = Scene::new(svg1).map_err(|e| e.to_string())?;
    let _scene2 = Scene::new(svg2).map_err(|e| e.to_string())?;

    let ids = marker_ids("shared-svg")?;
    check(
        ids.len() == 2,
        &format!("expected exactly 2 <marker> elements, found {}", ids.len()),
    )?;
    check(
        ids[0] != ids[1],
        &format!("both Scenes' arrow markers share the same id: {ids:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping a dragged node so it overlaps another pushes it back to a clear position, along the line from the blocker's
/// centre to where the drag started, plus padding.
///
/// Same geometry as `cdp-integration-test`'s `overlap_resolution` test, so the expected result there — worked out by
/// hand in that test's own doc comment — applies unchanged here: mover (20, 150), blocker (300, 150), both 80x40,
/// dropped on blocker's centre lands mover's origin at (214, 150).
#[wasm_bindgen_test]
fn dropping_a_dragged_node_onto_another_pushes_it_back_to_a_clear_position() -> Result<(), String> {
    let svg = make_svg("drag-overlap", Size::new(500.0, 300.0), Size::new(500.0, 300.0));
    let box_size = Size::new(80.0, 40.0);

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .add_node(Point::new(300.0, 150.0), box_size, "blocker")
        .map_err(|e| e.to_string())?;
    let mover = scene
        .add_node(Point::new(20.0, 150.0), box_size, "mover")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(mover).map_err(|e| e.to_string())?;

    let group_mover = nth_group("drag-overlap", 1)?; // mover was added second.
    let rect_mover = group_mover
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in mover's group")?;

    // Drag mover's centre (60, 170) onto blocker's centre (340, 170) — a 280-pixel move right, 1:1 client-pixel
    // to user-space here.
    dispatch_pointer_event(&group_mover, "pointerdown", 60, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointermove", 340, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointerup", 340, 170, 1)?;

    check_close(attr_f64(&rect_mover, "x")?, 214.0)?;
    check_close(attr_f64(&rect_mover, "y")?, 150.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `CollisionPolicy::Allow` leaves a dropped node exactly where the pointer released it, even where that overlaps
/// another node — the push-clear correction the previous test checks is opt-in, not `make_draggable_with`'s only
/// behaviour.
///
/// Same drag as `dropping_a_dragged_node_onto_another_pushes_it_back_to_a_clear_position`, so the same 280-pixel
/// move lands mover's rect exactly on blocker's own origin (300, 150) if nothing corrects it afterwards.
#[wasm_bindgen_test]
fn dropping_a_dragged_node_onto_another_with_allow_policy_leaves_them_overlapping() -> Result<(), String> {
    let svg = make_svg("drag-overlap-allow", Size::new(500.0, 300.0), Size::new(500.0, 300.0));
    let box_size = Size::new(80.0, 40.0);

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .add_node(Point::new(300.0, 150.0), box_size, "blocker")
        .map_err(|e| e.to_string())?;
    let mover = scene
        .add_node(Point::new(20.0, 150.0), box_size, "mover")
        .map_err(|e| e.to_string())?;
    scene
        .make_draggable_with(
            mover,
            DragOptions {
                collision: CollisionPolicy::Allow,
            },
        )
        .map_err(|e| e.to_string())?;

    let group_mover = nth_group("drag-overlap-allow", 1)?; // mover was added second.
    let rect_mover = group_mover
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in mover's group")?;

    dispatch_pointer_event(&group_mover, "pointerdown", 60, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointermove", 340, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointerup", 340, 170, 1)?;

    check_close(attr_f64(&rect_mover, "x")?, 300.0)?;
    check_close(attr_f64(&rect_mover, "y")?, 150.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `PushClear` drop whose pre-drag centre coincides exactly with the blocker's own centre reverts to the
/// pre-drag origin, rather than landing somewhere still overlapping.
///
/// mover and blocker start at the same origin (so `add_node` allows an overlapping starting position, and their
/// centres coincide exactly), then mover is dragged a short distance that leaves it still overlapping blocker.
/// `resolve_overlap`'s own doc comment (`src/scene/mod.rs`) works out that this specific case is numerically
/// identical whether or not it is handled as an explicit fallback — the point of handling it explicitly is not to
/// change this outcome but to stop it depending on an algebraic coincidence inside `nearest_clear_centre`. This
/// test locks the outcome in either way, so a future change to either function that broke it would be caught here.
#[wasm_bindgen_test]
fn dropping_a_node_whose_pre_drag_centre_coincides_with_the_blockers_centre_reverts_the_drag() -> Result<(), String> {
    let svg = make_svg("drag-degenerate", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let box_size = Size::new(80.0, 40.0);
    let same_origin = Point::new(50.0, 50.0);

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene.add_node(same_origin, box_size, "blocker").map_err(|e| e.to_string())?;
    let mover = scene.add_node(same_origin, box_size, "mover").map_err(|e| e.to_string())?;
    scene.make_draggable(mover).map_err(|e| e.to_string())?;

    let group_mover = nth_group("drag-degenerate", 1)?; // mover was added second.
    let rect_mover = group_mover
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in mover's group")?;

    // A short drag that leaves mover still overlapping blocker — mover and blocker started fully coincident.
    dispatch_pointer_event(&group_mover, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_mover, "pointermove", 105, 105, 1)?;
    dispatch_pointer_event(&group_mover, "pointerup", 105, 105, 1)?;

    check_close(attr_f64(&rect_mover, "x")?, same_origin.x)?;
    check_close(attr_f64(&rect_mover, "y")?, same_origin.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A second `make_draggable`/`make_draggable_with` call for the same node is rejected with
/// `Error::AlreadyDraggable`, rather than silently installing a second, independent set of pointer listeners
/// alongside the first.
///
/// Also drags the node afterwards and checks it moved by exactly the drag delta, not double it — the strongest
/// available proof that the rejected second call did not sneak a duplicate `move_node` call onto every
/// `pointermove` alongside the first installation's own.
#[wasm_bindgen_test]
fn a_second_make_draggable_call_for_the_same_node_is_rejected() -> Result<(), String> {
    let svg = make_svg("drag-twice", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let box_size = Size::new(80.0, 40.0);
    let before = Point::new(20.0, 20.0);

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene.add_node(before, box_size, "node").map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;

    let second = scene.make_draggable(node);
    check(
        matches!(second, Err(Error::AlreadyDraggable(_))),
        &format!("expected Err(Error::AlreadyDraggable(_)) from a second make_draggable call, got {second:?}"),
    )?;
    let third = scene.make_draggable_with(node, DragOptions::default());
    check(
        matches!(third, Err(Error::AlreadyDraggable(_))),
        &format!("expected Err(Error::AlreadyDraggable(_)) from a second make_draggable_with call, got {third:?}"),
    )?;

    let group = nth_group("drag-twice", 0)?;
    let rect = group
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in node's group")?;

    // A 50x30 client-pixel drag, 1:1 with user-space here.
    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group, "pointerup", 150, 130, 1)?;

    check_close(attr_f64(&rect, "x")?, before.x + 50.0)?;
    check_close(attr_f64(&rect, "y")?, before.y + 30.0)
}
