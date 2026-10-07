//! The per-frame path. A `MutationObserver` shows what a pan or zoom frame *writes*, but not what it *reads*. Reading
//! an attribute from the DOM crosses the WASM and JavaScript boundary and allocates a `String` for the answer, so a
//! frame that reads to decide there is nothing to write is still paying for it — on every animation frame of a pan.

use super::support::*;
use crate::common::{check, dispatch_pointer_event};
use svg_dom_graph::scene::ToolbarOptions;
use wasm_bindgen_test::*;

/// Counts every `Element.getAttribute` call on the page while it is alive, and puts the original back when dropped.
struct AttributeReads;

impl AttributeReads {
    fn start() -> Result<Self, String> {
        js_sys::Function::new_no_args(
            "const proto = Element.prototype;
             if (!proto.__originalGetAttribute) { proto.__originalGetAttribute = proto.getAttribute; }
             window.__attributeReads = 0;
             proto.getAttribute = function (...args) {
                 window.__attributeReads += 1;
                 return proto.__originalGetAttribute.apply(this, args);
             };",
        )
        .call0(&wasm_bindgen::JsValue::NULL)
        .map_err(|e| format!("{e:?}"))?;
        Ok(Self)
    }

    fn count(&self) -> u32 {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &"__attributeReads".into()).ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::NAN) as u32
    }
}

impl Drop for AttributeReads {
    fn drop(&mut self) {
        let _ = js_sys::Function::new_no_args(
            "const proto = Element.prototype;
             if (proto.__originalGetAttribute) { proto.getAttribute = proto.__originalGetAttribute; }",
        )
        .call0(&wasm_bindgen::JsValue::NULL);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counter itself is trustworthy: an attribute read made on purpose is seen.
#[wasm_bindgen_test]
fn the_attribute_read_counter_sees_a_read() -> Result<(), String> {
    let element = required("body")?;
    let reads = AttributeReads::start()?;
    let _ = element.get_attribute("class");
    let _ = element.get_attribute("id");
    check(reads.count() == 2, &format!("expected 2 reads, counted {}", reads.count()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan frame reads nothing from the DOM. Pointer moves, keyboard arrows, a burst of wheel events, and the frame that
/// writes the result are all measured. The view changes, so the frame writes; nothing has to be read to know that.
#[wasm_bindgen_test]
async fn a_pan_or_wheel_frame_reads_no_attributes_from_the_dom() -> Result<(), String> {
    let scene = new_scene("frame-reads")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("frame-reads")?;
    let target = focus_target("frame-reads")?;

    // Settle first, so any one-off work from setting the scene up is not counted.
    scene.zoom_in().map_err(|e| e.to_string())?;
    next_frame().await?;

    let reads = AttributeReads::start()?;

    // A pan: many moves, one frame.
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    for step in 1..=20 {
        dispatch_pointer_event(&surface, "pointermove", 100 + 3 * step, 100 + step, 1)?;
    }
    next_frame().await?;
    dispatch_pointer_event(&surface, "pointerup", 160, 120, 1)?;
    check(
        reads.count() == 0,
        &format!("a pan frame read {} attribute(s) from the DOM", reads.count()),
    )?;

    // The keyboard.
    for _ in 0..5 {
        key(&target, "ArrowRight", false, false, false)?;
    }
    next_frame().await?;
    check(
        reads.count() == 0,
        &format!("keyboard panning read {} attribute(s) from the DOM", reads.count()),
    )?;

    // A burst of wheel events: the zoom changes on every one, and so does the label.
    for _ in 0..20 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    next_frame().await?;
    check(
        reads.count() == 0,
        &format!("a wheel frame read {} attribute(s) from the DOM", reads.count()),
    )?;

    // Zoom steps from the buttons, the keyboard, and the API. Each works out the centre of the visible area, which must
    // not mean reading the `viewBox` back from the DOM.
    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_out().map_err(|e| e.to_string())?;
    key(&target, "+", false, false, false)?;
    click(&button("frame-reads", 1)?)?;
    click(&button("frame-reads", 2)?)?;
    next_frame().await?;
    check(
        reads.count() == 0,
        &format!("a zoom step read {} attribute(s) from the DOM", reads.count()),
    )?;

    // The zoom steps are one more way in, so leave the view zoomed for the check below.
    scene.zoom_in().map_err(|e| e.to_string())?;
    for _ in 0..20 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    next_frame().await?;

    // And the frames did their job, so this is not a test of nothing.
    check(scene.zoom_scale() > 1.25, "the wheel burst did not zoom")?;
    check(
        content("frame-reads")?.get_attribute("transform").is_some(),
        "no frame wrote the view",
    )
}
