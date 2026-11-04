use super::*;

const VIEWPORT: (f64, f64) = (1800.0, 880.0);

fn layout() -> Layout {
    Layout::build(101.3, 91.8)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_scene_point_is_on_the_screen_where_the_scale_and_pan_put_it() {
    let c = Camera { scale: 2.0, tx: 10.0, ty: -4.0 };
    assert_eq!(c.to_screen((3.0, 5.0)), (16.0, 6.0));
}

#[test]
fn centring_puts_the_point_in_the_middle_and_keeps_the_scale() {
    let c = Camera { scale: 0.9, tx: 0.0, ty: 0.0 }.centred_on((500.0, 300.0), VIEWPORT);
    assert_eq!(c.scale, 0.9);
    let (x, y) = c.to_screen((500.0, 300.0));
    assert!(close(x, 900.0) && close(y, 440.0));
}

#[test]
fn the_page_opens_with_the_whole_picture_in_view_scaled_down_at_the_left() {
    let layout = layout();
    let c = initial(&layout, VIEWPORT);
    assert!(c.scale < 1.0 && c.scale >= MIN_SCALE, "scale {}", c.scale);
    let (left, top) = c.to_screen((0.0, 0.0));
    let (right, bottom) = c.to_screen(layout.size);
    assert!(left >= 0.0 && top >= 0.0 && right <= VIEWPORT.0 && bottom <= VIEWPORT.1);
    assert!(close(left, LEFT_MARGIN), "the left of the picture is at the left of the view");
    assert!(close(top, VIEWPORT.1 - bottom), "it is in the middle of the height");
    for v in 0..64 {
        assert_eq!(visibility(&c, VIEWPORT, &layout, v), Visibility::Inside, "vertex {v}");
    }
}

#[test]
fn a_viewport_that_is_larger_than_the_picture_shows_it_at_scale_one_and_no_larger() {
    let c = initial(&layout(), (9000.0, 9000.0));
    assert_eq!(c.scale, 1.0);
}

#[test]
fn a_picture_too_large_for_the_smallest_scale_is_shown_at_that_scale() {
    let c = initial(&layout(), (300.0, 200.0));
    assert_eq!(c.scale, MIN_SCALE);
}

#[test]
fn visibility_has_three_states_and_the_edge_of_the_safe_rectangle_counts_as_inside() {
    let layout = layout();
    let (x, y) = layout.centre(0);
    let (left, top, ..) = safe_rect(VIEWPORT);
    let on_edge = Camera {
        scale: 1.0,
        tx: left - x,
        ty: 400.0 - y,
    };
    assert_eq!(visibility(&on_edge, VIEWPORT, &layout, 0), Visibility::Inside);
    let near = Camera {
        scale: 1.0,
        tx: left - x - 5.0,
        ty: 400.0 - y,
    };
    assert_eq!(visibility(&near, VIEWPORT, &layout, 0), Visibility::NearEdge);
    let off = Camera {
        scale: 1.0,
        tx: -x - 50.0,
        ty: top - y,
    };
    assert_eq!(visibility(&off, VIEWPORT, &layout, 0), Visibility::OffScreen);
}

#[test]
fn following_does_nothing_for_a_vertex_that_is_well_placed_or_for_no_vertex() {
    let layout = layout();
    let c = initial(&layout, VIEWPORT);
    assert_eq!(follow(&c, VIEWPORT, &layout, Some(5)), None);
    assert_eq!(follow(&c, VIEWPORT, &layout, None), None);
}

#[test]
fn following_pans_by_the_least_for_a_vertex_near_the_edge_and_never_zooms() {
    let layout = layout();
    let (x, y) = layout.centre(3);
    let (left, ..) = safe_rect(VIEWPORT);
    let c = Camera {
        scale: 0.9,
        tx: left * 0.5 - 0.9 * x,
        ty: 440.0 - 0.9 * y,
    };
    assert_eq!(visibility(&c, VIEWPORT, &layout, 3), Visibility::NearEdge);
    let plan = follow(&c, VIEWPORT, &layout, Some(3)).unwrap();
    assert_eq!(plan.camera.scale, 0.9);
    assert_eq!(plan.camera.ty, c.ty, "only the axis that was out of place moved");
    assert_eq!(visibility(&plan.camera, VIEWPORT, &layout, 3), Visibility::Inside);
    let (sx, _) = plan.camera.to_screen((x, y));
    assert!(close(sx, left), "it stops at the safe edge");
}

#[test]
fn following_puts_a_vertex_that_is_off_the_screen_in_the_middle() {
    let layout = layout();
    let c = Camera {
        scale: 1.0,
        tx: -9000.0,
        ty: 0.0,
    };
    assert_eq!(visibility(&c, VIEWPORT, &layout, 20), Visibility::OffScreen);
    let plan = follow(&c, VIEWPORT, &layout, Some(20)).unwrap();
    let (x, y) = plan.camera.to_screen(layout.centre(20));
    assert!(close(x, VIEWPORT.0 / 2.0) && close(y, VIEWPORT.1 / 2.0));
}

#[test]
fn a_jump_to_a_component_puts_the_middle_of_its_vertices_in_the_middle_and_keeps_the_zoom() {
    let layout = layout();
    let c = Camera { scale: 0.7, tx: 12.0, ty: 34.0 };
    let members = [3, 17, 40, 41];
    let plan = destination(&c, VIEWPORT, &layout, &members).unwrap();
    assert_eq!(plan.camera.scale, 0.7);
    let bounds = layout.bounds_of(&members, 0.0).unwrap();
    let (x, y) = plan.camera.to_screen(bounds.centre());
    assert!(close(x, VIEWPORT.0 / 2.0) && close(y, VIEWPORT.1 / 2.0));
    assert_eq!(destination(&c, VIEWPORT, &layout, &[]), None);
}

#[test]
fn zooming_to_a_vertex_puts_it_in_the_middle_at_the_scale_that_is_easy_to_read() {
    let layout = layout();
    let plan = zoom_to(VIEWPORT, &layout, 30);
    assert_eq!(plan.camera.scale, MAX_FIT);
    let (x, y) = plan.camera.to_screen(layout.centre(30));
    assert!(close(x, VIEWPORT.0 / 2.0) && close(y, VIEWPORT.1 / 2.0));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn memory() -> Memory {
    Memory::new(Camera { scale: 1.0, tx: 5.0, ty: 5.0 })
}

#[test]
fn an_unchanged_view_changes_nothing() {
    let mut m = memory();
    assert_eq!(m.reconcile(m.acknowledged), Reconciled::Unchanged);
    assert!(m.follow);
}

#[test]
fn a_view_that_a_person_changed_turns_following_off_once() {
    let mut m = memory();
    let moved = Camera { tx: 99.0, ..m.acknowledged };
    assert_eq!(m.reconcile(moved), Reconciled::FollowTurnedOff);
    assert!(!m.follow);
    assert_eq!(m.acknowledged, moved);
    assert_eq!(m.reconcile(moved), Reconciled::Unchanged, "the new view is the baseline");
    assert_eq!(
        m.reconcile(Camera { ty: 1.0, ..moved }),
        Reconciled::Changed,
        "a second change finds it already off"
    );
}

#[test]
fn a_view_that_the_controller_wrote_never_turns_following_off() {
    let mut m = memory();
    let plan = Plan {
        camera: Camera { scale: 1.0, tx: 500.0, ty: 5.0 },
    };
    m.committed(plan);
    assert_eq!(m.reconcile(plan.camera), Reconciled::Unchanged);
    assert!(m.follow);
}

#[test]
fn turning_following_back_on_is_for_the_person_only() {
    let mut m = memory();
    m.reconcile(Camera { tx: 1.0, ..m.acknowledged });
    m.committed(Plan {
        camera: Camera { scale: 1.0, tx: 2.0, ty: 5.0 },
    });
    assert!(!m.follow, "a pan by the controller does not turn it back on");
}
