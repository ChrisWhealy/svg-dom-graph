//! `DragOptions::bounds`: clamps a drag event to ensure the dragged object stays inside a given rectangle (usually the
//! `<svg>`'s viewBox). This avoids a usability bug in which a node could be dragged outside its own `<svg>`'s visible
//! area, then dropped. This clips the node making it unclickable.

use crate::common::{attr_f64, check, check_close, dispatch_pointer_event, make_svg, nth_group};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    Error,
    scene::{DragOptions, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a bounded node far past the top-left corner clamps its rect to `bounds`'s own near edge, `(0, 0)`,
/// rather than letting it go negative.
#[wasm_bindgen_test]
fn dragging_past_the_near_edge_clamps_to_bounds_origin() -> Result<(), String> {
    let svg = make_svg("bounds-near-edge", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(a, options).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-near-edge", 0)?;
    let rect_a = group_a
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in A's group")?;

    // Drags A 1000 client-pixels up and to the left — far past the top-left corner of the view box.
    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;

    check_close(attr_f64(&rect_a, "x")?, 0.0)?;
    check_close(attr_f64(&rect_a, "y")?, 0.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a bounded node far past the bottom-right corner clamps its rect so its own bottom-right corner stays
/// on `bounds`'s own far edge: `x = 400 - 90 = 310`, `y = 260 - 50 = 210`.
#[wasm_bindgen_test]
fn dragging_past_the_far_edge_clamps_to_bounds_own_far_corner() -> Result<(), String> {
    let svg = make_svg("bounds-far-edge", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(a, options).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-far-edge", 0)?;
    let rect_a = group_a
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in A's group")?;

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", 1100, 1100, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", 1100, 1100, 1)?;

    check_close(attr_f64(&rect_a, "x")?, 310.0)?;
    check_close(attr_f64(&rect_a, "y")?, 210.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::make_draggable` (no `bounds`) stays exactly as unconstrained as before this feature existed — a plain
/// regression guard against clamping ever applying unconditionally.
#[wasm_bindgen_test]
fn dragging_without_bounds_stays_unconstrained() -> Result<(), String> {
    let svg = make_svg("bounds-none", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(a).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-none", 0)?;
    let rect_a = group_a
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in A's group")?;

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;

    check_close(attr_f64(&rect_a, "x")?, -900.0)?;
    check_close(attr_f64(&rect_a, "y")?, -900.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After a bounded drag clamps a node to the edge and drops it there, the same node's pointer listeners are still
/// live and still respond to a fresh drag — the exact case this feature exists to protect. Without `bounds`, a
/// node dropped outside the view box renders clipped, and a real browser could never hit-test it again to start
/// this second drag at all.
#[wasm_bindgen_test]
fn a_node_clamped_to_the_edge_can_still_be_dragged_again() -> Result<(), String> {
    let svg = make_svg("bounds-redrag", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(a, options).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-redrag", 0)?;
    let rect_a = group_a
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in A's group")?;

    // First drag: far past the near edge, clamped to (0, 0).
    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;
    check_close(attr_f64(&rect_a, "x")?, 0.0)?;
    check_close(attr_f64(&rect_a, "y")?, 0.0)?;

    // Second drag, starting from wherever the node now sits: moves it by (40, 20), still comfortably in bounds.
    dispatch_pointer_event(&group_a, "pointerdown", 0, 0, 2)?;
    dispatch_pointer_event(&group_a, "pointermove", 40, 20, 2)?;
    dispatch_pointer_event(&group_a, "pointerup", 40, 20, 2)?;

    check(
        attr_f64(&rect_a, "x")? > 1.0 && attr_f64(&rect_a, "y")? > 1.0,
        "a node clamped to the edge by one drag did not respond to a later, separate drag",
    )?;
    check_close(attr_f64(&rect_a, "x")?, 40.0)?;
    check_close(attr_f64(&rect_a, "y")?, 20.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::make_draggable_with` rejects a `DragOptions::bounds` with a non-finite origin, before installing
/// anything — a rejected call leaves `id` not draggable at all, not draggable-but-unbounded.
#[wasm_bindgen_test]
fn make_draggable_with_rejects_a_non_finite_bounds_origin() -> Result<(), String> {
    let svg = make_svg("bounds-nan-origin", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(f64::NAN, 0.0),
        size: Size::new(100.0, 100.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));

    let result = scene.make_draggable_with(a, options);
    check(
        matches!(result, Err(Error::InvalidDragBounds(_))),
        &format!("a NaN bounds origin should have been rejected as Err(Error::InvalidDragBounds(_)), got {result:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::make_draggable_with` rejects a `DragOptions::bounds` with a negative width or height.
#[wasm_bindgen_test]
fn make_draggable_with_rejects_a_negative_bounds_size() -> Result<(), String> {
    let svg = make_svg("bounds-negative-size", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(-100.0, 100.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));

    let result = scene.make_draggable_with(a, options);
    check(
        matches!(result, Err(Error::InvalidDragBounds(_))),
        &format!(
            "a negative bounds width should have been rejected as Err(Error::InvalidDragBounds(_)), got {result:?}"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rejected `bounds` leaves `id` exactly as it was: not marked draggable at all, not silently installed
/// without the bound. A later, valid `make_draggable_with` call for the same node still succeeds.
#[wasm_bindgen_test]
fn a_rejected_bounds_leaves_the_node_not_draggable() -> Result<(), String> {
    let svg = make_svg("bounds-rejected-then-valid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let invalid_bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(-1.0, 100.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;

    scene
        .make_draggable_with(a, DragOptions::default().with_bounds(Some(invalid_bounds)))
        .expect_err("invalid bounds should have been rejected");

    // If the rejected call had left `a` marked draggable, this second call would fail with AlreadyDraggable.
    scene.make_draggable(a).map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A zero-width or zero-height `bounds` is legal, not rejected — `clamp_to_bounds` already gives it a
/// deterministic result: every drag pins to that zero-width axis's own fixed coordinate.
#[wasm_bindgen_test]
fn make_draggable_with_accepts_a_zero_width_bounds() -> Result<(), String> {
    let svg = make_svg("bounds-zero-width", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(50.0, 0.0),
        size: Size::new(0.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));

    scene.make_draggable_with(a, options).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-zero-width", 0)?;
    let rect_a = group_a
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in A's group")?;

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", 500, 500, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", 500, 500, 1)?;

    check_close(attr_f64(&rect_a, "x")?, 50.0)
}
