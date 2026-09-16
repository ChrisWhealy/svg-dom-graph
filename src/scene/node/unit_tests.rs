use super::*;
use crate::test_support::check;
use svg_dom::root::utils::Size;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A fresh `SvgRoot` in its own container, with a unique `id` so parallel tests in this binary do not collide.
fn make_svg(id: &str) -> SvgRoot {
    let container_id = format!("{id}-container");
    let el = document().create_element("div").unwrap();
    el.set_id(&container_id);
    document().query_selector("body").unwrap().unwrap().append_child(&el).unwrap();

    let svg = SvgRoot::create_in(&container_id, Size::new(200.0, 200.0)).unwrap();
    svg.root.set_id(id);
    svg
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed `RenderGuard` removes both `group` and every element tracked via `track` from the document.
/// `draw_box`/`draw_content_box` can be left in exactly that partial state. This happens when a later fallible step
/// fails, and the `?` operator drops the guard on the way out.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_removes_the_group_and_every_loose_element() -> Result<(), String> {
    let svg = make_svg("render-guard-rollback");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::new(group.clone());
    guard.track(loose.clone());
    drop(guard);

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed RenderGuard dropped",
    )?;
    check(
        loose.as_element().parent_node().is_none(),
        "the loosely tracked element was still attached after an armed RenderGuard dropped",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed guard leaves `group` and every tracked element attached. So a
/// successful `draw_box`/`draw_content_box` call is not accidentally rolled back by its own cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_guard_leaves_the_group_and_every_loose_element_attached() -> Result<(), String> {
    let svg = make_svg("render-guard-disarm");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::new(group.clone());
    guard.track(loose.clone());
    guard.disarm();

    check(
        group.as_element().parent_node().is_some(),
        "group was removed even though the guard was disarmed",
    )?;
    check(
        loose.as_element().parent_node().is_some(),
        "the loosely tracked element was removed even though the guard was disarmed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed guard that tracked nothing beyond `group` itself is still safe. `group.remove()` is a harmless
/// no-op on an already-empty group.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_with_no_loose_elements_only_removes_the_group() -> Result<(), String> {
    let svg = make_svg("render-guard-empty");
    let group = svg.group().unwrap();

    drop(RenderGuard::new(group.clone()));

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed, otherwise-empty RenderGuard dropped",
    )
}
