//! Pan and wheel zoom are independent of the toolbar. Each has its own `InputMode`: `WithToolbar` (the default,
//! which follows the toolbar), `On`, or `Off`.

use super::support::*;
use crate::common::{attr_f64, check, check_close, dispatch_pointer_event, group_translate, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{
    DataFormat, DataNodeContent, GridLayout, InputMode, NodeValues, Scene, Selection, Side, ToolbarOptions,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

/// Ctrl+wheel one notch up, then reports whether the scene zoomed. Uses a bounding-box-relative pointer position.
fn ctrl_wheel_zooms(scene: &Scene, id: &str) -> Result<bool, String> {
    let before = scene.zoom_scale();
    let target = query(&format!("#{id} > rect[aria-hidden=\"true\"]"))?.unwrap_or(required(&format!("#{id}"))?);
    wheel(&target, 100, 100, -100.0, true, false)?;
    Ok((scene.zoom_scale() - before).abs() > 1e-9)
}

/// Drags the background 40 pixels right, then reports whether the content moved. Needs a pan surface to exist.
fn background_drag_pans(id: &str) -> Result<bool, String> {
    let surface = match query(&format!("#{id} > rect[aria-hidden=\"true\"]"))? {
        Some(surface) => surface,
        None => return Ok(false),
    };
    let content = content(id)?;
    let before = content.get_attribute("transform");
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    Ok(content.get_attribute("transform") != before)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The default follows the toolbar exactly as before: off with no toolbar, on while one is shown, off again once hidden.
#[wasm_bindgen_test]
fn by_default_both_gestures_follow_the_toolbar() -> Result<(), String> {
    let scene = new_scene("im-default")?;
    check(
        scene.pan_mode() == InputMode::WithToolbar,
        "pan does not default to WithToolbar",
    )?;
    check(
        scene.wheel_zoom_mode() == InputMode::WithToolbar,
        "wheel zoom does not default to WithToolbar",
    )?;
    check(
        !scene.pan_enabled() && !scene.wheel_zoom_enabled(),
        "a gesture is active with no toolbar",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "showing the toolbar did not activate the gestures",
    )?;

    scene.hide_toolbar();
    check(
        !scene.pan_enabled() && !scene.wheel_zoom_enabled(),
        "hiding the toolbar left a gesture active",
    )?;
    check(
        query("#im-default > rect[aria-hidden=\"true\"]")?.is_none(),
        "the surface outlived the toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The motivating case: an application supplies its own controls, hides the stock toolbar, and still wants both
/// gestures.
#[wasm_bindgen_test]
fn both_gestures_can_be_forced_on_with_no_toolbar_at_all() -> Result<(), String> {
    let scene = new_scene("im-own-controls")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;

    check(!scene.has_toolbar(), "test setup: a toolbar is shown")?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "a forced gesture is not enabled",
    )?;
    check(background_drag_pans("im-own-controls")?, "dragging the background did not pan")?;
    check(ctrl_wheel_zooms(&scene, "im-own-controls")?, "ctrl+wheel did not zoom")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A forced-on gesture survives the toolbar being shown and hidden, while one following the toolbar does not.
#[wasm_bindgen_test]
fn a_forced_gesture_survives_hiding_the_toolbar_and_a_following_one_does_not() -> Result<(), String> {
    let scene = new_scene("im-mixed")?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "both should be active with the toolbar",
    )?;

    scene.hide_toolbar();
    check(!scene.pan_enabled(), "pan followed the toolbar away but is still active")?;
    check(
        scene.wheel_zoom_enabled(),
        "a forced wheel zoom was switched off with the toolbar",
    )?;
    check(
        ctrl_wheel_zooms(&scene, "im-mixed")?,
        "wheel zoom stopped working after the toolbar was hidden",
    )?;
    check(
        !background_drag_pans("im-mixed")?,
        "the background still pans with no pan mode active",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Off` beats a shown toolbar, and the two gestures are independent of each other.
#[wasm_bindgen_test]
fn a_gesture_can_be_forced_off_even_while_the_toolbar_is_shown() -> Result<(), String> {
    let scene = new_scene("im-off")?;
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(!scene.pan_enabled(), "pan is active despite being forced off")?;
    check(scene.wheel_zoom_enabled(), "wheel zoom was affected by the pan mode")?;
    check(
        !background_drag_pans("im-off")?,
        "dragging the background panned despite pan being off",
    )?;
    check(ctrl_wheel_zooms(&scene, "im-off")?, "ctrl+wheel stopped zooming")?;
    check(
        query("#im-off > rect[aria-hidden=\"true\"]")?
            .and_then(|r| r.get_attribute("style"))
            .is_none(),
        "a surface with panning off still shows a grab cursor",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both `Off` with a toolbar shown leaves no surface at all, so nothing sits behind the content.
#[wasm_bindgen_test]
fn with_both_gestures_off_there_is_no_surface_even_with_a_toolbar() -> Result<(), String> {
    let scene = new_scene("im-both-off")?;
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(scene.has_toolbar(), "the toolbar is not shown")?;
    check(
        query("#im-both-off > rect[aria-hidden=\"true\"]")?.is_none(),
        "a surface exists with both gestures off",
    )?;
    check(!ctrl_wheel_zooms(&scene, "im-both-off")?, "wheel zoom worked with the mode off")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pan on, wheel zoom untouched: only the requested gesture is wired.
#[wasm_bindgen_test]
fn forcing_only_pan_on_leaves_wheel_zoom_off() -> Result<(), String> {
    let scene = new_scene("im-pan-only")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;

    check(background_drag_pans("im-pan-only")?, "dragging the background did not pan")?;
    check(
        !ctrl_wheel_zooms(&scene, "im-pan-only")?,
        "wheel zoom worked though it was never enabled",
    )?;
    check(scene.zoom_scale() == 1.0, "the scale changed")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Going back to `WithToolbar`, or to `Off`, tears the gesture down completely: no surface, no leftover wheel listener
/// on the content layer, so a wheel over a node is no longer cancelled.
#[wasm_bindgen_test]
fn switching_a_gesture_back_off_removes_its_surface_and_listener() -> Result<(), String> {
    let scene = new_scene("im-teardown")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("im-teardown", 0)?;

    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    check(
        wheel(&node, 110, 110, -100.0, true, false)?,
        "a wheel over a node was not cancelled while on",
    )?;

    scene.set_wheel_zoom_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    check(
        query("#im-teardown > rect[aria-hidden=\"true\"]")?.is_none(),
        "the surface remained after switching back",
    )?;
    let before = scene.zoom_scale();
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a leftover wheel listener still cancels",
    )?;
    check_close(scene.zoom_scale(), before)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Setting the same mode again changes nothing — in particular it does not rebuild the surface.
#[wasm_bindgen_test]
fn setting_the_same_mode_again_keeps_the_same_surface() -> Result<(), String> {
    let scene = new_scene("im-idempotent")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let first = required("#im-idempotent > rect[aria-hidden=\"true\"]")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let second = required("#im-idempotent > rect[aria-hidden=\"true\"]")?;
    check(first.is_same_node(Some(&second)), "an unchanged mode rebuilt the surface")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `refresh_layout` resizes the surface with no toolbar shown, and `refresh_toolbar_layout` still works as before.
#[wasm_bindgen_test]
fn refresh_layout_resizes_the_surface_with_no_toolbar() -> Result<(), String> {
    let scene = new_scene("im-refresh")?;
    let svg_element = required("#im-refresh")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let surface = required("#im-refresh > rect[aria-hidden=\"true\"]")?;
    check_close(attr_f64(&surface, "width")?, 400.0)?;

    // The scene cannot observe a `viewBox` change, which is why `refresh_layout` exists.
    svg_element
        .set_attribute("viewBox", "0 0 800 600")
        .map_err(|e| format!("{e:?}"))?;
    scene.refresh_layout().map_err(|e| e.to_string())?;
    check_close(attr_f64(&surface, "width")?, 800.0)?;
    check_close(attr_f64(&surface, "height")?, 600.0)?;

    svg_element
        .set_attribute("viewBox", "0 0 500 350")
        .map_err(|e| format!("{e:?}"))?;
    scene.refresh_toolbar_layout().map_err(|e| e.to_string())?;
    check_close(attr_f64(&surface, "width")?, 500.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a node that is currently selected drags only that node. Its selection — the text it exposes to assistive
/// technology, and every cell's colour — is untouched by the drag, and the scene does not pan.
#[wasm_bindgen_test]
fn dragging_a_selected_node_moves_it_keeps_its_selection_and_does_not_pan() -> Result<(), String> {
    let scene = new_scene("tb-selected-drag")?;
    let node = scene
        .add_data_node(
            Point::new(100.0, 100.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(0) })
        .map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let group = nth_group("tb-selected-drag", 0)?;
    let fills = |group: &web_sys::Element| -> Result<Vec<Option<String>>, String> {
        let rects = group.query_selector_all("rect").map_err(|e| format!("{e:?}"))?;
        Ok((0..rects.length())
            .filter_map(|i| rects.get(i))
            .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
            .map(|r| r.get_attribute("fill"))
            .collect())
    };
    let title = |group: &web_sys::Element| -> Result<Option<String>, String> {
        Ok(group
            .query_selector(":scope > title")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|t| t.text_content()))
    };
    let (label_before, fills_before) = (title(&group)?, fills(&group)?);
    check(
        fills_before.iter().any(|f| f.as_deref() == Some("#ff6b4a")),
        "test setup: no cell shows the focused colour",
    )?;

    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointermove", 140, 130, 1)?;
    dispatch_pointer_event(&group, "pointerup", 140, 130, 1)?;

    let (x, y) = group_translate(&group)?;
    check_close(x, 130.0)?;
    check_close(y, 120.0)?;
    check(
        title(&group)? == label_before,
        "the drag changed the node's accessible description",
    )?;
    check(fills(&group)? == fills_before, "the drag changed a cell's colour")?;
    check(
        content("tb-selected-drag")?.get_attribute("transform").is_none(),
        "dragging a selected node panned the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The surface's cursor tracks the pan: grabbing while the button is down, back to grab after a release or a cancel.
/// Real pointer capture is checked separately, against real input, in the CDP suite — a synthetic pointer id is not one
/// the browser will let a test capture.
#[wasm_bindgen_test]
fn the_pan_cursor_returns_to_idle_after_release_and_after_cancel() -> Result<(), String> {
    let scene = new_scene("tb-pan-cursor")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-cursor")?;
    let style = || attr(&surface, "style");

    check(
        style()?.contains("cursor: grab;"),
        "the idle surface does not show a grab cursor",
    )?;

    for end in ["pointerup", "pointercancel"] {
        dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
        check(
            style()?.contains("cursor: grabbing;"),
            "a pan in progress does not show a grabbing cursor",
        )?;
        dispatch_pointer_event(&surface, end, 120, 100, 1)?;
        check(
            style()?.contains("cursor: grab;"),
            &format!("the cursor did not return to grab after {end}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// However a pan ends — released, or cancelled — the next one starts cleanly and adds to the last, so no stale gesture
/// state is left behind.
#[wasm_bindgen_test]
fn a_pan_can_be_started_again_after_any_way_of_ending_one() -> Result<(), String> {
    let scene = new_scene("tb-pan-reuse")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-reuse")?;

    let mut expected = 0.0;
    for end in ["pointerup", "pointercancel", "pointerup"] {
        dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
        dispatch_pointer_event(&surface, "pointermove", 110, 100, 1)?;
        dispatch_pointer_event(&surface, end, 110, 100, 1)?;
        // `pointerup` flushes at once. A cancel does too, since it also ends the gesture.
        expected += 10.0;
        let (tx, _, _) = parse_view(&attr(&content("tb-pan-reuse")?, "transform")?)?;
        check_close(tx, expected)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's own buttons sit above the pan surface, so pressing one never starts a pan — but does still click.
#[wasm_bindgen_test]
fn pressing_a_toolbar_button_does_not_start_a_pan() -> Result<(), String> {
    let scene = new_scene("tb-button-pan")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("tb-button-pan", 0)?;

    dispatch_pointer_event(&plus, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&plus, "pointermove", 160, 100, 1)?;
    dispatch_pointer_event(&plus, "pointerup", 160, 100, 1)?;
    check(
        content("tb-button-pan")?.get_attribute("transform").is_none(),
        "pressing a toolbar button panned the scene",
    )?;

    click(&plus)?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing and hiding the toolbar repeatedly must never leave a handler behind. Every listener lives on the surface,
/// which is removed with the gestures, except the wheel listener on the content layer.
///
/// A leaked wheel listener cannot zoom, since it only holds a weak reference to a surface that no longer exists. What it
/// can still do is cancel the event, so the test that catches a leak is the last one: with everything torn down, a
/// modified wheel over a node must no longer be cancelled by anything.
#[wasm_bindgen_test]
fn showing_and_hiding_the_toolbar_repeatedly_installs_no_duplicate_handlers() -> Result<(), String> {
    let scene = new_scene("tb-cycles")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("tb-cycles", 0)?;
    let count = |selector: &str| -> Result<u32, String> {
        Ok(web_sys::window()
            .and_then(|w| w.document())
            .ok_or("no document")?
            .query_selector_all(selector)
            .map_err(|e| format!("{e:?}"))?
            .length())
    };

    for _ in 0..5 {
        scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
        scene.hide_toolbar();
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    // Showing again with a toolbar already up must replace it, not add a second.
    scene
        .show_toolbar(ToolbarOptions::new(Side::South))
        .map_err(|e| e.to_string())?;

    check(
        count("#tb-cycles > rect[aria-hidden=\"true\"]")? == 1,
        "there is not exactly one pan surface",
    )?;
    check(
        count("#tb-cycles > [role=\"toolbar\"]")? == 1,
        "there is not exactly one toolbar",
    )?;

    // One notch over a node reaches the content layer's own listener; over the background, the surface's.
    wheel(&node, 110, 110, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&pan_surface("tb-cycles")?, 100, 100, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // Tear it all down: nothing left from any earlier cycle may still react to the wheel.
    scene.hide_toolbar();
    check(
        count("#tb-cycles > rect[aria-hidden=\"true\"]")? == 0,
        "a surface survived hiding the toolbar",
    )?;
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a listener left by an earlier cycle still cancels the wheel",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, for a mode toggled back and forth: no accumulated surfaces or listeners.
#[wasm_bindgen_test]
fn toggling_input_modes_repeatedly_installs_no_duplicate_handlers() -> Result<(), String> {
    let scene = new_scene("im-cycles")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("im-cycles", 0)?;

    for _ in 0..8 {
        for mode in [InputMode::On, InputMode::Off, InputMode::WithToolbar] {
            scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
            scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
        }
    }
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;

    let surfaces = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all("#im-cycles > rect[aria-hidden=\"true\"]")
        .map_err(|e| format!("{e:?}"))?
        .length();
    check(surfaces == 1, &format!("expected one surface, found {surfaces}"))?;
    wheel(&node, 110, 110, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // Tear it all down: nothing left from any earlier toggle may still react to the wheel.
    scene.set_wheel_zoom_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    scene.set_pan_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a listener left by an earlier toggle still cancels the wheel",
    )
}
