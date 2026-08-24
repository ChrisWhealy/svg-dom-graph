//! Browser tests for the drag-to-reroute pipeline: pointerdown, pointer capture, pointermove, model update, rect
//! move, label move, edge reroute, pointerup.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state.
//! That proves the whole pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state
//! changed correctly.

mod common;

use common::{attr_f64, check, check_close, dispatch_pointer_event, make_svg, marker_ids, nth_group, the_connector};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    geometry::boundary_point,
    scene::{Scene, make_draggable},
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

    let mut scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene.add_node(a_rect.origin, a_rect.size, "A").map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let scene = Rc::new(RefCell::new(scene));
    make_draggable(&scene, b).map_err(|e| e.to_string())?;

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

    let mut scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(45.0, 25.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let scene = Rc::new(RefCell::new(scene));
    make_draggable(&scene, b).map_err(|e| e.to_string())?;

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
/// `make_draggable`'s listener closures must not keep the `Scene` alive on their own.
///
/// Registers draggable handlers, then drops the caller's only strong `Rc<RefCell<Scene>>`.
/// A `Weak` reference taken beforehand must fail to upgrade afterwards — proving nothing internal to `Scene` (a node's
/// `SvgNode`, its listener closures, and so on) still holds a strong clone of `Scene` itself, which would otherwise
/// keep the whole `Scene` alive forever, even with every external handle gone.
#[wasm_bindgen_test]
fn dropping_the_last_scene_handle_frees_the_scene() -> Result<(), String> {
    let svg = make_svg("scene-lifetime", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let mut scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(200.0, 150.0), Size::new(90.0, 50.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let scene = Rc::new(RefCell::new(scene));
    let scene_weak = Rc::downgrade(&scene);

    make_draggable(&scene, a).map_err(|e| e.to_string())?;
    make_draggable(&scene, b).map_err(|e| e.to_string())?;

    drop(scene); // the only strong handle the caller ever held

    check(
        scene_weak.upgrade().is_none(),
        "Scene was still alive after its only strong handle was dropped",
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
