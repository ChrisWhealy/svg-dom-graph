//! Ordinary dragging: coordinate conversion, connector reroute, listener and scene lifetime, and pointer/button
//! edge cases.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state. That proves the
//! whole pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state changed correctly.

use crate::common::{
    attr_f64, check, check_close, dispatch_pointer_event, dispatch_pointer_event_with_button, last_point_of_path,
    make_svg, nth_group, path_d, the_connector,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    Error,
    scene::{CollisionPolicy, DragOptions, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

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
