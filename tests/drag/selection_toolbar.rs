//! Browser tests for `Scene`'s selection toolbar: showing, hiding, Prev/Next/Restart by click and by keyboard,
//! disabled-button no-ops, an empty data node, reentrancy into the same `Scene` from `on_step`, and the callback's
//! own lifetime once the toolbar is hidden.

use super::common::*;
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
    scene::{
        DataFormat, DataNodeContent, NodeValues, Scene, Selection, SelectionStride, SelectionToolbarOptions,
        SelectionTransition,
    },
};
use wasm_bindgen_test::*;

fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(400.0, 300.0), Size::new(400.0, 300.0))).map_err(|e| e.to_string())
}

fn query(selector: &str) -> Result<Option<web_sys::Element>, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector(selector)
        .map_err(|e| format!("{e:?}"))
}

fn required(selector: &str) -> Result<web_sys::Element, String> {
    query(selector)?.ok_or_else(|| format!("nothing matches {selector}"))
}

fn bar(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > [role=\"toolbar\"][aria-label=\"Selection controls\"]"))
}

fn bar_count(id: &str) -> Result<u32, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all(&format!("#{id} > [role=\"toolbar\"][aria-label=\"Selection controls\"]"))
        .map_err(|e| format!("{e:?}"))
        .map(|list| list.length())
}

/// The `n`th selection toolbar button: 0 is Prev, 1 is Next, 2 is Restart.
fn button(id: &str, n: usize) -> Result<web_sys::Element, String> {
    required(&format!(
        "#{id} > [role=\"toolbar\"][aria-label=\"Selection controls\"] > [role=\"button\"]:nth-child({})",
        n + 1
    ))
}

fn attr(element: &web_sys::Element, name: &str) -> Result<String, String> {
    element.get_attribute(name).ok_or_else(|| format!("missing attribute {name}"))
}

fn click(element: &web_sys::Element) -> Result<(), String> {
    let event = web_sys::MouseEvent::new("click").map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

fn keydown(element: &web_sys::Element, key: &str) -> Result<(), String> {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

fn add_four_values(scene: &Scene) -> Result<NodeId, String> {
    scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![10, 20, 30, 40]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())
}

fn no_op(_: &Scene, _: NodeId, _: SelectionTransition) {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn there_is_no_selection_toolbar_until_one_is_shown_and_none_after_it_is_hidden() -> Result<(), String> {
    let scene = new_scene("st-show-hide")?;
    let node = add_four_values(&scene)?;
    check(!scene.has_selection_toolbar(), "a new scene already has a selection toolbar")?;

    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), no_op)
        .map_err(|e| e.to_string())?;
    check(scene.has_selection_toolbar(), "has_selection_toolbar is false after showing it")?;
    bar("st-show-hide")?;

    scene.hide_selection_toolbar();
    check(!scene.has_selection_toolbar(), "has_selection_toolbar is true after hiding it")?;
    check(
        query("#st-show-hide > [role=\"toolbar\"][aria-label=\"Selection controls\"]")?.is_none(),
        "the bar is still in the DOM after hide_selection_toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ordinary case: a freshly built data node's own `Selection` is already `Selection::None` before
/// `show_selection_toolbar` ever runs, so its own commit (`set_selection(node, Selection::None)`) is a same-as-
/// current no-op — it must still sync every button's own `aria-disabled`, not skip it because nothing needed
/// recolouring. Reproduces a real regression: `set_selection`'s own no-op fast path used to return before ever
/// reaching that sync, so a toolbar installed against an already-unstarted node drew every button with no
/// `aria-disabled` attribute at all.
#[wasm_bindgen_test]
fn showing_it_on_an_already_unstarted_node_still_syncs_every_buttons_disabled_state() -> Result<(), String> {
    let scene = new_scene("st-fresh-node")?;
    let node = add_four_values(&scene)?;

    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), no_op)
        .map_err(|e| e.to_string())?;

    check(
        attr(&button("st-fresh-node", 0)?, "aria-disabled")? == "true",
        "Prev has no synced aria-disabled after showing the toolbar on an already-unstarted node",
    )?;
    check(
        attr(&button("st-fresh-node", 1)?, "aria-disabled")? == "false",
        "Next has no synced aria-disabled after showing the toolbar on an already-unstarted node",
    )?;
    check(
        attr(&button("st-fresh-node", 2)?, "aria-disabled")? == "true",
        "Restart has no synced aria-disabled after showing the toolbar on an already-unstarted node",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn showing_it_on_a_plain_label_node_is_rejected_and_installs_nothing() -> Result<(), String> {
    let scene = new_scene("st-label-node")?;
    let node = scene
        .add_node(Point::new(10.0, 10.0), Size::new(80.0, 40.0), "label")
        .map_err(|e| e.to_string())?;

    let result = scene.show_selection_toolbar(node, SelectionToolbarOptions::default(), no_op);
    check(
        matches!(result, Err(Error::InvalidSelection(id, Selection::None)) if id == node),
        &format!("expected Err(InvalidSelection), got {result:?}"),
    )?;
    check(!scene.has_selection_toolbar(), "a toolbar was installed despite the rejection")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn invalid_options_are_rejected_and_leave_any_existing_toolbar_unchanged() -> Result<(), String> {
    let scene = new_scene("st-invalid-options")?;
    let node = add_four_values(&scene)?;
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), no_op)
        .map_err(|e| e.to_string())?;

    let mut bad = SelectionToolbarOptions::default();
    bad.button_height = -1.0;
    let result = scene.show_selection_toolbar(node, bad, no_op);
    check(
        matches!(result, Err(Error::InvalidSelectionToolbarOptions(_))),
        &format!("expected Err(InvalidSelectionToolbarOptions), got {result:?}"),
    )?;
    check(
        scene.has_selection_toolbar(),
        "the existing toolbar was removed by the rejected call",
    )?;
    bar("st-invalid-options").map(|_| ())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing a second selection toolbar removes the first's own bar from the DOM — not just from
/// `SceneInner::selection_toolbar` — drops the first's own `on_step` along with it, and leaves only the second one
/// responding to activation.
#[wasm_bindgen_test]
fn showing_a_second_selection_toolbar_replaces_the_first_in_the_dom_and_drops_its_callback() -> Result<(), String> {
    let scene = new_scene("st-replace")?;
    let node_a = add_four_values(&scene)?;
    let node_b = add_four_values(&scene)?;

    let sentinel_a = Rc::new(());
    let weak_a = Rc::downgrade(&sentinel_a);
    scene
        .show_selection_toolbar(node_a, SelectionToolbarOptions::default(), move |_, _, _| {
            // Captured only so `sentinel_a`'s own refcount reflects this closure's lifetime. Never actually called.
            let _ = &sentinel_a;
        })
        .map_err(|e| e.to_string())?;

    let seen_b: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen_b.clone();
    scene
        .show_selection_toolbar(node_b, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    check(
        bar_count("st-replace")? == 1,
        "more than one selection toolbar bar remains in the DOM after replacing it",
    )?;
    check(
        weak_a.upgrade().is_none(),
        "the first toolbar's on_step outlived being replaced by the second",
    )?;

    // Only the second toolbar's own button now exists at all, and it responds.
    click(&button("st-replace", 1)?)?;
    check(
        seen_b.borrow().len() == 1,
        &format!("the second toolbar's own on_step did not fire: {:?}", seen_b.borrow()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing the toolbar resets the managed node to the unstarted state regardless of whatever `Selection` it already
/// held — observed indirectly here, through Prev/Restart both starting out disabled (which could only be true if
/// the node is genuinely unstarted, not still at cell 2).
#[wasm_bindgen_test]
fn showing_it_resets_the_managed_node_to_unstarted_regardless_of_prior_selection() -> Result<(), String> {
    let scene = new_scene("st-resets-on-show")?;
    let node = add_four_values(&scene)?;
    scene.set_selection(node, Selection::Cell(2)).map_err(|e| e.to_string())?;

    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), no_op)
        .map_err(|e| e.to_string())?;

    check(
        attr(&button("st-resets-on-show", 0)?, "aria-disabled")? == "true",
        "Prev is enabled right after showing the toolbar",
    )?;
    check(
        attr(&button("st-resets-on-show", 2)?, "aria-disabled")? == "true",
        "Restart is enabled right after showing the toolbar",
    )?;
    check(
        attr(&button("st-resets-on-show", 1)?, "aria-disabled")? == "false",
        "Next is disabled right after showing the toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn clicking_next_from_unstarted_selects_element_zero_and_calls_on_step() -> Result<(), String> {
    let scene = new_scene("st-next-from-unstarted")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    click(&button("st-next-from-unstarted", 1)?)?;

    check(
        seen.borrow().len() == 1,
        &format!("expected 1 transition, got {:?}", seen.borrow()),
    )?;
    check(
        seen.borrow()[0] == SelectionTransition { from: None, to: Some(0) },
        &format!("{:?}", seen.borrow()[0]),
    )?;
    check(
        attr(&button("st-next-from-unstarted", 0)?, "aria-disabled")? == "false",
        "Prev is still disabled after the first Next",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn clicking_next_at_the_last_element_is_disabled_and_does_not_call_on_step() -> Result<(), String> {
    let scene = new_scene("st-next-at-end")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    // 4 values: Next four times reaches index 3, the last element.
    for _ in 0..4 {
        click(&button("st-next-at-end", 1)?)?;
    }
    check(
        seen.borrow().len() == 4,
        &format!("expected 4 transitions, got {:?}", seen.borrow()),
    )?;
    check(
        attr(&button("st-next-at-end", 1)?, "aria-disabled")? == "true",
        "Next is not disabled at the last element",
    )?;

    // A further click changes nothing and does not call `on_step` again.
    click(&button("st-next-at-end", 1)?)?;
    check(
        seen.borrow().len() == 4,
        &format!("a disabled Next still called on_step: {:?}", seen.borrow()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn clicking_prev_from_unstarted_is_disabled_and_does_not_call_on_step() -> Result<(), String> {
    let scene = new_scene("st-prev-from-unstarted")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    click(&button("st-prev-from-unstarted", 0)?)?;
    check(
        seen.borrow().is_empty(),
        &format!("a disabled Prev called on_step: {:?}", seen.borrow()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn prev_after_next_returns_to_unstarted() -> Result<(), String> {
    let scene = new_scene("st-prev-after-next")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    click(&button("st-prev-after-next", 1)?)?; // Next: None -> Some(0)
    click(&button("st-prev-after-next", 0)?)?; // Prev: Some(0) -> None

    check(seen.borrow().len() == 2, &format!("{:?}", seen.borrow()))?;
    check(
        seen.borrow()[1] == SelectionTransition { from: Some(0), to: None },
        &format!("{:?}", seen.borrow()[1]),
    )?;
    check(
        attr(&button("st-prev-after-next", 0)?, "aria-disabled")? == "true",
        "Prev is still enabled once back at unstarted",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Restart` from any already-started element jumps straight back to unstarted, in one step — never one `Prev` per
/// element walked.
#[wasm_bindgen_test]
fn restart_jumps_straight_to_unstarted_from_any_element() -> Result<(), String> {
    let scene = new_scene("st-restart")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    click(&button("st-restart", 1)?)?; // -> Some(0)
    click(&button("st-restart", 1)?)?; // -> Some(1)
    click(&button("st-restart", 1)?)?; // -> Some(2)
    click(&button("st-restart", 2)?)?; // Restart -> None

    check(seen.borrow().len() == 4, &format!("{:?}", seen.borrow()))?;
    check(
        seen.borrow()[3] == SelectionTransition { from: Some(2), to: None },
        &format!("{:?}", seen.borrow()[3]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn keyboard_activation_works_the_same_as_a_click() -> Result<(), String> {
    let scene = new_scene("st-keyboard")?;
    let node = add_four_values(&scene)?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, t| {
            recorder.borrow_mut().push(t);
        })
        .map_err(|e| e.to_string())?;

    keydown(&button("st-keyboard", 1)?, "Enter")?;
    keydown(&button("st-keyboard", 1)?, " ")?;

    check(seen.borrow().len() == 2, &format!("{:?}", seen.borrow()))?;
    check(
        seen.borrow()[1] == SelectionTransition { from: Some(0), to: Some(1) },
        &format!("{:?}", seen.borrow()[1]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `on_step` is explicitly meant to reenter this crate. This drives that directly: the callback itself calls
/// `scene.set_selection` on a second node — if the click handler still held any part of `SceneInner` borrowed while
/// calling `on_step`, this would panic on a re-entrant `RefCell` borrow instead of succeeding.
#[wasm_bindgen_test]
fn on_step_can_reenter_the_same_scene_without_panicking() -> Result<(), String> {
    let scene = new_scene("st-reentrant")?;
    let node = add_four_values(&scene)?;
    let other = add_four_values(&scene)?;

    let reentered: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let flag = reentered.clone();
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |scene, _, _| {
            // Reenters via a different Scene handle, and a different node, but the same underlying `SceneInner`.
            let _ = scene.set_selection(other, Selection::Cell(1));
            *flag.borrow_mut() = true;
        })
        .map_err(|e| e.to_string())?;

    click(&button("st-reentrant", 1)?)?;
    check(*reentered.borrow(), "on_step never ran")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `on_step` is owned only by the three buttons' own DOM closures, never by `SceneInner` itself — so once
/// `hide_selection_toolbar` removes them, the callback becomes droppable as soon as nothing else in the host's own
/// code still holds a reference into it.
#[wasm_bindgen_test]
fn hiding_the_toolbar_drops_the_callback() -> Result<(), String> {
    let scene = new_scene("st-drop-callback")?;
    let node = add_four_values(&scene)?;

    let sentinel = Rc::new(());
    let weak_sentinel = Rc::downgrade(&sentinel);
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_, _, _| {
            // Captured only so `sentinel`'s own refcount reflects this closure's lifetime. Never actually called.
            let _ = &sentinel;
        })
        .map_err(|e| e.to_string())?;

    check(
        weak_sentinel.upgrade().is_some(),
        "the sentinel was already dropped before hide_selection_toolbar ran",
    )?;

    scene.hide_selection_toolbar();

    check(
        weak_sentinel.upgrade().is_none(),
        "on_step (and its captured sentinel) outlived hide_selection_toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With a stride, the bar holds five buttons in order — `Prev Round`, `Prev`, `Next`, `Next Round`, `Restart` — each
/// with its own accessible name; without one it still holds the original three.
#[wasm_bindgen_test]
fn a_stride_adds_two_buttons_in_the_expected_order_with_their_own_names() -> Result<(), String> {
    let scene = new_scene("st-stride-buttons")?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((0..20).collect()), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let mut options = SelectionToolbarOptions::default();
    options.stride = Some(SelectionStride::new(5, "Round"));
    scene.show_selection_toolbar(node, options, no_op).map_err(|e| e.to_string())?;

    let names: Vec<String> = (0..5)
        .map(|n| attr(&button("st-stride-buttons", n)?, "aria-label"))
        .collect::<Result<_, _>>()?;
    check(
        names
            == [
                "Previous round",
                "Previous selection",
                "Next selection",
                "Next round",
                "Restart selection",
            ],
        &format!("{names:?}"),
    )?;
    let labels: Vec<String> = (0..5)
        .map(|n| button("st-stride-buttons", n).map(|b| b.text_content().unwrap_or_default()))
        .collect::<Result<_, _>>()?;
    check(
        labels == ["Prev Round", "Prev", "Next", "Next Round", "Restart"],
        &format!("{labels:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Next Round` and `Prev Round` move five cells at a time, report a transition for each, clamp at the ends, and are
/// disabled when they could not move.
#[wasm_bindgen_test]
fn the_stride_buttons_move_a_whole_group_and_are_disabled_at_the_ends() -> Result<(), String> {
    let scene = new_scene("st-stride-walk")?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((0..12).collect()), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let seen: Rc<RefCell<Vec<SelectionTransition>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    let mut options = SelectionToolbarOptions::default();
    options.stride = Some(SelectionStride::new(5, "Round"));
    scene
        .show_selection_toolbar(node, options, move |_, _, t| recorder.borrow_mut().push(t))
        .map_err(|e| e.to_string())?;
    let (prev_round, next, next_round) = (0, 2, 3);

    check(
        attr(&button("st-stride-walk", prev_round)?, "aria-disabled")? == "true",
        "Prev Round starts disabled",
    )?;
    click(&button("st-stride-walk", next)?)?;
    click(&button("st-stride-walk", next_round)?)?;
    click(&button("st-stride-walk", next_round)?)?;
    // 0 -> 5 -> 10, then the last group is short: 10 + 5 clamps to 11.
    click(&button("st-stride-walk", next_round)?)?;
    check(
        attr(&button("st-stride-walk", next_round)?, "aria-disabled")? == "true",
        "Next Round is disabled on the last cell",
    )?;
    click(&button("st-stride-walk", next_round)?)?;
    let tos: Vec<_> = seen.borrow().iter().map(|t| t.to).collect();
    check(
        tos == [Some(0), Some(5), Some(10), Some(11)],
        &format!("a disabled Next Round must not call on_step; saw {tos:?}"),
    )?;

    click(&button("st-stride-walk", prev_round)?)?;
    click(&button("st-stride-walk", prev_round)?)?;
    click(&button("st-stride-walk", prev_round)?)?;
    let tos: Vec<_> = seen.borrow().iter().map(|t| t.to).collect();
    // 11 -> 6 -> 1 -> 0 (clamped, never the unstarted state).
    check(
        tos[4..] == [Some(6), Some(1), Some(0)],
        &format!("Prev Round walked {:?}", &tos[4..]),
    )?;
    check(
        attr(&button("st-stride-walk", prev_round)?, "aria-disabled")? == "true",
        "Prev Round is disabled at cell 0 and never goes back to unstarted",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A stride of `0` is rejected like any other invalid option.
#[wasm_bindgen_test]
fn a_zero_stride_is_rejected() -> Result<(), String> {
    let scene = new_scene("st-stride-zero")?;
    let node = add_four_values(&scene)?;
    let mut options = SelectionToolbarOptions::default();
    options.stride = Some(SelectionStride::new(0, "Round"));
    let result = scene.show_selection_toolbar(node, options, no_op);
    check(
        matches!(result, Err(Error::InvalidSelectionToolbarOptions(_))),
        &format!("{result:?}"),
    )?;
    check(!scene.has_selection_toolbar(), "nothing was installed")
}
