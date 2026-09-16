//! `demo_fn_source`'s textual extraction out of `LIB_SOURCE` is a separate mechanism from `DEMO_PANELS` itself —
//! compilation only proves each name in `demo_gallery!` resolves to a real function, not that `demo_fn_source` can find
//! its source text. A function that compiles fine but breaks the extraction (renamed without updating `demo_gallery!`,
//! or reformatted in a way that moves its closing brace out of column 0) would still pass `cargo build`, and only
//! surface as `append_demo_source` returning `Err` — a startup failure a user would only notice by looking at the
//! browser console. This test is what actually catches that class of drift, on every `cargo test` run.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_registered_demo_has_extractable_source() -> Result<(), String> {
    for &(panel_id, _, fn_name) in DEMO_PANELS {
        if demo_fn_source(fn_name).is_none() {
            return Err(format!("source not found for panel {panel_id} (fn {fn_name})"));
        }
    }
    Ok(())
}
