//! The listeners that the page has put on the document, which can all be taken off again.
//!
//! The library's own helpers for listeners keep their closures for the life of the page and give nothing back to remove
//! them with. These keep each closure and the function that was registered, so that [`Listeners::detach_all`] removes
//! every one from its target and frees what it captured.

use std::any::Any;
use wasm_bindgen::{JsCast, closure::Closure, convert::FromWasmAbi};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One listener that is on a target: where, for which event, the function that the browser holds, and the closure that
/// the function calls, which is kept so that it is freed when the listener is taken off.
struct Entry {
    target: web_sys::EventTarget,
    event: &'static str,
    function: js_sys::Function,
    _closure: Box<dyn Any>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every listener that a page has added, in the order that it added them.
#[derive(Default)]
pub(super) struct Listeners {
    entries: Vec<Entry>,
}

impl Listeners {
    /// Adds a listener for `event` on `target`, and keeps it so that it can be taken off.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the browser refused the listener, in which case nothing was added.
    pub(super) fn add<E: FromWasmAbi + 'static>(
        &mut self,
        target: &web_sys::EventTarget,
        event: &'static str,
        handler: impl FnMut(E) + 'static,
    ) -> Result<(), String> {
        let closure = Closure::<dyn FnMut(E)>::new(handler);
        let function: js_sys::Function = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
        target
            .add_event_listener_with_callback(event, &function)
            .map_err(|e| format!("could not attach the {event} listener: {e:?}"))?;
        self.entries.push(Entry {
            target: target.clone(),
            event,
            function,
            _closure: Box::new(closure),
        });
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Takes every listener off its target, last first, and frees the closures. A listener that the browser will not
    /// remove is still freed, since a freed closure that is called does nothing but report an error.
    pub(super) fn detach_all(&mut self) {
        while let Some(entry) = self.entries.pop() {
            let _ = entry.target.remove_event_listener_with_callback(entry.event, &entry.function);
        }
    }
}

impl Drop for Listeners {
    fn drop(&mut self) {
        self.detach_all();
    }
}
