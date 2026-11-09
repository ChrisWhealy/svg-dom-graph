//! The size of this module's linear memory, as a capacity.
//!
//! It is read in bytes from the memory's buffer. It grows in 64 KiB pages and never shrinks, so it says how much the
//! module has ever needed and nothing about what is live. It is reported as a capacity, apart from the allocator's live
//! figures.

/// The byte length of the linear memory, or `None` when there is no wasm memory to ask, as on a native build.
#[cfg(target_arch = "wasm32")]
pub fn capacity_bytes() -> Option<f64> {
    use wasm_bindgen::JsCast;
    let memory = wasm_bindgen::memory().dyn_into::<js_sys::WebAssembly::Memory>().ok()?;
    let buffer = memory.buffer().dyn_into::<js_sys::ArrayBuffer>().ok()?;
    Some(f64::from(buffer.byte_length()))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn capacity_bytes() -> Option<f64> {
    None
}
