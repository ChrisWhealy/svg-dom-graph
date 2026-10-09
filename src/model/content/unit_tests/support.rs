//! Helpers shared by every test module here.

use super::super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Collects every one of `content`'s own cells into a `Vec<String>`, via `DataNodeContent::for_each_cell_string`.
/// Production code streams instead of collecting. A test asserting on formatting correctness reads far more naturally
/// against a plain `Vec<String>` equality check than against a sequence of callback invocations.
pub(super) fn cells(content: &DataNodeContent) -> Vec<String> {
    let mut out = Vec::new();
    let mut scratch = String::new();
    content.for_each_cell_string(&mut scratch, |_, cell_text| out.push(cell_text.to_owned()));
    out
}
