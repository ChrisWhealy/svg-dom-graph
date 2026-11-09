//! A wasm-bindgen import that returns an object and throws loses one externref slot for each throw.
//!
//! Each import below is declared `catch`, so that a JavaScript exception becomes an `Err`. The test calls each one many
//! times, with a failure, and asks wasm-bindgen how many externrefs it believes are live before and after. That is
//! `wasm_bindgen::externref_heap_live_count()`, the function that crate documents for finding this kind of leak.
//!
//! The result with wasm-bindgen 0.2.129 is that the count rises by exactly one for each failure of an import that
//! returns an object, and by nothing for one that returns nothing or a number, or for any import that does not fail.
//! The `Err` that comes back is dropped by the caller each time, so nothing in the caller holds the slot.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/helpers.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn returns_object(fail: bool) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    fn returns_nothing(fail: bool) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn returns_number(fail: bool) -> Result<f64, JsValue>;
}

/// Calls an import `times` times, failing each time or not, and says how many externref slots that left live.
pub fn slots_left_by(call: impl Fn(bool), times: u32, fail: bool) -> u32 {
    let before = wasm_bindgen::externref_heap_live_count();
    for _ in 0..times {
        call(fail);
    }
    wasm_bindgen::externref_heap_live_count() - before
}

pub fn object(fail: bool) {
    let _ = returns_object(fail);
}

pub fn nothing(fail: bool) {
    let _ = returns_nothing(fail);
}

pub fn number(fail: bool) {
    let _ = returns_number(fail);
}
