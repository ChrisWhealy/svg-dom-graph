//! Lifecycle. Showing the toolbar builds the buttons, the pan surface, and the pointer, wheel, and keyboard
//! handling. Hiding it takes them away. Do that repeatedly, and any handler that was not taken away is now stacked
//! on the new one: one input would then do its job twice.

use super::support::*;
use crate::common::{check, check_close, dispatch_pointer_event};
use svg_dom_graph::scene::{InputMode, Scene, Side, ToolbarOptions};
use wasm_bindgen_test::*;

/// Sends exactly one of each kind of input to a scene, and checks that each did its job exactly once. Every input is
/// measured from a reset view, so a doubled handler shows up as double the distance or double the zoom.
async fn every_input_does_exactly_one_thing(scene: &Scene, id: &str) -> Result<(), String> {
    let surface = pan_surface(id)?;
    let root = focus_target(id)?;

    // A toolbar button: one click is one step, not two.
    scene.reset_view().map_err(|e| e.to_string())?;
    click(&button(id, 0)?)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // The keyboard: one press is one 40-unit step or one zoom step.
    scene.reset_view().map_err(|e| e.to_string())?;
    key(&root, "ArrowRight", false, false, false)?;
    check(
        content_translate(id)? == (-40.0, 0.0),
        "one arrow key press did not move the view exactly 40 units",
    )?;
    scene.reset_view().map_err(|e| e.to_string())?;
    key(&root, "+", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // The wheel: one notch is one step.
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&surface, 100, 100, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // A pan: 40 pixels of drag is 40 units of movement.
    scene.reset_view().map_err(|e| e.to_string())?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    check(
        content_translate(id)? == (40.0, 0.0),
        "a 40 pixel drag did not move the view exactly 40 units",
    )?;

    next_frame().await?;
    scene.reset_view().map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sequence itself: show, hide, show, hide, show. After it, the toolbar and its gestures behave exactly as they would
/// have after a single show.
#[wasm_bindgen_test]
async fn show_hide_show_hide_show_then_every_input_does_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-sequence")?;

    // A fresh scene is the baseline.
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    every_input_does_exactly_one_thing(&scene, "lifecycle-sequence").await?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-sequence").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Many more cycles, mixing in a toolbar moved to every edge, replaced while shown, and re-shown with new options. One
/// stale listener from any of them would double an input.
#[wasm_bindgen_test]
async fn many_lifecycle_changes_still_leave_every_input_doing_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-many")?;

    for edge in [Side::North, Side::East, Side::South, Side::West, Side::North] {
        scene.show_toolbar(ToolbarOptions::new(edge)).map_err(|e| e.to_string())?;
        // Replace it while it is shown, which tears the old one down and builds another.
        scene.show_toolbar(ToolbarOptions::new(edge)).map_err(|e| e.to_string())?;
        scene.set_toolbar_edge(Side::East).map_err(|e| e.to_string())?;
        scene.hide_toolbar();
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-many").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for the input modes: one gesture switched on, off, and on again, over and over, with the toolbar showing
/// throughout.
#[wasm_bindgen_test]
async fn many_mode_changes_still_leave_every_input_doing_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-modes")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    for _ in 0..6 {
        for mode in [InputMode::Off, InputMode::On, InputMode::WithToolbar] {
            scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
            scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
        }
    }
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "test setup: the gestures are not active",
    )?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-modes").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Once every `Scene` handle is dropped, no input route does anything at all: not a button, not the wheel, not a key, not a
/// drag. If a listener were kept alive by a reference cycle, the scene it points at would still be alive and would still
/// answer, moving the view. Nothing here has a handle left to read the zoom from, so the rendered DOM is what is checked.
#[wasm_bindgen_test]
fn a_dropped_scene_answers_no_input_of_any_kind() -> Result<(), String> {
    let scene = new_scene("lifecycle-dropped")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    // Show and hide first, so the listeners left behind by earlier cycles, if any, are in play too.
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let plus = button("lifecycle-dropped", 0)?;
    let surface = pan_surface("lifecycle-dropped")?;
    let root = focus_target("lifecycle-dropped")?;
    drop(scene);

    click(&plus)?;
    keydown(&plus, "Enter")?;
    wheel(&surface, 100, 100, -100.0, true, false)?;
    key(&root, "ArrowRight", false, false, false)?;
    key(&root, "+", false, false, false)?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 160, 100, 1)?;

    check(
        content("lifecycle-dropped")?.get_attribute("transform").is_none(),
        "a dropped scene still answered input: a listener is holding it alive",
    )
}
