//! `demo_fn_source`'s textual extraction out of each demo module's own `SOURCE` is a separate mechanism from
//! `DEMO_PANELS` itself. Compilation only proves each name in `demo_gallery!` resolves to a real function, not that
//! `demo_fn_source` can find its source text there.
//!
//! A function that compiles fine but breaks the extraction would still pass `cargo build`. Examples are one renamed
//! without updating `demo_gallery!`, or reformatted in a way that moves its closing brace out of column 0. It would
//! only surface as `append_demo_source` returning `Err` — a startup failure a user would only notice by looking at the
//! browser console. This test is what actually catches that class of drift, on every `cargo test` run.

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_extra_source_file_is_named_and_not_empty() -> Result<(), String> {
    for panel in DEMO_PANELS {
        for file in panel.files {
            if file.path.is_empty() || file.source.trim().is_empty() {
                return Err(format!("panel {} has an empty source file entry ({:?})", panel.id, file.path));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_sha2_panel_shows_the_calculation_behind_its_one_function() -> Result<(), String> {
    let panel = DEMO_PANELS
        .iter()
        .find(|p| p.id == "panel-sha2-256")
        .ok_or("panel-sha2-256 is not registered")?;
    let shows = |needle: &str| panel.files.iter().any(|f| f.source.contains(needle));
    if shows("fn big_sigma0") && shows("fn expanded_word") && shows("pub(in crate::sha2_256) fn shown") {
        Ok(())
    } else {
        Err("the SHA-256 panel should show its algorithm and its walk".to_owned())
    }
}
