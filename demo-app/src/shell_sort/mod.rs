//! The Shell sort panel:
//!
//! - `panel-shell-sort` / `#shell-sort-diagram` — [`build_shell_sort_demo`]: Shell sort of ten values drawn as bars,
//!   one step at a time.

mod code;

use crate::{
    sorting::{self, Algorithm},
    source_file,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the algorithm, then what every sorting demo shares: the bars, the buttons and the walk.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("shell_sort/code/mod.rs", "code/mod.rs"),
    source_file!("sorting/mod.rs", "../sorting/mod.rs"),
    source_file!("sorting/bars.rs", "../sorting/bars.rs"),
    source_file!("sorting/controls.rs", "../sorting/controls.rs"),
    source_file!("sorting/array.rs", "../sorting/array.rs"),
    source_file!("shell_sort/mod.rs", "mod.rs"),
];

/// Shell sort, as the shared sorting demo needs it: the start of the ids of the panel's elements, and the steps.
static ALGORITHM: Algorithm = Algorithm {
    prefix: "shell-sort",
    frames: code::frames,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Shell sort demo: ten values as thin bars on a common baseline, each as tall as its value, sorted one step
/// at a time. Everything about drawing, stepping and announcing is shared with the other sorting demos: see
/// [`sorting`]. What is particular to Shell sort is in [`code::frames`].
///
/// - A **green** bar is the element being put in order with the elements a gap before it. If it has to move it goes a
///   whole gap to the left, and stays green.
/// - A **blue** bar is the element a gap before the green one, which it is compared with. If they are the wrong way
///   round, the next step swaps them, and the bars slide to change places.
/// - A **teal** bar is one of the elements a gap apart, up to the green one, that are being put in order with each
///   other.
/// - A **lilac** bar is in its final place, which is all of them, at the end.
///
/// The gaps are 5, 2 and 1 for ten elements. A selection toolbar at the top steps through the sort, and the buttons
/// choose another array. Every step is announced in a live region outside the scene. Every value is worked out in plain
/// Rust by [`code::frames`]. The library does no sorting.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub(crate) fn build_shell_sort_demo() -> Result<(), String> {
    sorting::build_demo(&ALGORITHM)
}
