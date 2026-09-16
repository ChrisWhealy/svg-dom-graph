//! `DragOptions::bounds`: clamps a drag event to ensure the dragged object stays inside a given rectangle (usually the
//! `<svg>`'s viewBox). This avoids a usability bug in which a node could be dragged outside its own `<svg>`'s visible
//! area, then dropped. This clips the node making it unclickable.

use crate::common::{attr_f64, check, check_close, dispatch_pointer_event, group_translate, make_svg, nth_group};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, DragOptions, GridLayout, NodeValues, Scene},
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

    // Drags A 1000 client-pixels up and to the left — far past the top-left corner of the view box.
    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;

    let (x, y) = group_translate(&group_a)?;
    check_close(x, 0.0)?;
    check_close(y, 0.0)
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

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", 1100, 1100, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", 1100, 1100, 1)?;

    let (x, y) = group_translate(&group_a)?;
    check_close(x, 310.0)?;
    check_close(y, 210.0)
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

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;

    let (x, y) = group_translate(&group_a)?;
    check_close(x, -900.0)?;
    check_close(y, -900.0)
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

    // First drag: far past the near edge, clamped to (0, 0).
    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", -900, -900, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", -900, -900, 1)?;
    let (x, y) = group_translate(&group_a)?;
    check_close(x, 0.0)?;
    check_close(y, 0.0)?;

    // Second drag, starting from wherever the node now sits: moves it by (40, 20), still comfortably in bounds.
    dispatch_pointer_event(&group_a, "pointerdown", 0, 0, 2)?;
    dispatch_pointer_event(&group_a, "pointermove", 40, 20, 2)?;
    dispatch_pointer_event(&group_a, "pointerup", 40, 20, 2)?;

    let (x, y) = group_translate(&group_a)?;
    check(
        x > 1.0 && y > 1.0,
        "a node clamped to the edge by one drag did not respond to a later, separate drag",
    )?;
    check_close(x, 40.0)?;
    check_close(y, 20.0)
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

    dispatch_pointer_event(&group_a, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", 500, 500, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", 500, 500, 1)?;

    let (x, _y) = group_translate(&group_a)?;
    check_close(x, 50.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `CollisionPolicy::PushClear`'s own corrective `move_node` call is clamped to `bounds` too, not just the
/// `pointermove` that preceded it. This proves the ordering in `Scene::make_draggable_with`'s own pointerup handler
/// actually holds, rather than assuming it from reading the code. It resolves the collision, *then* clamps its result.
///
/// # Expected result, worked by hand
///
/// `blocker` is `(30, 100)`, size `(80, 40)` — centre `(70, 120)`.
/// `A` starts at `(0, 100)`, size `(90, 50)` — pre-drag centre `(45, 125)`, already on `blocker`'s own west side.
///
/// Dragging `A`'s centre from `(45, 125)` to `(55, 125)` — a small, deliberate 10-unit move — lands `A`'s new rect
/// at `(10, 100)`, overlapping `blocker`. `CollisionPolicy::PushClear`'s default 6-unit padding then pushes `A`
/// back along the line from `blocker`'s centre through `A`'s own *pre-drag* centre — continuing further west, not
/// back toward where it was just dropped. That push alone would land `A`'s corrected origin at roughly
/// `(-65.9, 113.2)` — far past `bounds`'s own `x = 0` edge. With `bounds` in effect, the corrected origin clamps
/// to `x = 0`, leaving `y` (`113.18`, well inside `bounds`) untouched.
#[wasm_bindgen_test]
fn collision_pushback_near_an_edge_stays_within_bounds() -> Result<(), String> {
    let svg = make_svg("bounds-collision-pushback", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .add_node(Point::new(30.0, 100.0), Size::new(80.0, 40.0), "blocker")
        .map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(a, options).map_err(|e| e.to_string())?;

    let group_a = nth_group("bounds-collision-pushback", 1)?; // A was added second.

    // Drags A's centre (45, 125) to (55, 125): a small, deliberate move that overlaps blocker.
    dispatch_pointer_event(&group_a, "pointerdown", 45, 125, 1)?;
    dispatch_pointer_event(&group_a, "pointermove", 55, 125, 1)?;
    dispatch_pointer_event(&group_a, "pointerup", 55, 125, 1)?;

    let (x, y) = group_translate(&group_a)?;
    check_close(x, 0.0)?;
    check_close(y, 113.1767)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data node whose own rendered width exceeds `bounds`'s own width pins to `bounds`'s near edge on that axis.
/// This matches an oversized ordinary node exactly. `clamp_to_bounds` only ever looks at a node's `Rect`, never
/// at what kind of content produced it.
///
/// Four `u64` binary values, forced into 4 columns, render far wider than the deliberately narrow 100-unit
/// `bounds` used here. Each cell alone (32 space-separated binary digits) is wider than that on any reasonable
/// font. So this holds regardless of exact glyph metrics.
#[wasm_bindgen_test]
fn dragging_a_data_node_wider_than_bounds_pins_to_the_near_edge() -> Result<(), String> {
    let svg = make_svg("bounds-large-data-node", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(100.0, 260.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content =
        DataNodeContent::new(NodeValues::U64(vec![1, 2, 3, 4]), DataFormat::Binary).with_layout(GridLayout::Columns(4));
    let node = scene
        .add_data_node(Point::new(50.0, 50.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("bounds-large-data-node", 0)?;
    let rect = group
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> in the data node's group")?;
    check(
        attr_f64(&rect, "width")? > bounds.size.width,
        "expected the data node's own rendered width to exceed bounds's own width",
    )?;

    let options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(node, options).map_err(|e| e.to_string())?;

    // A modest rightward/downward move — well inside an ordinarily sized node's own clamp range. Y is not
    // oversized, so it lands wherever an ordinary drag would (50 + 20 = 70). X pins to 0 regardless of this
    // rightward push. `clamp_to_bounds`'s own clamp range collapses to the single point `bounds.origin.x` once
    // the node's own width exceeds `bounds`'s width. So no drag direction can move it off that edge.
    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", 130, 120, 1)?;
    dispatch_pointer_event(&group, "pointerup", 130, 120, 1)?;

    let (x, y) = group_translate(&group)?;
    check_close(x, 0.0)?;
    check_close(y, 70.0)
}
