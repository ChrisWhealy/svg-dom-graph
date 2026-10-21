//! The Bubble sort panel:
//!
//! - `panel-bubble-sort` / `#bubble-sort-diagram` — [`build_bubble_sort_demo`]: Bubble sort of ten values drawn as
//!   bars, one step at a time.

mod code;

use crate::{
    sorting::{self, Algorithm},
    source_file,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the algorithm, then what every sorting demo shares: the bars, the buttons and the walk.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("bubble_sort/code/mod.rs", "code/mod.rs"),
    source_file!("sorting/mod.rs", "../sorting/mod.rs"),
    source_file!("sorting/bars.rs", "../sorting/bars.rs"),
    source_file!("sorting/controls.rs", "../sorting/controls.rs"),
    source_file!("sorting/array.rs", "../sorting/array.rs"),
    source_file!("bubble_sort/mod.rs", "mod.rs"),
];

/// Bubble sort, as the shared sorting demo needs it: the start of the ids of the panel's elements, and the steps.
static ALGORITHM: Algorithm = Algorithm {
    prefix: "bubble-sort",
    frames: code::frames,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Bubble sort demo: ten values as thin bars on a common baseline, each as tall as its value, sorted one
/// step at a time. Everything about drawing, stepping and announcing is shared with the other sorting demos: see
/// [`sorting`]. What is particular to Bubble sort is in [`code::frames`].
///
/// - A **green** bar is the left one of the two neighbours being compared. If they are the wrong way round, it is
///   carried a place to the right, and stays green.
/// - A **blue** bar is the right one of the two neighbours being compared. If the pair is the wrong way round, the next
///   step swaps the two, and the bars slide to change places.
/// - A **lilac** bar is in its final place: after each pass, the largest element left is carried to the end.
///
/// A selection toolbar at the top steps through the sort, and the buttons choose another array: a random one, one
/// already sorted, and one in reverse order. An array already in order takes a single pass, and one in reverse is the
/// worst case, with every pair swapped. Every step is announced in a live region outside the scene. Every value is
/// worked out in plain Rust by [`code::frames`]. The library does no sorting.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub(crate) fn build_bubble_sort_demo() -> Result<(), String> {
    sorting::build_demo(&ALGORITHM)
}
