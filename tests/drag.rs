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
use svg_dom_graph::{geometry::boundary_point, scene::Scene};
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
