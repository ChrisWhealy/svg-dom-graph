//! Browser tests for the drag-to-reroute pipeline: pointerdown, pointer capture, pointermove, model update, rect
//! move, label move, edge reroute, pointerup.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state.
//! That proves the whole pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state
//! changed correctly.

mod common;

use common::{
    attr_f64, check, check_close, connector_count, dispatch_pointer_event, dispatch_pointer_event_with_button,
    last_point_of_path, make_svg, marker_ids, nth_group, path_d, the_connector,
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    Error,
    scene::{CollisionPolicy, ConnectorOptions, ConnectorType, DragOptions, Scene},
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

    // The connector's B-end rerouted to sit at the midpoint of one of B's new rect's four sides.
    // This is the anchor rule every elbowed connector follows, checked here independently of the crate's own
    // internal side-selection logic.
    let (end_x, end_y) = last_point_of_path(&path_d(&connector)?)?;
    let b = b_rect_after;
    let side_midpoints = [
        (b.origin.x, b.origin.y + b.size.height / 2.0),                // west
        (b.origin.x + b.size.width, b.origin.y + b.size.height / 2.0), // east
        (b.origin.x + b.size.width / 2.0, b.origin.y),                 // north
        (b.origin.x + b.size.width / 2.0, b.origin.y + b.size.height), // south
    ];
    check(
        side_midpoints
            .iter()
            .any(|&(mx, my)| (end_x - mx).abs() < 0.01 && (end_y - my).abs() < 0.01),
        &format!("connector end ({end_x}, {end_y}) is not the midpoint of any side of B's new rect {b:?}"),
    )?;

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
/// A `pointercancel` belonging to the drag's own active pointer ends that drag: a further move from the same
/// pointer no longer moves the node, and starting an entirely new drag afterwards still works normally.
///
/// The complement to `an_unrelated_pointers_pointercancel_does_not_end_the_active_drag`, which only checks that an
/// unrelated pointer's `pointercancel` is ignored — this checks the positive path `pointercancel` exists for:
/// clearing `drag_start` for the pointer it actually belongs to.
#[wasm_bindgen_test]
fn a_pointercancel_for_the_active_pointer_ends_the_drag() -> Result<(), String> {
    let svg = make_svg("pointer-cancel-active", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let group_b = nth_group("pointer-cancel-active", 0)?; // B was added first.
    let rect_b = group_b
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in B's group")?;

    // Start a drag with pointer 1, move it, then cancel that same pointer.
    dispatch_pointer_event(&group_b, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointercancel", 150, 130, 1)?;

    let b_rect_cancelled = Rect {
        origin: Point::new(b_rect_before.origin.x + 50.0, b_rect_before.origin.y + 30.0),
        size: b_rect_before.size,
    };
    check_close(attr_f64(&rect_b, "x")?, b_rect_cancelled.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_cancelled.origin.y)?;

    // Further movement from the same, now-cancelled pointer must not move the node any further.
    dispatch_pointer_event(&group_b, "pointermove", 200, 200, 1)?;
    check_close(attr_f64(&rect_b, "x")?, b_rect_cancelled.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_cancelled.origin.y)?;

    // A brand new drag afterwards — even reusing the same pointer_id, since pointercancel released it — still
    // works normally.
    dispatch_pointer_event(&group_b, "pointerdown", 150, 130, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 200, 160, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 200, 160, 1)?;

    let b_rect_after = Rect {
        origin: Point::new(b_rect_cancelled.origin.x + 50.0, b_rect_cancelled.origin.y + 30.0),
        size: b_rect_before.size,
    };
    check_close(attr_f64(&rect_b, "x")?, b_rect_after.origin.x)?;
    check_close(attr_f64(&rect_b, "y")?, b_rect_after.origin.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_edge(node, node)` must be rejected, not silently accepted as a zero-length connector.
///
/// Both endpoints of a self-edge share one rectangle, so both ends would anchor at the same point — a real `<path>`
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
        connector_count("self-loop")? == 0,
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
        .make_draggable_with(mover, DragOptions::default().with_collision(CollisionPolicy::Allow))
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `make_draggable_with` rejects a `CollisionPolicy::PushClear` padding that is not a finite value `>= 0.0`, before
/// changing any state — a rejected call never marks the node draggable, so the same node can be retried (here, with
/// several different invalid values in a row) without ever hitting `Error::AlreadyDraggable` instead.
#[wasm_bindgen_test]
fn make_draggable_with_rejects_a_non_finite_or_negative_padding() -> Result<(), String> {
    let svg = make_svg("drag-padding", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "node")
        .map_err(|e| e.to_string())?;

    for padding in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = scene.make_draggable_with(
            node,
            DragOptions::default().with_collision(CollisionPolicy::PushClear { padding }),
        );
        check(
            matches!(result, Err(Error::InvalidCollisionPadding(_))),
            &format!(
                "padding {padding} should have been rejected as Err(Error::InvalidCollisionPadding(_)), got {result:?}"
            ),
        )?;
    }

    // 0.0 is the boundary — valid, not rejected.
    scene
        .make_draggable_with(
            node,
            DragOptions::default().with_collision(CollisionPolicy::PushClear { padding: 0.0 }),
        )
        .map_err(|e| e.to_string())?;

    // A distinct, non-default positive padding is also valid, on a separate node (the one above is now already
    // draggable).
    let other = scene
        .add_node(Point::new(200.0, 0.0), Size::new(80.0, 40.0), "other")
        .map_err(|e| e.to_string())?;
    scene
        .make_draggable_with(
            other,
            DragOptions::default().with_collision(CollisionPolicy::PushClear { padding: 42.5 }),
        )
        .map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_node` rejects non-finite or non-positive geometry before drawing anything or touching the graph's model —
/// a rejected call leaves the scene with no rendered `<g>` at all, and a later, valid `add_node` call still lands
/// as the scene's first (and only) node, not a second one after some partially-added ghost.
#[wasm_bindgen_test]
fn add_node_rejects_invalid_geometry_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("add-node-geometry", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let box_size = Size::new(80.0, 40.0);
    let origin = Point::new(20.0, 20.0);

    let invalid_sizes = [
        Size::new(-80.0, 40.0),             // negative width
        Size::new(80.0, -40.0),             // negative height
        Size::new(0.0, 40.0),               // zero width
        Size::new(80.0, 0.0),               // zero height
        Size::new(f64::NAN, 40.0),          // NaN width
        Size::new(80.0, f64::NAN),          // NaN height
        Size::new(f64::INFINITY, 40.0),     // +inf width
        Size::new(f64::NEG_INFINITY, 40.0), // -inf width
    ];
    for size in invalid_sizes {
        let result = scene.add_node(origin, size, "invalid");
        check(
            matches!(result, Err(Error::InvalidNodeGeometry(_))),
            &format!("size {size:?} should have been rejected as Err(Error::InvalidNodeGeometry(_)), got {result:?}"),
        )?;
    }

    let invalid_origins = [
        Point::new(f64::NAN, 20.0),
        Point::new(20.0, f64::NAN),
        Point::new(f64::INFINITY, 20.0),
        Point::new(20.0, f64::NEG_INFINITY),
    ];
    for invalid_origin in invalid_origins {
        let result = scene.add_node(invalid_origin, box_size, "invalid");
        check(
            matches!(result, Err(Error::InvalidNodeGeometry(_))),
            &format!(
                "origin {invalid_origin:?} should have been rejected as Err(Error::InvalidNodeGeometry(_)), got {result:?}"
            ),
        )?;
    }

    // None of the rejected calls above touched the scene — no <g> has been rendered yet.
    check(
        nth_group("add-node-geometry", 0).is_err(),
        "a rejected add_node call left a <g> rendered in the scene",
    )?;

    // A valid call afterwards still lands as the scene's first node.
    scene.add_node(origin, box_size, "valid").map_err(|e| e.to_string())?;
    nth_group("add-node-geometry", 0)?;
    check(
        nth_group("add-node-geometry", 1).is_err(),
        "expected exactly one <g> after the rejected calls and one valid add_node call",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// When a drop overlaps two blockers exactly equidistant from its centre, `resolve_overlap` breaks the tie by
/// choosing the lower `NodeId` index, not by `HashMap`'s unspecified iteration order.
///
/// `blocker_a` and `blocker_b` sit symmetrically either side of where `mover` lands, both exactly 40 user-space
/// units from its centre and both genuinely overlapping it. Resolving against `blocker_a` (added first, the lower
/// index) and against `blocker_b` (added second) push in opposite directions and land at different, hand-worked
/// positions — (134, 150) vs (214, 150) — so this test can tell which one actually won, not just that the result
/// avoided both.
///
/// Worked out with `blocker_a`'s own geometry, mirroring `overlap_resolution.rs`'s CDP test: mover starts at
/// (20, 150), size (80, 40) — pre-drag centre (60, 170), on the same horizontal line as every box here, so the
/// approach direction and every intermediate calculation stay purely horizontal (no diagonal rounding). `mover` is
/// dragged so its centre lands at (300, 170) — origin (260, 150). `blocker_a`, size (80, 40), centred at
/// (260, 170) (origin (220, 150)), inflated by half of `mover`'s size on every side spans x: [180, 340],
/// y: [130, 210], centre (260, 170). The approach line from (60, 170) is horizontal, crossing that inflated
/// boundary at x = 180. Padding (6.0, the default) pushes another 6 units left, to (174, 170) — `mover`'s final
/// origin (174 - 40, 170 - 20) = (134, 150).
#[wasm_bindgen_test]
fn dropping_between_two_equidistant_blockers_resolves_to_the_lower_index() -> Result<(), String> {
    let svg = make_svg("drag-tiebreak", Size::new(500.0, 300.0), Size::new(500.0, 300.0));
    let box_size = Size::new(80.0, 40.0);

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let mover = scene
        .add_node(Point::new(20.0, 150.0), box_size, "mover")
        .map_err(|e| e.to_string())?;
    // Centre (260, 170) — 40 units left of where mover will land.
    scene
        .add_node(Point::new(220.0, 150.0), box_size, "blocker_a")
        .map_err(|e| e.to_string())?;
    // Centre (340, 170) — 40 units right of where mover will land. Equidistant from (300, 170) as blocker_a.
    scene
        .add_node(Point::new(300.0, 150.0), box_size, "blocker_b")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(mover).map_err(|e| e.to_string())?;

    let group_mover = nth_group("drag-tiebreak", 0)?; // mover was added first.
    let rect_mover = group_mover
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in mover's group")?;

    // Drag mover's centre (60, 170) to (300, 170) — a 240-pixel move right, 1:1 client-pixel to user-space here.
    dispatch_pointer_event(&group_mover, "pointerdown", 60, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointermove", 300, 170, 1)?;
    dispatch_pointer_event(&group_mover, "pointerup", 300, 170, 1)?;

    check_close(attr_f64(&rect_mover, "x")?, 134.0)?;
    check_close(attr_f64(&rect_mover, "y")?, 150.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` rejects a non-finite or negative `ConnectorType::Elbow` corner radius, before
/// touching the connector, so a rejected call never corrupts the previously-rendered path.
#[wasm_bindgen_test]
fn set_connector_type_rejects_a_non_finite_or_negative_radius() -> Result<(), String> {
    let svg = make_svg("corner-radius-invalid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let connector = the_connector("corner-radius-invalid")?;
    let before = path_d(&connector)?;

    for corner_radius in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius });
        check(
            matches!(result, Err(Error::InvalidCornerRadius(_))),
            &format!(
                "radius {corner_radius} should have been rejected as Err(Error::InvalidCornerRadius(_)), got {result:?}"
            ),
        )?;
    }

    let after = path_d(&connector)?;
    check(
        after == before,
        &format!("a rejected set_connector_type call changed the rendered path: {before:?} -> {after:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` rejects an `EdgeId` that does not name an edge in this scene.
#[wasm_bindgen_test]
fn set_connector_type_rejects_an_unknown_edge() -> Result<(), String> {
    let foreign_svg = make_svg("corner-radius-foreign", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign_a = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let foreign_b = foreign_scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    let foreign_edge = foreign_scene.add_edge(foreign_a, foreign_b).map_err(|e| e.to_string())?;

    let svg = make_svg("corner-radius-local", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    check(
        matches!(
            scene.set_connector_type(foreign_edge, ConnectorType::Elbow { corner_radius: 10.0 }),
            Err(Error::UnknownEdge(_))
        ),
        "set_connector_type silently accepted an EdgeId from a different Scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A corner radius too large for the current geometry renders clamped to the available room, with no error. Once a
/// drag gives the connector more room, the very same stored radius renders at its full, unclamped value — proving
/// the clamp recomputes from the current geometry on every redraw, rather than permanently shrinking the setting.
///
/// # Expected paths, worked by hand
///
/// `A` is fixed at `(0, 0)`, size `(80, 40)` — centre `(40, 20)`, half-extents `(40, 20)`.
/// `B` starts at `(140, 90)`, same size — centre `(180, 110)`.
///
/// From `A` toward `B`: `dx = 140`, `dy = 90`. `20 / 90 ≈ 0.222` is smaller than `40 / 140 ≈ 0.286`, so `A` anchors
/// on its south side: `(40, 40)`. The same comparison, from `B` toward `A`, anchors `B` on its north side:
/// `(180, 90)`.
///
/// Both anchors leave vertically and do not share an x coordinate, so the route jogs across their midpoint:
/// `mid_y = (40 + 90) / 2 = 65`. Each of the two vertical segments is only `25` units long, so a requested radius of
/// `30.0` shrinks to `12.5` at both corners.
///
/// Dragging `B` down by `100` moves it to `(140, 190)` — centre `(180, 210)`. `A`'s own anchor is unchanged, since
/// neither `A` nor the direction toward it moved. `B`'s new anchor is `(180, 190)`. The new `mid_y = (40 + 190) / 2
/// = 115`, so both vertical segments are now `75` units long — comfortably past `2 * 30.0` — so the full requested
/// `30.0` now applies at both corners.
#[wasm_bindgen_test]
fn set_connector_type_clamps_to_available_room_and_restores_when_room_returns() -> Result<(), String> {
    let svg = make_svg("corner-radius-clamp", Size::new(400.0, 400.0), Size::new(400.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene
        .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 30.0 })
        .map_err(|e| e.to_string())?;

    let connector = the_connector("corner-radius-clamp")?;
    let clamped = path_d(&connector)?;
    let expected_clamped = "M 40 40 L 40 52.5 A 12.5 12.5 0 0 0 52.5 65 L 167.5 65 A 12.5 12.5 0 0 1 180 77.5 L 180 90";
    check(
        clamped == expected_clamped,
        &format!("expected the clamped path to be {expected_clamped:?}, got {clamped:?}"),
    )?;

    // Drag B down by 100 units, giving both corners far more than 2 * 30.0 units of headroom.
    let group_b = nth_group("corner-radius-clamp", 1)?; // B was added second.
    dispatch_pointer_event(&group_b, "pointerdown", 180, 110, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 180, 210, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 180, 210, 1)?;

    let restored = path_d(&connector)?;
    let expected_restored = "M 40 40 L 40 85 A 30 30 0 0 0 70 115 L 150 115 A 30 30 0 0 1 180 145 L 180 190";
    check(
        restored == expected_restored,
        &format!("expected the restored path to be {expected_restored:?}, got {restored:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `ConnectorType::Straight` draws the crate's original, pre-elbow connector style: a straight line between the
/// two boxes' own ray/boundary crossings, not an elbowed route.
///
/// `A` is `(0, 0)`, size `(40, 20)` — centre `(20, 10)`. `B` is `(40, 100)`, same size — centre `(60, 110)`. This is
/// the same pair `straight_vertices_between_diagonal_boxes_lands_on_each_rays_own_crossing` uses in
/// `src/geometry/unit_tests.rs`, so the expected endpoints, `(24, 20)` and `(56, 100)`, come from that test.
#[wasm_bindgen_test]
fn add_edge_with_straight_connector_type_draws_the_original_boundary_to_boundary_line() -> Result<(), String> {
    let svg = make_svg("connector-type-straight", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;

    scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(|e| e.to_string())?;

    let connector = the_connector("connector-type-straight")?;
    let d = path_d(&connector)?;
    check(
        d == "M 24 20 L 56 100",
        &format!("expected a straight line from (24, 20) to (56, 100), got {d:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` can switch a live connector between `Straight` and `Elbow`, and back, redrawing it
/// correctly each time — not just adjusting an elbow's own radius.
///
/// Same `A`/`B` pair as `add_edge_with_straight_connector_type_draws_the_original_boundary_to_boundary_line`. The
/// sharp elbow route between them, `(20, 20) -> (20, 60) -> (60, 60) -> (60, 100)`, is worked out the same way
/// `elbow_vertices_between_stacked_boxes_is_one_straight_vertical_segment`'s neighbouring tests in
/// `src/geometry/unit_tests.rs` work out theirs.
#[wasm_bindgen_test]
fn set_connector_type_toggles_a_connector_between_straight_and_elbow() -> Result<(), String> {
    let svg = make_svg("connector-type-toggle", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?; // default: sharp elbow

    let connector = the_connector("connector-type-toggle")?;
    let elbow_d = path_d(&connector)?;
    let expected_elbow = "M 20 20 L 20 60 L 60 60 L 60 100";
    check(
        elbow_d == expected_elbow,
        &format!("expected the sharp elbow route {expected_elbow:?}, got {elbow_d:?}"),
    )?;

    scene
        .set_connector_type(edge, ConnectorType::Straight)
        .map_err(|e| e.to_string())?;
    let straight_d = path_d(&connector)?;
    check(
        straight_d == "M 24 20 L 56 100",
        &format!("expected the straight route after toggling, got {straight_d:?}"),
    )?;

    scene
        .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 0.0 })
        .map_err(|e| e.to_string())?;
    let elbow_again = path_d(&connector)?;
    check(
        elbow_again == expected_elbow,
        &format!("expected toggling back to Elbow to restore {expected_elbow:?}, got {elbow_again:?}"),
    )
}
