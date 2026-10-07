//! `Scene::make_enterable` — a container node made clickable/keyboard-activatable.

use super::support::{FailingWrites, make_svg};
use crate::{error::Error, scene::Scene, test_support::check};
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn click(element: &web_sys::Element) {
    let event = web_sys::MouseEvent::new("click").unwrap();
    element.dispatch_event(&event).unwrap();
}

fn keydown(element: &web_sys::Element, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    element.dispatch_event(&event).unwrap();
}

/// `make_enterable` rejects a node that is not a container — there is no nested `Scene` for a click on it to enter.
#[wasm_bindgen_test]
fn make_enterable_rejects_a_non_container_node() -> Result<(), String> {
    let scene = Scene::new(make_svg("make-enterable-not-a-container")).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "Plain")
        .map_err(|e| e.to_string())?;

    check(
        matches!(scene.make_enterable(plain), Err(Error::NotAContainerNode(id)) if id == plain),
        "make_enterable on a plain node did not fail with NotAContainerNode",
    )
}

/// A second `make_enterable` call for the same node is rejected outright, the same reasoning `Error::AlreadyDraggable`
/// already documents for `make_draggable`.
#[wasm_bindgen_test]
fn make_enterable_rejects_a_second_call() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-enterable-twice-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("make-enterable-twice-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child)
        .map_err(|e| e.to_string())?;
    parent.make_enterable(node).map_err(|e| e.to_string())?;

    check(
        matches!(parent.make_enterable(node), Err(Error::AlreadyEnterable(id)) if id == node),
        "a second make_enterable call did not fail with AlreadyEnterable",
    )
}

/// Clicking an enterable container node's own rendered group enters its nested `Scene`. That is the whole point of
/// `make_enterable`: no external button, no host-written listener, just a click on the node itself.
#[wasm_bindgen_test]
fn clicking_an_enterable_container_node_enters_it() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-enterable-click-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("make-enterable-click-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;
    parent.make_enterable(node).map_err(|e| e.to_string())?;

    let group = parent.inner.borrow().node_handle(node).unwrap().group.as_element().clone();
    click(&group);

    check(
        child.is_focused(),
        "clicking the enterable container node did not enter its nested Scene",
    )?;
    check(
        !parent.is_focused(),
        "the parent was still focused after its container node was clicked",
    )
}

/// Enter/Space while an enterable container node has keyboard focus enters its nested `Scene` too, matching the same
/// pointer-or-keyboard activation `toolbar::build_button`'s own buttons already offer.
#[wasm_bindgen_test]
fn keydown_on_an_enterable_container_node_enters_it() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-enterable-keydown-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("make-enterable-keydown-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;
    parent.make_enterable(node).map_err(|e| e.to_string())?;

    let group = parent.inner.borrow().node_handle(node).unwrap().group.as_element().clone();
    keydown(&group, "Enter");

    check(
        child.is_focused(),
        "pressing Enter on the enterable container node did not enter its nested Scene",
    )
}

/// The external review that caught this found that a failed attribute write or listener registration inside
/// `make_enterable` must leave the node exactly as it was. It must not leave a container node advertising
/// `role="button"` with no working click handler behind it. It must not leave a caller's own pre-existing `style`
/// clobbered and never restored. Forces the `tabindex` write to fail, after `role` has already been written
/// successfully, so a correct rollback has real work to do beyond just removing listeners that were never reached.
#[wasm_bindgen_test]
fn a_failed_make_enterable_restores_every_attribute_it_had_already_written() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-enterable-rollback-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("make-enterable-rollback-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child)
        .map_err(|e| e.to_string())?;

    let group = parent.inner.borrow().node_handle(node).unwrap().group.as_element().clone();
    // Deliberately non-empty and unrelated to anything `make_enterable` itself writes, so a correct rollback must
    // restore *this exact value*, not merely remove whatever `make_enterable` would have written.
    group
        .set_attribute("style", "opacity: 0.5;")
        .map_err(|e| format!("could not seed a pre-existing style: {e:?}"))?;

    {
        let _failing = FailingWrites::start(&["tabindex"])?;
        check(
            parent.make_enterable(node).is_err(),
            "make_enterable with a failing tabindex write reported success",
        )?;
    }

    check(
        group.get_attribute("role").is_none(),
        "role was left behind after a failed make_enterable",
    )?;
    check(
        group.get_attribute("tabindex").is_none(),
        "tabindex was left behind after a failed make_enterable",
    )?;
    check(
        group.get_attribute("style").as_deref() == Some("opacity: 0.5;"),
        "the node's own pre-existing style was not restored after a failed make_enterable",
    )?;

    // A retry, with the injector gone, must still succeed — the failed attempt left nothing behind for a fresh one to
    // conflict with.
    parent.make_enterable(node).map_err(|e| e.to_string())?;
    check(
        group.get_attribute("role").as_deref() == Some("button"),
        "a retried make_enterable did not install role",
    )
}

/// `make_unenterable` stops a click and Enter from entering the nested `Scene`, removes the button affordances, is
/// harmless when called again, and lets the node be made enterable once more.
#[wasm_bindgen_test]
fn make_unenterable_stops_activation_and_allows_making_it_enterable_again() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-unenterable-parent")).map_err(|e| e.to_string())?;
    let child = Scene::new(make_svg("make-unenterable-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::origin(), Size::new(60.0, 40.0), "A", child.clone())
        .map_err(|e| e.to_string())?;
    parent.make_enterable(node).map_err(|e| e.to_string())?;
    parent.make_unenterable(node).map_err(|e| e.to_string())?;
    parent.make_unenterable(node).map_err(|e| e.to_string())?;

    let group = parent.inner.borrow().node_handle(node).unwrap().group.as_element().clone();
    click(&group);
    keydown(&group, "Enter");
    check(!child.is_focused(), "a node made unenterable was still entered")?;
    check(
        group.get_attribute("role").is_none() && group.get_attribute("tabindex").is_none(),
        "the button role and tab stop were left behind",
    )?;
    check(
        group.get_attribute("style").is_none_or(|style| !style.contains("pointer")),
        "the pointer cursor was left behind",
    )?;

    parent.make_enterable(node).map_err(|e| e.to_string())?;
    click(&group);
    check(child.is_focused(), "a node made enterable again was not entered")
}

/// `make_unenterable` rejects a non-container node and does nothing for one never made enterable.
#[wasm_bindgen_test]
fn make_unenterable_rejects_a_non_container_node_and_ignores_one_never_enterable() -> Result<(), String> {
    let parent = Scene::new(make_svg("make-unenterable-validation")).map_err(|e| e.to_string())?;
    let plain = parent
        .add_node(Point::origin(), Size::new(40.0, 20.0), "Plain")
        .map_err(|e| e.to_string())?;
    check(
        matches!(parent.make_unenterable(plain), Err(Error::NotAContainerNode(id)) if id == plain),
        "a plain node was not rejected",
    )?;
    let child = Scene::new(make_svg("make-unenterable-validation-child")).map_err(|e| e.to_string())?;
    let node = parent
        .add_container_node(Point::new(100.0, 0.0), Size::new(60.0, 40.0), "A", child)
        .map_err(|e| e.to_string())?;
    parent.make_unenterable(node).map_err(|e| e.to_string())
}
