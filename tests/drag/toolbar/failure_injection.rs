//! Failure. A view change is a transaction: it is applied, or it is not, and `zoom_scale()` never disagrees with
//! what is drawn. The DOM writes involved almost never fail for real, so these tests make them fail on purpose.

use super::support::*;
use crate::common::{check, check_close};
use svg_dom_graph::scene::ToolbarOptions;
use wasm_bindgen_test::*;

/// Makes `Element.setAttribute` throw for the named attributes while it is alive, and puts the original back when dropped.
struct FailingWrites;

impl FailingWrites {
    fn start(names: &[&str]) -> Result<Self, String> {
        let list = names.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>().join(", ");
        js_sys::Function::new_no_args(&format!(
            "const proto = Element.prototype;
             if (!proto.__originalSetAttribute) {{ proto.__originalSetAttribute = proto.setAttribute; }}
             const failing = [{list}];
             proto.setAttribute = function (name, value) {{
                 if (failing.includes(name)) {{ throw new Error('injected failure writing ' + name); }}
                 return proto.__originalSetAttribute.apply(this, arguments);
             }};"
        ))
        .call0(&wasm_bindgen::JsValue::NULL)
        .map_err(|e| format!("{e:?}"))?;
        Ok(Self)
    }
}

impl Drop for FailingWrites {
    fn drop(&mut self) {
        let _ = js_sys::Function::new_no_args(
            "const proto = Element.prototype;
             if (proto.__originalSetAttribute) { proto.setAttribute = proto.__originalSetAttribute; }",
        )
        .call0(&wasm_bindgen::JsValue::NULL);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The injector works: with it on, a write to the named attribute fails and one to any other does not.
#[wasm_bindgen_test]
fn the_write_failure_injector_fails_only_the_named_attribute() -> Result<(), String> {
    let element = required("body")?;
    {
        let _failing = FailingWrites::start(&["data-injected"])?;
        check(
            element.set_attribute("data-injected", "1").is_err(),
            "the named write did not fail",
        )?;
        check(element.set_attribute("data-other", "1").is_ok(), "an unnamed write failed")?;
    }
    check(
        element.set_attribute("data-injected", "1").is_ok(),
        "the injector was not removed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A zoom whose `transform` cannot be written is not applied at all: it reports the error, `zoom_scale()` is what it was,
/// and nothing is drawn. Once writes work again, the next zoom is one step from where the view really is — not two.
#[wasm_bindgen_test]
fn a_zoom_whose_write_fails_is_not_applied() -> Result<(), String> {
    let scene = new_scene("tx-zoom-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    {
        let _failing = FailingWrites::start(&["transform"])?;
        check(scene.zoom_in().is_err(), "a zoom whose write failed reported success")?;
        check_close(scene.zoom_scale(), 1.0)?;
        check(
            content("tx-zoom-fails")?.get_attribute("transform").is_none(),
            "a failed zoom drew something",
        )?;
    }

    scene.zoom_in().map_err(|e| e.to_string())?;
    check_close(scene.zoom_scale(), 1.25)?;
    check_close(drawn_scale("tx-zoom-fails")?, 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same from a view that is already zoomed: a failed reset keeps the zoom the graph is drawn at.
#[wasm_bindgen_test]
fn a_reset_whose_write_fails_keeps_the_zoom_that_is_drawn() -> Result<(), String> {
    let scene = new_scene("tx-reset-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;

    {
        let _failing = FailingWrites::start(&["transform"])?;
        check(scene.reset_view().is_err(), "a reset whose write failed reported success")?;
        check_close(scene.zoom_scale(), 1.25)?;
        check_close(drawn_scale("tx-reset-fails")?, 1.25)?;
    }

    scene.reset_view().map_err(|e| e.to_string())?;
    check_close(scene.zoom_scale(), 1.0)?;
    check_close(drawn_scale("tx-reset-fails")?, 1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A view already waiting for its frame is the view a failed change falls back to, still waiting. It is not lost, and not
/// applied twice: once writes work again, the next zoom starts from it.
#[wasm_bindgen_test]
async fn a_failed_zoom_falls_back_to_a_view_still_waiting_for_its_frame() -> Result<(), String> {
    let scene = new_scene("tx-pending-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // One wheel notch: the view is 1.25 now, and waiting to be drawn.
    wheel(&pan_surface("tx-pending-fails")?, 100, 100, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    {
        let _failing = FailingWrites::start(&["transform"])?;
        check(scene.zoom_in().is_err(), "a zoom whose write failed reported success")?;
        check_close(scene.zoom_scale(), 1.25)?;
    }

    // The frame that was waiting draws the view it was waiting for.
    next_frame().await?;
    check_close(drawn_scale("tx-pending-fails")?, 1.25)?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    check_close(scene.zoom_scale(), 1.5625)?;
    check_close(drawn_scale("tx-pending-fails")?, 1.5625)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// If the graph has zoomed, the operation has succeeded, however the accessible name fares. The name is bookkeeping: it
/// is put right by the next change, since a name that failed to be written is not treated as up to date.
#[wasm_bindgen_test]
fn a_failure_writing_only_the_accessible_name_does_not_undo_or_fail_the_zoom() -> Result<(), String> {
    let scene = new_scene("tx-label-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let target = focus_target("tx-label-fails")?;

    {
        let _failing = FailingWrites::start(&["aria-label"])?;
        check(
            scene.zoom_in().is_ok(),
            "the zoom was reported as failed although the graph zoomed",
        )?;
        check_close(scene.zoom_scale(), 1.25)?;
        check_close(drawn_scale("tx-label-fails")?, 1.25)?;
        check(
            attr(&target, "aria-label")? == "Graph view, zoom 100%",
            "test setup: the name changed despite the failure",
        )?;
    }

    scene.zoom_in().map_err(|e| e.to_string())?;
    check(
        attr(&target, "aria-label")? == "Graph view, zoom 156%",
        "the name was not put right by the next change",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for the toolbar buttons' enabled state.
#[wasm_bindgen_test]
fn a_failure_writing_only_a_buttons_state_does_not_undo_or_fail_the_zoom() -> Result<(), String> {
    let scene = new_scene("tx-button-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        attr(&button("tx-button-fails", 2)?, "aria-disabled")? == "true",
        "test setup: Reset is enabled before any zoom",
    )?;

    {
        let _failing = FailingWrites::start(&["aria-disabled"])?;
        check(
            scene.zoom_in().is_ok(),
            "the zoom was reported as failed although the graph zoomed",
        )?;
        check_close(scene.zoom_scale(), 1.25)?;
        check_close(drawn_scale("tx-button-fails")?, 1.25)?;
    }

    scene.zoom_in().map_err(|e| e.to_string())?;
    check(
        attr(&button("tx-button-fails", 2)?, "aria-disabled")? == "false",
        "Reset's state was not put right by the next change",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel or pan change is drawn by a frame, and a frame has nobody to report an error to. If its write fails, the view is
/// not lost: it stays waiting, so the next thing that draws the view draws it, and the scene and the drawing agree again.
#[wasm_bindgen_test]
async fn a_frame_whose_write_fails_leaves_the_view_waiting_and_the_next_write_catches_up() -> Result<(), String> {
    let scene = new_scene("tx-frame-fails")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tx-frame-fails")?;

    {
        let _failing = FailingWrites::start(&["transform"])?;
        wheel(&surface, 100, 100, -100.0, true, false)?;
        next_frame().await?;
        // The wheel zoomed the scene. Its frame could not draw it.
        check_close(scene.zoom_scale(), 1.25)?;
        check(
            content("tx-frame-fails")?.get_attribute("transform").is_none(),
            "a frame whose write failed drew something",
        )?;
    }

    // Writes work again. The next change draws the view it leaves, which includes the one that was waiting.
    wheel(&surface, 100, 100, -100.0, true, false)?;
    next_frame().await?;
    check_close(scene.zoom_scale(), 1.5625)?;
    check_close(drawn_scale("tx-frame-fails")?, 1.5625)
}
