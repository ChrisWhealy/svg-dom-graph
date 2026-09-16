//! `demo_fn_source`'s textual extraction out of each demo module's own `SOURCE` is a separate mechanism from
//! `DEMO_PANELS` itself. Compilation only proves each name in `demo_gallery!` resolves to a real function, not
//! that `demo_fn_source` can find its source text there.
//!
//! A function that compiles fine but breaks the extraction (renamed without updating `demo_gallery!`, or
//! reformatted in a way that moves its closing brace out of column 0) would still pass `cargo build`. It would
//! only surface as `append_demo_source` returning `Err` — a startup failure a user would only notice by looking at
//! the browser console. This test is what actually catches that class of drift, on every `cargo test` run.

use super::DEMO_PANELS;
use crate::source_frame::demo_fn_source;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_registered_demo_has_extractable_source() -> Result<(), String> {
    for panel in DEMO_PANELS {
        if demo_fn_source(panel.source, panel.fn_name).is_none() {
            return Err(format!("source not found for panel {} (fn {})", panel.id, panel.fn_name));
        }
    }
    Ok(())
}
