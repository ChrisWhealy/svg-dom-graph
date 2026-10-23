//! `Scene::make_draggable_with_observer`: the events a drag reports, their order and their origins, what a failed write
//! sends, what dropping the handle does, and the commit order of `move_node` that the events depend on.
//!
//! This file uses only the crate's public paths, `svg_dom_graph::scene::{DragObserverHandle, DragPhase}` among them, so
//! it also shows that the new API works for an ordinary downstream caller.

use crate::common::{
    FailingWrites, check, check_close, dispatch_pointer_event, group_translate, make_svg, nth_group, path_d,
    the_connector,
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
    scene::{CollisionPolicy, ConnectorOptions, ConnectorType, DragObserverHandle, DragOptions, DragPhase, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

type Log = Rc<RefCell<Vec<(NodeId, DragPhase)>>>;

fn e(error: Error) -> String {
    error.to_string()
}

fn scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(700.0, 500.0), Size::new(700.0, 500.0))).map_err(e)
}

/// Makes `node` draggable with an observer that logs every event, and returns the log and the handle.
fn observe(scene: &Scene, node: NodeId, options: DragOptions) -> Result<(Log, DragObserverHandle), String> {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let sink = log.clone();
    let handle = scene
        .make_draggable_with_observer(node, options, move |id, phase| sink.borrow_mut().push((id, phase)))
        .map_err(e)?;
    Ok((log, handle))
}

fn phases(log: &Log) -> Vec<DragPhase> {
    log.borrow().iter().map(|(_, phase)| *phase).collect()
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}

/// Presses, moves and releases at client positions, which for these scenes are user units offset by a constant.
fn drag(group: &web_sys::Element, from: (i32, i32), to: (i32, i32)) -> Result<(), String> {
    dispatch_pointer_event(group, "pointerdown", from.0, from.1, 1)?;
    dispatch_pointer_event(group, "pointermove", to.0, to.1, 1)?;
    dispatch_pointer_event(group, "pointerup", to.0, to.1, 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_drag_reports_start_applied_and_end_in_that_order_with_the_committed_origins() -> Result<(), String> {
    let scene = scene("observer-order")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-order", 0)?;

    drag(&group, (110, 110), (140, 125))?;

    let seen = phases(&log);
    check(seen.len() == 3, &format!("{seen:?}"))?;
    check(
        matches!(seen[0], DragPhase::Start { origin } if near(origin, Point::new(100.0, 100.0))),
        &format!("{seen:?}"),
    )?;
    check(
        matches!(seen[1], DragPhase::Applied { origin } if near(origin, Point::new(130.0, 115.0))),
        &format!("{seen:?}"),
    )?;
    check(
        matches!(seen[2], DragPhase::End { origin } if near(origin, Point::new(130.0, 115.0))),
        &format!("{seen:?}"),
    )?;
    check(log.borrow().iter().all(|(id, _)| *id == node), "an event named another node")?;
    // And the node really is there.
    check(
        near(scene.node_rect(node).map_err(e)?.origin, Point::new(130.0, 115.0)),
        "node_rect",
    )
}

#[wasm_bindgen_test]
fn an_observer_can_read_the_scene_while_it_is_told_and_sees_the_move_already_committed() -> Result<(), String> {
    let scene = Rc::new(scene("observer-reentrant")?);
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let other = scene
        .add_node(Point::new(400.0, 300.0), Size::new(60.0, 30.0), "B")
        .map_err(e)?;
    let reads: Rc<RefCell<Vec<(DragPhase, Point)>>> = Rc::new(RefCell::new(Vec::new()));
    let (weak_scene, sink) = (Rc::downgrade(&scene), reads.clone());
    let handle = scene
        .make_draggable_with_observer(node, DragOptions::default(), move |id, phase| {
            // The scene is not borrowed here, so this must not panic, and may even move another node.
            let Some(scene) = weak_scene.upgrade() else { return };
            let origin = scene.node_rect(id).expect("a rect").origin;
            sink.borrow_mut().push((phase, origin));
            let _ = scene.move_node(other, Point::new(400.0, 300.0));
        })
        .map_err(e)?;
    drag(&nth_group("observer-reentrant", 0)?, (110, 110), (150, 120))?;
    let reads = reads.borrow();
    check(reads.len() == 3, &format!("{reads:?}"))?;
    for (phase, origin) in reads.iter() {
        match phase {
            DragPhase::Start { origin: o } => check(near(*o, *origin), "start")?,
            DragPhase::Applied { origin: o } | DragPhase::End { origin: o } => {
                check(near(*o, *origin), "the scene is behind the event")?
            },
            _ => return Err("an unexpected phase".to_owned()),
        }
    }
    drop(handle);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_move_whose_write_fails_sends_no_applied_and_end_reports_the_old_committed_origin() -> Result<(), String> {
    let scene = scene("observer-failed-move")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-failed-move", 0)?;
    {
        // The node's own transform cannot be written, so the move fails before anything is committed.
        let _failing = FailingWrites::on_element(&group, &["transform"], false)?;
        drag(&group, (110, 110), (150, 140))?;
    }
    let seen = phases(&log);
    check(seen.len() == 2, &format!("{seen:?}"))?;
    check(matches!(seen[0], DragPhase::Start { .. }), &format!("{seen:?}"))?;
    check(
        matches!(seen[1], DragPhase::End { origin } if near(origin, Point::new(100.0, 100.0))),
        &format!("{seen:?}"),
    )?;
    check(
        near(scene.node_rect(node).map_err(e)?.origin, Point::new(100.0, 100.0)),
        "the model moved without the page",
    )
}

#[wasm_bindgen_test]
fn a_label_that_fails_after_the_node_has_moved_sends_no_applied_but_the_move_stays_committed() -> Result<(), String> {
    let scene = scene("observer-label-failure")?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let b = scene
        .add_node(Point::new(400.0, 300.0), Size::new(90.0, 50.0), "B")
        .map_err(e)?;
    let edge = scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.set_edge_label(edge, Some("7")).map_err(e)?;
    let (log, _handle) = observe(&scene, a, DragOptions::default())?;
    let group = nth_group("observer-label-failure", 0)?;
    let label = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("a document")?
        .query_selector("#observer-label-failure > g.svg-dom-graph-content > g[data-svg-dom-graph-edge-label=\"0\"]")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no label")?;
    {
        // The node group's transform is written first and succeeds. The label's transform, written after it, fails once.
        let _failing = FailingWrites::on_element(&label, &["transform"], true)?;
        drag(&group, (110, 110), (150, 140))?;
    }
    let seen = phases(&log);
    check(
        seen.iter().all(|p| !matches!(p, DragPhase::Applied { .. })),
        &format!("{seen:?}"),
    )?;
    check(
        matches!(seen.last(), Some(DragPhase::End { origin }) if near(*origin, Point::new(140.0, 130.0))),
        &format!("End reports the committed origin: {seen:?}"),
    )?;
    check(
        near(scene.node_rect(a).map_err(e)?.origin, Point::new(140.0, 130.0)),
        "the move was rolled back",
    )?;
    check_close(group_translate(&group)?.0, 140.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_cancelled_drag_reports_the_last_committed_origin_and_drops_the_pending_position() -> Result<(), String> {
    let scene = scene("observer-cancel")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-cancel", 0)?;
    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    // The move is pending: it has not been drawn yet, so it is discarded by the cancel.
    dispatch_pointer_event(&group, "pointermove", 200, 200, 1)?;
    dispatch_pointer_event(&group, "pointercancel", 200, 200, 1)?;
    let seen = phases(&log);
    check(seen.len() == 2, &format!("{seen:?}"))?;
    check(
        matches!(seen[1], DragPhase::Cancel { origin } if near(origin, Point::new(100.0, 100.0))),
        &format!("{seen:?}"),
    )
}

#[wasm_bindgen_test]
fn a_pointer_that_is_not_the_one_dragging_is_ignored_and_sends_nothing() -> Result<(), String> {
    let scene = scene("observer-other-pointer")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-other-pointer", 0)?;
    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointerdown", 300, 300, 2)?;
    dispatch_pointer_event(&group, "pointerup", 300, 300, 2)?;
    dispatch_pointer_event(&group, "pointercancel", 300, 300, 2)?;
    check(phases(&log).len() == 1, &format!("{:?}", phases(&log)))?;
    dispatch_pointer_event(&group, "pointerup", 110, 110, 1)?;
    let seen = phases(&log);
    check(
        seen.len() == 2 && matches!(seen[1], DragPhase::End { .. }),
        &format!("{seen:?}"),
    )
}

#[wasm_bindgen_test]
fn a_push_clear_correction_is_reported_as_an_applied_move_before_the_end_and_end_has_the_corrected_origin()
-> Result<(), String> {
    let scene = scene("observer-push-clear")?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let _b = scene
        .add_node(Point::new(300.0, 100.0), Size::new(90.0, 50.0), "B")
        .map_err(e)?;
    let options = DragOptions::default().with_collision(CollisionPolicy::PushClear { padding: 6.0 });
    let (log, _handle) = observe(&scene, a, options)?;
    // Dropped right on top of B.
    drag(&nth_group("observer-push-clear", 0)?, (110, 110), (310, 110))?;
    let seen = phases(&log);
    let end = seen.last().copied();
    let corrected = scene.node_rect(a).map_err(e)?.origin;
    check(
        matches!(end, Some(DragPhase::End { origin }) if near(origin, corrected)),
        &format!("{seen:?}"),
    )?;
    // The drop was pushed back clear of B: its left edge is at 300, so A's right edge, 90 on, is 6 short of it.
    check((corrected.x + 90.0) <= 300.0 - 6.0 + 1e-6, &format!("A is at {corrected:?}"))?;
    let applied = seen.iter().filter(|p| matches!(p, DragPhase::Applied { .. })).count();
    check(
        applied >= 2,
        &format!("the drop and its correction are both reported: {seen:?}"),
    )
}

#[wasm_bindgen_test]
fn a_move_to_the_origin_the_node_already_has_is_reported_again_as_a_harmless_duplicate() -> Result<(), String> {
    let scene = scene("observer-duplicate")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-duplicate", 0)?;
    // A move to the same place: nothing to draw, but it succeeded, so it is still an Applied.
    drag(&group, (110, 110), (110, 110))?;
    let seen = phases(&log);
    check(
        seen.iter()
            .filter(|p| matches!(p, DragPhase::Applied { origin } if near(*origin, Point::new(100.0, 100.0))))
            .count()
            == 1,
        &format!("{seen:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn dropping_the_handle_ends_observation_but_leaves_the_node_draggable_and_not_re_registrable() -> Result<(), String> {
    let scene = scene("observer-drop")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let (log, handle) = observe(&scene, node, DragOptions::default())?;
    let group = nth_group("observer-drop", 0)?;
    drag(&group, (110, 110), (130, 110))?;
    let before = log.borrow().len();
    check(before == 3, &format!("{before}"))?;

    drop(handle);
    drag(&group, (110, 110), (150, 110))?;
    check(
        log.borrow().len() == before,
        "an event was delivered after the handle was dropped",
    )?;
    // The node is still draggable: it moved by the second drag, which was from where the first left it.
    check_close(group_translate(&group)?.0, 100.0 + 20.0 + 40.0)?;

    // And the listeners are still there, so it still cannot be registered again, with or without an observer.
    check(
        matches!(scene.make_draggable(node), Err(Error::AlreadyDraggable(_))),
        "make_draggable",
    )?;
    let again = scene.make_draggable_with_observer(node, DragOptions::default(), |_, _| {});
    check(matches!(again, Err(Error::AlreadyDraggable(_))), "make_draggable_with_observer")
}

#[wasm_bindgen_test]
fn dropping_the_handle_releases_what_the_callback_captured() -> Result<(), String> {
    let scene = scene("observer-captures")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let token = Rc::new(());
    let held = token.clone();
    let handle = scene
        .make_draggable_with_observer(node, DragOptions::default(), move |_, _| {
            let _ = &held;
        })
        .map_err(e)?;
    check(Rc::strong_count(&token) == 2, "the callback does not hold what it captured")?;
    drop(handle);
    check(
        Rc::strong_count(&token) == 1,
        "the scene's listeners keep the callback alive after the handle went",
    )
}

#[wasm_bindgen_test]
fn a_callback_that_drops_its_own_handle_in_the_middle_of_a_drag_finishes_and_gets_nothing_more() -> Result<(), String> {
    let scene = scene("observer-self-drop")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let holder: Rc<RefCell<Option<DragObserverHandle>>> = Rc::new(RefCell::new(None));
    let calls = Rc::new(RefCell::new(0));
    let (held, count) = (holder.clone(), calls.clone());
    let handle = scene
        .make_draggable_with_observer(node, DragOptions::default(), move |_, _| {
            *count.borrow_mut() += 1;
            held.borrow_mut().take();
        })
        .map_err(e)?;
    *holder.borrow_mut() = Some(handle);
    drag(&nth_group("observer-self-drop", 0)?, (110, 110), (130, 110))?;
    check(*calls.borrow() == 1, &format!("{}", calls.borrow()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_failed_registration_installs_nothing_makes_nothing_draggable_and_returns_no_handle() -> Result<(), String> {
    let scene = scene("observer-registration")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let other = Scene::new(make_svg(
        "observer-registration-other",
        Size::new(400.0, 300.0),
        Size::new(400.0, 300.0),
    ))
    .map_err(e)?;
    let foreign = other.add_node(Point::new(0.0, 0.0), Size::new(40.0, 30.0), "F").map_err(e)?;

    check(
        matches!(
            scene.make_draggable_with_observer(foreign, DragOptions::default(), |_, _| {}),
            Err(Error::UnknownNode(_))
        ),
        "a foreign node",
    )?;
    let bad = DragOptions::default().with_collision(CollisionPolicy::PushClear { padding: -1.0 });
    check(
        matches!(
            scene.make_draggable_with_observer(node, bad, |_, _| {}),
            Err(Error::InvalidCollisionPadding(_))
        ),
        "bad padding",
    )?;
    let bounds = svg_dom::root::utils::Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(-1.0, 5.0),
    };
    let bad = DragOptions::default().with_bounds(Some(bounds));
    check(
        matches!(
            scene.make_draggable_with_observer(node, bad, |_, _| {}),
            Err(Error::InvalidDragBounds(_))
        ),
        "bad bounds",
    )?;

    // None of those left the node draggable: it can still be made draggable, with an observer, once.
    let (log, _handle) = observe(&scene, node, DragOptions::default())?;
    drag(&nth_group("observer-registration", 0)?, (110, 110), (130, 110))?;
    check(
        log.borrow().len() == 3,
        "the observer registered after the failures did not work",
    )
}

#[wasm_bindgen_test]
fn an_unobserved_drag_and_an_observed_one_move_a_node_in_exactly_the_same_way() -> Result<(), String> {
    let scene = scene("observer-same-drag")?;
    let plain = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let watched = scene
        .add_node(Point::new(300.0, 100.0), Size::new(90.0, 50.0), "B")
        .map_err(e)?;
    scene.make_draggable(plain).map_err(e)?;
    let (_log, _handle) = observe(&scene, watched, DragOptions::default())?;
    drag(&nth_group("observer-same-drag", 0)?, (110, 110), (150, 140))?;
    drag(&nth_group("observer-same-drag", 1)?, (310, 110), (350, 140))?;
    let (a, b) = (
        scene.node_rect(plain).map_err(e)?.origin,
        scene.node_rect(watched).map_err(e)?.origin,
    );
    check(near(Point::new(b.x - 200.0, b.y), a), &format!("{a:?} and {b:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The commit order of `move_node`, which the events rely on.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[wasm_bindgen_test]
fn move_node_leaves_the_model_where_it_was_when_the_node_transform_cannot_be_written() -> Result<(), String> {
    let scene = scene("commit-node-transform")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let group = nth_group("commit-node-transform", 0)?;
    {
        let _failing = FailingWrites::on_element(&group, &["transform"], false)?;
        check(
            scene.move_node(node, Point::new(200.0, 200.0)).is_err(),
            "the failure was not reported",
        )?;
    }
    check(
        near(scene.node_rect(node).map_err(e)?.origin, Point::new(100.0, 100.0)),
        "node_rect describes a move that was never drawn",
    )?;
    check_close(group_translate(&group)?.0, 100.0)?;
    // And a retry now works from the same state.
    scene.move_node(node, Point::new(200.0, 200.0)).map_err(e)?;
    check(
        near(scene.node_rect(node).map_err(e)?.origin, Point::new(200.0, 200.0)),
        "retry",
    )
}

#[wasm_bindgen_test]
fn move_node_keeps_the_move_when_a_label_fails_afterwards_and_a_later_move_puts_the_connector_right()
-> Result<(), String> {
    let scene = scene("commit-label")?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let b = scene
        .add_node(Point::new(400.0, 300.0), Size::new(90.0, 50.0), "B")
        .map_err(e)?;
    let edge = scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.set_edge_label(edge, Some("7")).map_err(e)?;
    let label = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("a document")?
        .query_selector("#commit-label > g.svg-dom-graph-content > g[data-svg-dom-graph-edge-label=\"0\"]")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no label")?;
    {
        let _failing = FailingWrites::on_element(&label, &["transform"], true)?;
        check(
            scene.move_node(a, Point::new(150.0, 120.0)).is_err(),
            "the label failure was not reported",
        )?;
    }
    // Not rolled back: the node is where it was moved to, in the model and on the page.
    check(
        near(scene.node_rect(a).map_err(e)?.origin, Point::new(150.0, 120.0)),
        "the node move was rolled back",
    )?;
    check_close(group_translate(&nth_group("commit-label", 0)?)?.0, 150.0)?;
    // The connector was already redrawn before the label was placed.
    let after_failure = path_d(&the_connector("commit-label")?)?;
    // A move to a different origin recomputes the connector and places the label again.
    scene.move_node(a, Point::new(170.0, 140.0)).map_err(e)?;
    let after_move = path_d(&the_connector("commit-label")?)?;
    check(after_move != after_failure, "the next real move did not reroute the connector")
}

#[wasm_bindgen_test]
fn move_node_to_the_same_origin_retries_a_failed_label_and_does_not_promise_to_reroute() -> Result<(), String> {
    let scene = scene("commit-same-origin")?;
    let a = scene
        .add_node(Point::new(100.0, 100.0), Size::new(90.0, 50.0), "A")
        .map_err(e)?;
    let b = scene
        .add_node(Point::new(400.0, 300.0), Size::new(90.0, 50.0), "B")
        .map_err(e)?;
    let edge = scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.set_edge_label(edge, Some("7")).map_err(e)?;
    let label = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("a document")?
        .query_selector("#commit-same-origin > g.svg-dom-graph-content > g[data-svg-dom-graph-edge-label=\"0\"]")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no label")?;
    let before = label.get_attribute("transform");
    {
        let _failing = FailingWrites::on_element(&label, &["transform"], true)?;
        let _ = scene.move_node(a, Point::new(200.0, 150.0));
    }
    check(
        label.get_attribute("transform") == before,
        "the label moved although its write failed",
    )?;
    // The same origin again: nothing to draw for the node, but the dirty label is tried again.
    scene.move_node(a, Point::new(200.0, 150.0)).map_err(e)?;
    check(label.get_attribute("transform") != before, "the dirty label was not retried")
}
