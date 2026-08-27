//! Dropping a dragged node so it overlaps another: `CollisionPolicy::PushClear`/`Allow`, degenerate cases, and the
//! equidistant-blockers tie-break.

use crate::common::{attr_f64, check_close, dispatch_pointer_event, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{CollisionPolicy, DragOptions, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

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
