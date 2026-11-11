//! A callback for an animation frame, and the generation that it was asked for under.

use super::{LIVE_CALLBACKS, ROOT, frames::frame};
use demo_common::util::document;
use wasm_bindgen::closure::Closure;

/// A callback for an animation frame, and the generation that it was asked for under. It counts itself while it lives.
pub(super) struct FrameCallback {
    pub(super) generation: u32,
    pub(super) closure: Closure<dyn FnMut()>,
}

impl FrameCallback {
    pub(super) fn new(generation: u32) -> Self {
        let closure = Closure::<dyn FnMut()>::new(move || frame(generation));
        Self::count(1);
        Self { generation, closure }
    }

    fn count(change: isize) {
        let live = LIVE_CALLBACKS.with(|c| {
            c.set(c.get().saturating_add_signed(change));
            c.get()
        });
        if let Some(host) = document().ok().and_then(|d| d.get_element_by_id(ROOT)) {
            host.set_attribute("data-live-frame-callbacks", &live.to_string()).ok();
        }
    }
}

impl Drop for FrameCallback {
    fn drop(&mut self) {
        Self::count(-1);
    }
}
