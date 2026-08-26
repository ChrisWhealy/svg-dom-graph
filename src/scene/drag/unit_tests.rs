use super::*;
use svg_dom::{SvgRoot, root::utils::Size};
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

fn dispatch(element: &web_sys::Element, event_type: &str) {
    let init = web_sys::PointerEventInit::new();
    init.set_client_x(10);
    init.set_client_y(10);
    init.set_pointer_id(1);
    init.set_button(0);
    let event = web_sys::PointerEvent::new_with_event_init_dict(event_type, &init).unwrap();
    element.dispatch_event(&event).unwrap();
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an `InstallGuard` without disarming it removes every listener it covers — the state
/// `make_draggable_with` can be left in partway through installation (some, not all, of `DRAG_EVENT_TYPES`
/// registered) when a later registration fails and the `?` operator drops the guard on the way out.
#[wasm_bindgen_test]
fn dropping_an_unarmed_guard_removes_every_listener_it_covers() {
    let svg = make_svg("install-guard-rollback");
    let group = svg.group().unwrap();

    let saw_pointerdown = Rc::new(Cell::new(false));
    let saw_pointermove = Rc::new(Cell::new(false));
    {
        let flag = saw_pointerdown.clone();
        group.on_pointerdown(move |_| flag.set(true)).unwrap();
    }
    {
        let flag = saw_pointermove.clone();
        group.on_pointermove(move |_| flag.set(true)).unwrap();
    }

    drop(InstallGuard::new(group.clone()));

    dispatch(group.as_element(), "pointerdown");
    dispatch(group.as_element(), "pointermove");

    assert!(
        !saw_pointerdown.get(),
        "pointerdown listener still fired after an unarmed InstallGuard was dropped"
    );
    assert!(
        !saw_pointermove.get(),
        "pointermove listener still fired after an unarmed InstallGuard was dropped"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed guard leaves its listeners installed, so a successful
/// `make_draggable_with` call is not accidentally rolled back by its own cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_guard_leaves_its_listeners_installed() {
    let svg = make_svg("install-guard-disarm");
    let group = svg.group().unwrap();

    let saw_pointerdown = Rc::new(Cell::new(false));
    {
        let flag = saw_pointerdown.clone();
        group.on_pointerdown(move |_| flag.set(true)).unwrap();
    }

    InstallGuard::new(group.clone()).disarm();

    dispatch(group.as_element(), "pointerdown");

    assert!(
        saw_pointerdown.get(),
        "pointerdown listener was removed even though the guard was disarmed"
    );
}
