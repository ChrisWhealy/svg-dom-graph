//! Teardown between an input and its animation frame. Wheel zoom and panning update the view at once and write the DOM
//! at the next animation frame. If the input handling is taken away in between — the toolbar hidden, a mode changed,
//! the scene dropped — that frame is still pending in the browser. It must neither call into something that has been
//! freed, nor be lost so that the rendered graph and `zoom_scale()` disagree.

use super::support::*;
use crate::common::{check, check_close, dispatch_pointer_event, nth_group};
use svg_dom_graph::scene::{InputMode, ToolbarOptions};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

/// Collects every uncaught JavaScript error on the page. A browser calling a `wasm-bindgen` closure that has been
/// dropped throws one, so this is how a test sees it.
struct ErrorWatch {
    errors: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    callback: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

impl ErrorWatch {
    fn start() -> Result<Self, String> {
        let errors: std::rc::Rc<std::cell::RefCell<Vec<String>>> = Default::default();
        let sink = errors.clone();
        let callback =
            wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |event: web_sys::ErrorEvent| {
                sink.borrow_mut().push(event.message());
            });
        web_sys::window()
            .ok_or("no window")?
            .add_event_listener_with_callback("error", callback.as_ref().unchecked_ref())
            .map_err(|e| format!("{e:?}"))?;
        Ok(Self { errors, callback })
    }

    fn errors(&self) -> Vec<String> {
        self.errors.borrow().clone()
    }
}

impl Drop for ErrorWatch {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback("error", self.callback.as_ref().unchecked_ref());
        }
    }
}

/// Lets the browser run any pending frame, and then one more, so a frame scheduled by a frame has run too.
async fn settle() -> Result<(), String> {
    next_frame().await?;
    next_frame().await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel zoom, then the toolbar hidden before its frame. Nothing may throw, and the view the wheel made must reach
/// the DOM: the scene and what is drawn agree.
#[wasm_bindgen_test]
async fn hiding_the_toolbar_between_a_wheel_and_its_frame_throws_nothing_and_keeps_the_zoom() -> Result<(), String> {
    let scene = new_scene("td-wheel-hide")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let watch = ErrorWatch::start()?;

    wheel(&pan_surface("td-wheel-hide")?, 100, 100, -100.0, true, false)?;
    scene.hide_toolbar();
    settle().await?;

    check(
        watch.errors().is_empty(),
        &format!("a pending frame threw: {:?}", watch.errors()),
    )?;
    check_close(scene.zoom_scale(), 1.25)?;
    check_close(drawn_scale("td-wheel-hide")?, 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan move, then panning switched off before its frame. That rebuilds the gestures, so the old ones are torn down
/// while a frame is pending. The pan so far must reach the DOM.
#[wasm_bindgen_test]
async fn switching_panning_off_between_a_pan_move_and_its_frame_throws_nothing_and_keeps_the_pan() -> Result<(), String>
{
    let scene = new_scene("td-pan-off")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("td-pan-off")?;
    let watch = ErrorWatch::start()?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    settle().await?;

    check(
        watch.errors().is_empty(),
        &format!("a pending frame threw: {:?}", watch.errors()),
    )?;
    check(
        content_translate("td-pan-off")? == (40.0, 0.0),
        "the pan made so far did not reach the DOM",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel zoom, then the wheel mode switched off before its frame. Tears the wheel handling down and builds the rest
/// again.
#[wasm_bindgen_test]
async fn switching_wheel_zoom_off_between_a_wheel_and_its_frame_throws_nothing_and_keeps_the_zoom() -> Result<(), String>
{
    let scene = new_scene("td-wheel-off")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let watch = ErrorWatch::start()?;

    wheel(&pan_surface("td-wheel-off")?, 100, 100, -100.0, true, false)?;
    scene.set_wheel_zoom_mode(InputMode::Off).map_err(|e| e.to_string())?;
    settle().await?;

    check(
        watch.errors().is_empty(),
        &format!("a pending frame threw: {:?}", watch.errors()),
    )?;
    check_close(scene.zoom_scale(), 1.25)?;
    check_close(drawn_scale("td-wheel-off")?, 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel zoom, then every `Scene` handle dropped before its frame. The scene's own state goes, and with it the
/// handling that scheduled the frame. The last view must still be drawn.
#[wasm_bindgen_test]
async fn dropping_the_scene_between_a_wheel_and_its_frame_throws_nothing_and_draws_the_last_view() -> Result<(), String>
{
    let scene = new_scene("td-wheel-drop")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("td-wheel-drop")?;
    let watch = ErrorWatch::start()?;

    wheel(&surface, 100, 100, -100.0, true, false)?;
    drop(scene);
    settle().await?;

    check(
        watch.errors().is_empty(),
        &format!("a pending frame threw: {:?}", watch.errors()),
    )?;
    check_close(drawn_scale("td-wheel-drop")?, 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The older node-drag coalescer has the same shape: a drag move, then the scene dropped before the frame that would
/// apply it.
#[wasm_bindgen_test]
async fn dropping_the_scene_between_a_drag_move_and_its_frame_throws_nothing() -> Result<(), String> {
    let scene = draggable_node_scene("td-drag-drop")?;
    let group = nth_group("td-drag-drop", 0)?;
    let watch = ErrorWatch::start()?;

    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointermove", 150, 130, 1)?;
    drop(scene);
    settle().await?;

    check(
        watch.errors().is_empty(),
        &format!("a pending frame threw: {:?}", watch.errors()),
    )
}
