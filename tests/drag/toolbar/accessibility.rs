//! Accessibility. A keyboard user can zoom from the toolbar, so they must also be able to move the view afterwards, and
//! see where focus is.

use super::support::*;
use crate::common::{attr_f64, check, check_close, dispatch_pointer_event};
use svg_dom_graph::scene::{InputMode, ToolbarOptions};
use wasm_bindgen_test::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every button exposes the button role, a name that does not depend on its visible text, and a place in the Tab order.
#[wasm_bindgen_test]
fn every_toolbar_button_has_the_button_role_an_explicit_name_and_a_tab_stop() -> Result<(), String> {
    let scene = new_scene("a11y-buttons")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // "Reset" is drawn as "100%", so its name must not be read from the visible text.
    for (n, name) in ["Zoom in", "Zoom out", "Reset zoom"].iter().enumerate() {
        let b = button("a11y-buttons", n)?;
        check(
            attr(&b, "role")? == "button",
            &format!("button {n} does not have the button role"),
        )?;
        check(attr(&b, "aria-label")? == *name, &format!("button {n} is not named {name:?}"))?;
        check(attr(&b, "tabindex")? == "0", &format!("button {n} is not in the Tab order"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keyboard focus is drawn explicitly, as a thicker, differently coloured border. It is not left to whatever outline a
/// browser draws for a focused SVG element. It goes away again on blur.
#[wasm_bindgen_test]
fn a_focused_toolbar_button_shows_an_obvious_focus_ring_that_blur_removes() -> Result<(), String> {
    let scene = new_scene("a11y-focus")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("a11y-focus", 0)?;
    let rect = plus.first_element_child().ok_or("a button has no rect")?;
    let resting = (attr(&rect, "stroke")?, attr_f64(&rect, "stroke-width")?);

    plus.dispatch_event(&web_sys::FocusEvent::new("focus").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&rect, "stroke")? != resting.0, "focus did not change the border colour")?;
    check(attr_f64(&rect, "stroke-width")? > resting.1, "focus did not thicken the border")?;

    plus.dispatch_event(&web_sys::FocusEvent::new("blur").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&rect, "stroke")? == resting.0, "blur did not restore the border colour")?;
    check_close(attr_f64(&rect, "stroke-width")?, resting.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Activating a control marked `aria-disabled` does nothing at all — by click or by keyboard. In particular it writes
/// nothing to the DOM, so the content layer still has no `transform`.
#[wasm_bindgen_test]
fn activating_an_aria_disabled_button_does_nothing() -> Result<(), String> {
    let scene = new_scene("a11y-disabled")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // Unzoomed, "Reset" has nothing to do.
    let reset = button("a11y-disabled", 2)?;
    check(attr(&reset, "aria-disabled")? == "true", "test setup: Reset is not disabled")?;
    click(&reset)?;
    keydown(&reset, "Enter")?;
    keydown(&reset, " ")?;
    check(
        content("a11y-disabled")?.get_attribute("transform").is_none(),
        "activating a disabled Reset changed the view",
    )?;

    // At maximum zoom, "zoom in" has nothing to do.
    for _ in 0..30 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let plus = button("a11y-disabled", 0)?;
    check(
        attr(&plus, "aria-disabled")? == "true",
        "test setup: zoom in is not disabled at the limit",
    )?;
    let before = attr(&content("a11y-disabled")?, "transform")?;
    click(&plus)?;
    keydown(&plus, "Enter")?;
    check(
        attr(&content("a11y-disabled")?, "transform")? == before,
        "activating a disabled zoom in changed the view",
    )?;
    check_close(scene.zoom_scale(), 4.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With panning on, the scene adds a keyboard focus target of its own: a role, a name that says how far it is zoomed,
/// and a description of its keys. All of that is on that element, and none of it on the application's `<svg>`.
#[wasm_bindgen_test]
async fn the_scene_adds_a_named_keyboard_target_that_reports_its_zoom() -> Result<(), String> {
    let scene = new_scene("a11y-root")?;
    let svg = required("#a11y-root")?;
    check(
        query("#a11y-root > rect[role=\"application\"]")?.is_none(),
        "a keyboard target exists before anything asks for it",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let target = focus_target("a11y-root")?;
    check(attr(&target, "tabindex")? == "0", "the target is not in the Tab order")?;
    check(
        attr(&target, "aria-label")? == "Graph view, zoom 100%",
        "the target's name does not report its zoom",
    )?;
    check(
        attr(&target, "aria-description")?.contains("Arrow keys pan"),
        "the target does not say how to pan",
    )?;
    for name in ["role", "tabindex", "aria-label", "aria-description"] {
        check(!svg.has_attribute(name), &format!("{name} was put on the application's <svg>"))?;
    }

    scene.zoom_in().map_err(|e| e.to_string())?;
    check(
        attr(&target, "aria-label")? == "Graph view, zoom 125%",
        "the name did not follow a button zoom",
    )?;

    // The name is updated by a zoom that is deferred to an animation frame, too.
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&pan_surface("a11y-root")?, 100, 100, -100.0, true, false)?;
    next_frame().await?;
    check(
        attr(&target, "aria-label")? == "Graph view, zoom 125%",
        "the name did not follow a wheel zoom",
    )
}
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Arrow keys move the view like scrolling: pressing right moves the view right, so the content moves left. One press
/// is 40 units, and Shift makes it five times further. This is the keyboard way back to content that zooming pushed out
/// of view.
#[wasm_bindgen_test]
fn arrow_keys_pan_the_view_and_shift_pans_further() -> Result<(), String> {
    let scene = new_scene("a11y-arrows")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = focus_target("a11y-arrows")?;

    check(
        key(&root, "ArrowRight", false, false, false)?,
        "a handled arrow key was not cancelled",
    )?;
    check(
        content_translate("a11y-arrows")? == (-40.0, 0.0),
        "ArrowRight did not move the content left by 40",
    )?;
    key(&root, "ArrowDown", false, false, false)?;
    check(
        content_translate("a11y-arrows")? == (-40.0, -40.0),
        "ArrowDown did not move the content up by 40",
    )?;
    key(&root, "ArrowLeft", false, false, false)?;
    key(&root, "ArrowUp", false, false, false)?;
    check(
        content_translate("a11y-arrows")? == (0.0, 0.0),
        "opposite arrows did not cancel out",
    )?;

    key(&root, "ArrowLeft", true, false, false)?;
    check(
        content_translate("a11y-arrows")? == (200.0, 0.0),
        "Shift did not make a press five times further",
    )?;
    // The 100% button is enabled by a pan, and undoes it.
    check(
        attr(&button("a11y-arrows", 2)?, "aria-disabled")? == "false",
        "a keyboard pan did not enable Reset",
    )?;
    click(&button("a11y-arrows", 2)?)?;
    check(
        content_translate("a11y-arrows")? == (0.0, 0.0),
        "Reset did not undo a keyboard pan",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pan step is in the `<svg>`'s own units, so it looks the same on screen however far the content is zoomed.
#[wasm_bindgen_test]
fn a_keyboard_pan_step_is_the_same_distance_at_any_zoom() -> Result<(), String> {
    let scene = new_scene("a11y-arrows-zoom")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    for _ in 0..4 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let (before, _) = content_translate("a11y-arrows-zoom")?;
    key(&focus_target("a11y-arrows-zoom")?, "ArrowRight", false, false, false)?;
    let (after, _) = content_translate("a11y-arrows-zoom")?;
    check_close(after - before, -40.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Plus, minus, and zero zoom from the keyboard, about the centre of the visible area.
#[wasm_bindgen_test]
fn plus_minus_and_zero_zoom_from_the_keyboard() -> Result<(), String> {
    let scene = new_scene("a11y-zoom-keys")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = focus_target("a11y-zoom-keys")?;

    check(key(&root, "+", false, false, false)?, "a handled zoom key was not cancelled")?;
    check_close(scene.zoom_scale(), 1.25)?;
    key(&root, "=", false, false, false)?; // the unshifted key that shares a cap with "+"
    check_close(scene.zoom_scale(), 1.5625)?;
    key(&root, "-", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)?;
    key(&root, "0", false, false, false)?;
    check_close(scene.zoom_scale(), 1.0)?;
    check(
        attr(&content("a11y-zoom-keys")?, "transform")? == "translate(0, 0) scale(1)",
        "zero did not restore the identity",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keys the scene does not use, and keys held with Ctrl, Cmd, or Alt, are neither acted on nor cancelled. So the
/// browser's own page zoom (Ctrl or Cmd with plus or minus) and other shortcuts keep working.
#[wasm_bindgen_test]
fn unrelated_keys_and_browser_shortcuts_are_left_alone() -> Result<(), String> {
    let scene = new_scene("a11y-shortcuts")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = focus_target("a11y-shortcuts")?;

    check(!key(&root, "a", false, false, false)?, "an unrelated key was cancelled")?;
    check(
        !key(&root, "Tab", false, false, false)?,
        "Tab was cancelled, which would trap keyboard focus",
    )?;
    check(!key(&root, "ArrowRight", false, true, false)?, "Ctrl+Arrow was cancelled")?;
    check(
        !key(&root, "+", false, true, false)?,
        "Ctrl+plus was cancelled, which would block browser page zoom",
    )?;
    check(
        !key(&root, "-", false, false, true)?,
        "Cmd+minus was cancelled, which would block browser page zoom",
    )?;
    check(
        scene.zoom_scale() == 1.0 && content_translate("a11y-shortcuts")? == (0.0, 0.0),
        "an ignored key moved the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A key pressed on a toolbar button bubbles up through the `<svg>`, but is the button's, not the scene's. An arrow key
/// on a focused button must not pan the view.
#[wasm_bindgen_test]
fn a_key_pressed_on_a_toolbar_button_does_not_pan_the_view() -> Result<(), String> {
    let scene = new_scene("a11y-bubble")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(
        !key(&button("a11y-bubble", 0)?, "ArrowRight", false, false, false)?,
        "a button's arrow key was cancelled",
    )?;
    check(
        content_translate("a11y-bubble")? == (0.0, 0.0),
        "an arrow key on a button panned the view",
    )?;
    check_close(scene.zoom_scale(), 1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each key group follows its own mode: the arrows are the keyboard side of panning, and plus, minus, and zero the
/// keyboard side of wheel zoom.
#[wasm_bindgen_test]
fn the_arrow_keys_follow_the_pan_mode_and_the_zoom_keys_follow_the_wheel_zoom_mode() -> Result<(), String> {
    let scene = new_scene("a11y-modes")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let root = focus_target("a11y-modes")?;

    // Panning on, wheel zoom off: arrows work, zoom keys do not.
    key(&root, "ArrowRight", false, false, false)?;
    check(content_translate("a11y-modes")? == (-40.0, 0.0), "the arrow keys did not pan")?;
    check(
        !key(&root, "+", false, false, false)?,
        "a zoom key was handled with wheel zoom off",
    )?;
    check_close(scene.zoom_scale(), 1.0)?;
    check(
        !attr(&root, "aria-description")?.contains("Plus and minus"),
        "the description mentions keys that are off",
    )?;

    // Wheel zoom on, panning off: the reverse.
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    // Changing a mode rebuilds the gestures, and with them the focus target, so the old element is gone.
    let root = focus_target("a11y-modes")?;
    let before = content_translate("a11y-modes")?;
    check(
        !key(&root, "ArrowRight", false, false, false)?,
        "an arrow key was handled with panning off",
    )?;
    check(
        content_translate("a11y-modes")? == before,
        "an arrow key panned with panning off",
    )?;
    key(&root, "+", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The keyboard target is the scene's own, so once no gesture needs it the whole element goes, along with its role,
/// name, description, tab stop, and key listener. Nothing of the application's `<svg>` was involved, so nothing on it
/// changes.
#[wasm_bindgen_test]
fn hiding_the_toolbar_removes_the_keyboard_target_and_leaves_the_svg_alone() -> Result<(), String> {
    let scene = new_scene("a11y-teardown")?;
    let svg = required("#a11y-teardown")?;
    let before = attributes_of(&svg);
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let target = focus_target("a11y-teardown")?;

    scene.hide_toolbar();
    check(
        query("#a11y-teardown > rect[role=\"application\"]")?.is_none(),
        "the keyboard target was left in the DOM after the toolbar was hidden",
    )?;
    check(
        attributes_of(&svg) == before,
        "hiding the toolbar changed the application's <svg>",
    )?;
    check(
        !key(&target, "ArrowRight", false, false, false)?,
        "a key listener was left behind on the removed target",
    )?;
    check(
        content_translate("a11y-teardown")? == (0.0, 0.0),
        "a leftover key listener panned the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A key sent to the application's `<svg>` itself is not the scene's. Only the focus target takes keys. So an
/// application that handles keys on its own `<svg>` is not competing with anything.
#[wasm_bindgen_test]
fn a_key_sent_to_the_svg_itself_does_nothing() -> Result<(), String> {
    let scene = new_scene("a11y-svg-key")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let svg = required("#a11y-svg-key")?;

    check(
        !key(&svg, "ArrowRight", false, false, false)?,
        "the scene cancelled a key sent to the <svg>",
    )?;
    key(&svg, "+", false, false, false)?;
    check(
        content_translate("a11y-svg-key")? == (0.0, 0.0) && scene.zoom_scale() == 1.0,
        "a key sent to the <svg> changed the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The target draws nothing until it has keyboard focus. Then it outlines the visible area, so it is obvious where
/// focus is, and blur takes the outline away again.
#[wasm_bindgen_test]
fn the_keyboard_target_outlines_the_scene_only_while_it_has_focus() -> Result<(), String> {
    let scene = new_scene("a11y-outline")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let target = focus_target("a11y-outline")?;

    check(attr(&target, "stroke")? == "none", "the target draws an outline when unfocused")?;
    check(attr(&target, "fill")? == "none", "the target fills the scene")?;

    target
        .dispatch_event(&web_sys::FocusEvent::new("focus").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&target, "stroke")? != "none", "focus did not outline the scene")?;
    check(attr_f64(&target, "stroke-width")? > 0.0, "the focus outline has no width")?;

    target
        .dispatch_event(&web_sys::FocusEvent::new("blur").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&target, "stroke")? == "none", "blur did not remove the outline")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The target lies above the pan surface but takes no pointer events. It covers the same area. So it neither blocks
/// panning nor is left behind when the layout is refreshed.
#[wasm_bindgen_test]
fn the_keyboard_target_never_gets_in_the_way_of_the_pan_surface() -> Result<(), String> {
    let scene = new_scene("a11y-passthrough")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let target = focus_target("a11y-passthrough")?;
    let surface = pan_surface("a11y-passthrough")?;

    check(attr(&target, "pointer-events")? == "none", "the target takes pointer events")?;
    // Beneath the content layer, and after the surface: surface, target, content.
    check(
        surface.next_element_sibling().is_some_and(|n| n.is_same_node(Some(&target))),
        "the target does not directly follow the surface",
    )?;
    check(
        target
            .next_element_sibling()
            .is_some_and(|n| n.is_same_node(Some(&content("a11y-passthrough").unwrap_or(target.clone())))),
        "the target does not lie directly beneath the content layer",
    )?;
    for name in ["x", "y", "width", "height"] {
        check_close(attr_f64(&target, name)?, attr_f64(&surface, name)?)?;
    }

    // And panning through the same area still works.
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    check(
        content_translate("a11y-passthrough")? == (40.0, 0.0),
        "the surface no longer pans",
    )?;
    check_close(scene.zoom_scale(), 1.0)
}
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom changes must not cause disruptive announcements. Nothing here is a live region. The state attributes are only
/// rewritten when they actually change. So a run of zoom steps that changes no button's state touches none of them.
#[wasm_bindgen_test]
fn zooming_adds_no_live_region_and_rewrites_no_unchanged_button_state() -> Result<(), String> {
    let scene = new_scene("a11y-quiet")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;

    let live = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all("#a11y-quiet [aria-live], #a11y-quiet [role=\"status\"], #a11y-quiet [role=\"alert\"]")
        .map_err(|e| format!("{e:?}"))?
        .length();
    check(live == 0, "a live region was added, which would announce every zoom step")?;

    // Watch every button's state attributes through more zoom steps that leave every button's state as it was.
    let observed = ["aria-disabled", "opacity"];
    let before: Vec<Vec<String>> = (0..3)
        .map(|n| {
            button("a11y-quiet", n).map(|b| observed.iter().map(|a| b.get_attribute(a).unwrap_or_default()).collect())
        })
        .collect::<Result<_, _>>()?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_out().map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let after: Vec<Vec<String>> = (0..3)
        .map(|n| {
            button("a11y-quiet", n).map(|b| observed.iter().map(|a| b.get_attribute(a).unwrap_or_default()).collect())
        })
        .collect::<Result<_, _>>()?;
    check(
        before == after,
        "a zoom step that changed no button's state changed its attributes",
    )
}
