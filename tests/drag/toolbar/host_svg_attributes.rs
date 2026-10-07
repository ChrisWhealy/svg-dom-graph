//! The application's own `<svg>`. It may carry accessibility attributes of its own: a role, a name, a description, a
//! `tabindex`. The scene must never take them over, even while its keyboard control is switched on.

use super::support::*;
use crate::common::{check, dispatch_pointer_event};
use svg_dom_graph::scene::{InputMode, Side, ToolbarOptions};
use wasm_bindgen_test::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The application's `<svg>` supplies its own role, name, description, and `tabindex`. Showing the toolbar, zooming,
/// panning, changing every input mode, and hiding the toolbar must leave every one of them exactly as it was. That
/// holds not just at the end, but throughout. So no moment exists when the application's own name has been replaced.
#[wasm_bindgen_test]
fn the_applications_own_svg_attributes_are_never_touched() -> Result<(), String> {
    let scene = scene_in_svg("own-attrs", 400, 300, "0 0 400 300", None)?;
    let root = required("#own-attrs")?;
    for (name, value) in [
        ("role", "img"),
        ("tabindex", "-1"),
        ("aria-label", "SHA-256 data flow"),
        ("aria-description", "How the message schedule feeds the compression function"),
    ] {
        root.set_attribute(name, value).map_err(|e| format!("{e:?}"))?;
    }
    let before = attributes_of(&root);

    // Watch the root's own attributes for the whole sequence. Its children are not in question here.
    let observer = web_sys::MutationObserver::new(&js_sys::Function::new_no_args("")).map_err(|e| format!("{e:?}"))?;
    let init = web_sys::MutationObserverInit::new();
    init.set_attributes(true);
    observer.observe_with_options(&root, &init).map_err(|e| format!("{e:?}"))?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let surface = pan_surface("own-attrs")?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    for mode in [InputMode::On, InputMode::Off, InputMode::WithToolbar] {
        scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
        scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
    }
    scene.set_toolbar_edge(Side::South).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();

    let records = observer.take_records().length();
    check(
        records == 0,
        &format!("the scene changed the application's own <svg> attributes {records} time(s)"),
    )?;
    check(
        attributes_of(&root) == before,
        "the application's own <svg> attributes are not as they were",
    )
}
