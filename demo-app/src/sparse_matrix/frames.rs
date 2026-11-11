//! The run: asking for each frame, taking a step in it, and stopping.

use super::{ERROR, PANEL, controls::selected, frame_callback::FrameCallback, interaction::step};
use demo_common::util::{document, report_error};
use spmv_scene::scene::now;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Stops a run in progress, cancelling the frame that is waiting, and makes any frame that already started do nothing.
pub(super) fn stop_run() {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.running = false;
        p.generation += 1;
        if let (Some(id), Some(window)) = (p.frame.take(), web_sys::window()) {
            window.cancel_animation_frame(id).ok();
        }
        // A frame that was cancelled is never called, so its callback is dropped here and not by the browser. One that
        // is running cannot be dropped from inside itself, and goes when the next starts.
        p.callback = None;
        if !p.in_frame {
            p.finished = None;
        }
    });
    if let Some(e) = document().ok().and_then(|d| d.get_element_by_id("spmv-run")) {
        e.set_text_content(Some("Run"));
        e.set_attribute("aria-pressed", "false").ok();
    }
}

pub(super) fn frame(generation: u32) {
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.in_frame = true;
        p.frame = None;
        // This is the callback that is being called. The one before it has finished, and goes.
        if p.callback.as_ref().is_some_and(|c| c.generation == generation) {
            p.finished = p.callback.take();
        }
    });
    frame_body(generation);
    let stopped = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.in_frame = false;
        !p.running
    });
    if stopped {
        release_when_unwound();
    }
}

/// Lets go of the callback that has just run, after the browser's call of it has returned. A run that stops inside its
/// own frame, for a failed step or a failed request for the next frame or a hidden page, has no later frame to release
/// it, and it cannot be dropped while it is running. A microtask runs when the call has returned, and before anything
/// else does.
pub(super) fn release_when_unwound() {
    let release = Closure::once_into_js(|| {
        PANEL.with(|p| {
            let mut p = p.borrow_mut();
            if !p.in_frame && !p.running {
                p.finished = None;
            }
        });
    });
    if let Some(window) = web_sys::window() {
        window.queue_microtask(release.unchecked_ref());
    }
}

/// Whether a test has asked for a failure at `name`, which it does by setting that property of the window. It is read
/// once and cleared, so that a failure is injected once. Nothing sets one but a test.
pub(super) fn injected(name: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let asked = js_sys::Reflect::get(&window, &name.into()).is_ok_and(|v| v.is_truthy());
    if asked {
        js_sys::Reflect::set(&window, &name.into(), &wasm_bindgen::JsValue::FALSE).ok();
    }
    asked
}

pub(super) fn frame_body(generation: u32) {
    let current = PANEL.with(|p| {
        let p = p.borrow();
        p.running && p.generation == generation
    });
    if !current {
        return;
    }
    // A window that is hidden has no use for a step, and a run stops itself.
    if document().map_or(true, |d| d.hidden()) {
        stop_run();
        return;
    }
    let paced = PANEL.with(|p| {
        let p = p.borrow();
        now() < p.next_step_at
    });
    if paced {
        // Not yet time for the next step, so that a person can follow the last one.
        if let Err(message) = schedule(generation) {
            stop_run();
            report_error(ERROR, &message);
        }
        return;
    }
    let pace: f64 = selected("spmv-speed").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    PANEL.with(|p| p.borrow_mut().next_step_at = now() + pace);
    if let Err(message) = step() {
        stop_run();
        report_error(ERROR, &message);
        return;
    }
    if let Err(message) = schedule(generation) {
        stop_run();
        report_error(ERROR, &message);
    }
}

pub(super) fn schedule(generation: u32) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    if injected("__spmvFailNextSchedule") {
        return Err("A failure was injected into asking for the next frame.".to_owned());
    }
    let callback = FrameCallback::new(generation);
    let id = window
        .request_animation_frame(callback.closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not ask for a frame: {e:?}"))?;
    PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.frame = Some(id);
        p.callback = Some(callback);
    });
    Ok(())
}

pub(super) fn toggle_run() -> Result<(), String> {
    let running = PANEL.with(|p| p.borrow().running);
    if running {
        stop_run();
        return Ok(());
    }
    if PANEL.with(|p| p.borrow().live.is_none()) {
        return Err("Nothing is built yet.".to_owned());
    }
    let generation = PANEL.with(|p| {
        let mut p = p.borrow_mut();
        p.generation += 1;
        p.running = true;
        p.next_step_at = 0.0;
        p.generation
    });
    if let Some(e) = document()?.get_element_by_id("spmv-run") {
        e.set_text_content(Some("Stop"));
        e.set_attribute("aria-pressed", "true").ok();
    }
    schedule(generation)
}
