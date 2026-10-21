//! The Quicksort panel:
//!
//! - `panel-quicksort` / `#quicksort-diagram` — [`build_quicksort_demo`]: Quicksort of ten values drawn as bars, one
//!   step at a time.

mod code;

use crate::{
    sorting::{self, Algorithm},
    source_file,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the algorithm, then what every sorting demo shares: the bars, the buttons and the walk.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("quicksort/code/mod.rs", "code/mod.rs"),
    source_file!("sorting/mod.rs", "../sorting/mod.rs"),
    source_file!("sorting/bars.rs", "../sorting/bars.rs"),
    source_file!("sorting/controls.rs", "../sorting/controls.rs"),
    source_file!("sorting/array.rs", "../sorting/array.rs"),
    source_file!("quicksort/mod.rs", "mod.rs"),
];

/// Quicksort, as the shared sorting demo needs it: the start of the ids of the panel's elements, and the steps.
static ALGORITHM: Algorithm = Algorithm {
    prefix: "quicksort",
    frames: code::frames,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Quicksort demo: ten values as thin bars on a common baseline, each as tall as its value, sorted one step
/// at a time. Everything about drawing, stepping and announcing is shared with the other sorting demos: see
/// [`sorting`]. What is particular to Quicksort is in [`code::frames`].
///
/// - A **red** bar is the pivot, the last element of the range being sorted. It stays red until it is swapped into
///   place.
/// - A **green** bar is the element the pass is looking at. It stays green if it has to move.
/// - A **blue** bar is the element the green one will swap with. The next step is the swap, and the two bars slide to
///   change places.
/// - A **teal** bar is one already found to be smaller than the pivot, gathered at the front of the range.
/// - A **pale** bar is outside the range being sorted.
/// - A **lilac** bar is in its final place.
///
/// So the sort can be followed by the bars changing places. A selection toolbar at the top steps through it, and the
/// buttons choose another array: a random one, one already sorted, and one in reverse order. The last two are the worst
/// cases for a pivot that is always the last element. Every step is announced in a live region outside the scene. Every
/// value is worked out in plain Rust by [`code::steps`]. The library does no sorting.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub(crate) fn build_quicksort_demo() -> Result<(), String> {
    sorting::build_demo(&ALGORITHM)
}
