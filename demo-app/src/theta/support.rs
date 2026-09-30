//! Shared by [`super::theta_c`] and [`super::theta_d`]: the live state their own selection toolbars carry across
//! steps, and the "clone a fresh `<svg>` and give it a never-reused id" dance every step needs — see
//! [`super::theta_c::rebuild_child`]'s own doc comment for why a step cannot redraw its own `<svg>` in place.

use crate::util::required_element;
use std::cell::Cell;
use wasm_bindgen::JsCast;

thread_local! {
    // A fresh numeric suffix for every step's own nested child `<svg>` id, shared by both `ThetaC` and `ThetaD` —
    // see [`next_child_svg_id`]'s own doc comment.
    static NEXT_CHILD_SVG_SUFFIX: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state a nested child's own selection toolbar shares across steps — the same shape as
/// `crate::selection::ThetaCDemo`, reused here for both `ThetaC` and `ThetaD`, each stepped from inside its own
/// nested view instead of standalone.
pub(super) struct SteppedChildState {
    /// Row `i`'s own result — see `crate::selection::theta_c_outputs`'s own doc comment (for `ThetaC`) or
    /// [`super::theta_d::outputs`]'s own (for `ThetaD`).
    pub(super) outputs: [u64; 5],
    /// `written[i]` is `true` once row `i` has been stepped into going forward, and not since stepped away from
    /// going backward — see `crate::selection::ThetaCDemo::written`'s own doc comment.
    pub(super) written: [bool; 5],
    /// The id of whichever `<svg>` currently backs the nested child — see [`super::theta_c::rebuild_child`]'s/
    /// [`super::theta_d::rebuild_child`]'s own doc comment for why every step needs a fresh one.
    pub(super) child_svg_id: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A fresh, never-reused id for this step's own nested child `<svg>`, prefixed with `prefix` (`"theta-thetac-child"`
/// or `"theta-thetad-child"`). Shares one counter across both `ThetaC` and `ThetaD`, so every id handed out is
/// unique regardless of which nested child it backs.
pub(super) fn next_child_svg_id(prefix: &str) -> String {
    NEXT_CHILD_SVG_SUFFIX.with(|counter| {
        let n = counter.get();
        counter.set(n + 1);
        format!("{prefix}-{n}")
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Clones `previous_id`'s own `<svg>` — shallow, attributes only, no content — gives the clone `next_id`, and
/// inserts it as `previous_id`'s own next sibling. Inherits size, `viewBox`, and `class="nested-scene"` from
/// whichever element is currently in the DOM, rather than a second, hardcoded copy of them.
///
/// # Errors
///
/// Returns `Err` if `previous_id` names no element currently in the DOM, or if cloning or inserting the fresh
/// element fails.
pub(super) fn create_child_svg(document: &web_sys::Document, previous_id: &str, next_id: &str) -> Result<(), String> {
    let previous = required_element(document, previous_id)?;
    let fresh = previous
        .clone_node_with_deep(false)
        .map_err(|e| format!("could not clone #{previous_id} for its own next step: {e:?}"))?;
    let fresh: web_sys::Element = fresh
        .dyn_into()
        .map_err(|_| "cloning the nested child <svg> did not produce an Element".to_string())?;
    fresh
        .set_attribute("id", next_id)
        .map_err(|e| format!("could not id the fresh nested child <svg>: {e:?}"))?;
    previous
        .after_with_node_1(&fresh)
        .map_err(|e| format!("could not insert the fresh nested child <svg>: {e:?}"))
}
