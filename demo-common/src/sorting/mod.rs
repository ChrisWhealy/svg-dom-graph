//! What every sorting demo shares: the array, the bars it is drawn as, the buttons that choose another array, the
//! stepping toolbar that walks through the sort, and the announcements for a screen reader.
//!
//! A sorting demo supplies only an [`Algorithm`]: the id its panel's elements start with, and a function that sorts an
//! array and records every step as a [`Frame`]. A frame is the value of each bar, a [`Mark`] for each bar that says
//! what the algorithm is doing with it, and a sentence about the step. This module draws the frames, and the demo does
//! nothing else. So Bubble sort or Shell sort needs only the function that records its steps.
//!
//! The elements are drawn as thin nodes standing on a common baseline across the scene, each as tall as its value. A
//! swap shows as two bars changing places. The colours say what the algorithm is doing: see [`Mark`].

pub mod array;
mod bars;
mod controls;
pub mod testing;

use crate::util::{clear_error, document, report_error, stringify};
use std::cell::RefCell;
use svg_dom_graph::scene::{Scene, Selection, SelectionToolbarOptions, Side};
use wasm_bindgen::JsCast;

/// The room east of the scene for the zoom toolbar, and where the buttons start: below the stepping toolbar, which is
/// fixed to the top edge.
const TOOLBAR_ROOM: f64 = 80.0;
const TOP: f64 = 64.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the algorithm is doing with one bar, which sets its colour.
///
/// A colour alone says nothing to assistive technology, so the note of a step says what it did to which values, and the
/// live region reads the note out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Nothing special: the usual light blue.
    Plain,
    /// The element the algorithm is comparing the others with. Red.
    Pivot,
    /// The element the algorithm is looking at, or has just moved. Green.
    Examined,
    /// The element the examined one is being compared with, or is to swap with. Blue.
    Candidate,
    /// An element found to belong with others, such as those smaller than the pivot. Teal.
    Group,
    /// An element that has reached its final place. Lilac.
    Done,
    /// An element outside the part of the array being worked on. Pale.
    Outside,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of a sort, as it is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The value of each element, left to right, which is the height of its bar.
    pub values: Vec<u8>,
    /// What the algorithm is doing with each element. There is one for each value.
    pub marks: Vec<Mark>,
    /// The two places whose elements this step swapped, if it swapped any. The step is drawn with the bars where they
    /// have gone, and the two bars slide from where they were.
    pub swap: Option<(usize, usize)>,
    /// What the step did, in a sentence or two of plain text.
    pub note: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A sorting algorithm, as the demo needs it.
pub struct Algorithm {
    /// What the ids of the panel's elements start with: `quicksort` for `quicksort-diagram-stage`, `quicksort-diagram`,
    /// `quicksort-error` and `quicksort-announcement`.
    pub prefix: &'static str,
    /// Every step of sorting an array, first the array as it was given and last the sorted array. There is at least
    /// one.
    pub frames: fn(&[u8]) -> Vec<Frame>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a demo needs to find again between steps.
struct Runtime {
    /// Which demo this is, by its [`Algorithm::prefix`].
    prefix: &'static str,
    /// The scene on the page. Every step draws a new one, so this keeps the last one alive.
    scene: Option<Scene>,
    /// The elements being sorted, which stay across steps.
    array: Vec<u8>,
    /// The step the scene on the page is at, if there is one.
    position: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    static RUNTIMES: RefCell<Vec<Runtime>> = const { RefCell::new(Vec::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `f` on the state of `algorithm`'s demo, which is made if it is not there yet.
fn with_runtime<T>(algorithm: &Algorithm, f: impl FnOnce(&mut Runtime) -> T) -> T {
    RUNTIMES.with_borrow_mut(|all| {
        let index = all.iter().position(|r| r.prefix == algorithm.prefix).unwrap_or_else(|| {
            all.push(Runtime {
                prefix: algorithm.prefix,
                scene: None,
                array: array::INITIAL.to_vec(),
                position: None,
            });
            all.len() - 1
        });
        f(&mut all[index])
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a screen reader is told when the walk is at `position` of `count`: how far along it is, and what the step did.
fn announcement(position: usize, count: usize, note: &str) -> String {
    format!("Step {} of {count}. {note}", position + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws `algorithm`'s scene at step `position` of the sort of the array the user has, wires its stepping toolbar, and
/// replaces whatever scene was there. It keeps the zoom and pan, and the keyboard focus on a button that the old scene
/// had. Returns what a screen reader should be told.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
fn build(algorithm: &'static Algorithm, position: usize) -> Result<String, String> {
    let diagram_id = format!("{}-diagram", algorithm.prefix);
    let view = with_runtime(algorithm, |runtime| runtime.scene.as_ref().map(Scene::view));
    let focused = focused_selector(&diagram_id);
    let values = with_runtime(algorithm, |runtime| runtime.array.clone());
    let frames = (algorithm.frames)(&values);
    let position = position.min(frames.len().saturating_sub(1));
    let frame = frames
        .get(position)
        .ok_or_else(|| "the algorithm recorded no steps".to_owned())?;

    // A swap slides the two bars only when the walk has come one step forward to it. Arriving any other way, such as
    // stepping back over one, draws the bars where they are, since no one saw them where they were.
    let came_forward = with_runtime(algorithm, |runtime| {
        runtime.position.is_some_and(|before| before + 1 == position)
    });
    let drawn = bars::draw(algorithm.prefix, frame, position, frames.len(), came_forward)?;
    if let Some(view) = view {
        drawn.scene.set_view(view).map_err(stringify)?;
    }
    drawn
        .scene
        .show_selection_toolbar(
            drawn.step,
            SelectionToolbarOptions::new(Side::North),
            move |_scene, _node, transition| match transition.to {
                Some(position) => go(algorithm, position, false),
                // "Restart" and "Previous" from the first position are the same transition, so both land here. Stepping
                // back from the start is therefore a restart.
                None => go(algorithm, 0, false),
            },
        )
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    drawn
        .scene
        .set_selection(drawn.step, Selection::Cell(position))
        .map_err(stringify)?;
    with_runtime(algorithm, |runtime| {
        runtime.scene = Some(drawn.scene);
        runtime.position = Some(position);
    });
    if let Some(selector) = focused {
        focus(&selector);
    }
    Ok(announcement(position, frames.len(), &frame.note))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves `algorithm`'s diagram to step `position`, and announces it. `new_array` is whether the user has just chosen
/// another array, which the announcement then says first. A failure is shown in the panel's own error banner, since the
/// toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(algorithm: &'static Algorithm, position: usize, new_array: bool) {
    let (error_id, announcement_id) = (
        format!("{}-error", algorithm.prefix),
        format!("{}-announcement", algorithm.prefix),
    );
    match build(algorithm, position) {
        Ok(said) => {
            clear_error(&error_id);
            let said = if new_array {
                let values = with_runtime(algorithm, |runtime| runtime.array.clone());
                format!("{} {said}", controls::describe(&values))
            } else {
                said
            };
            controls::announce(&announcement_id, &said);
        },
        Err(e) => report_error(&error_id, &format!("This step failed (position {position}): {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A selector for the button in the diagram `diagram_id` that has keyboard focus, if one does: by its action if it is
/// one of the demo's own buttons, and otherwise by its accessible name, as a toolbar button is.
fn focused_selector(diagram_id: &str) -> Option<String> {
    let document = document().ok()?;
    let active = document.active_element()?;
    document.get_element_by_id(diagram_id)?.contains(Some(&active)).then_some(())?;
    if let Some(action) = active.get_attribute("data-action") {
        return Some(format!("#{diagram_id} g[data-action=\"{action}\"]"));
    }
    active
        .get_attribute("aria-label")
        .map(|label| format!("#{diagram_id} g[aria-label=\"{label}\"]"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives keyboard focus to the element `selector` finds, if there is one and it can take focus. A toolbar button that
/// is disabled now is left alone.
fn focus(selector: &str) {
    let Ok(document) = document() else { return };
    if let Ok(Some(element)) = document.query_selector(selector) {
        if let Ok(element) = element.dyn_into::<web_sys::SvgElement>() {
            let _ = element.focus();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo of `algorithm`: the bars of the array it opens on, the buttons that choose another, and the stepping
/// toolbar. This is the whole of a sorting demo's own function.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, or if any library call fails.
pub fn build_demo(algorithm: &'static Algorithm) -> Result<(), String> {
    let document = document()?;
    controls::watch_buttons(&document, algorithm)?;
    with_runtime(algorithm, |runtime| runtime.array = array::INITIAL.to_vec());
    let said = build(algorithm, 0)?;
    controls::announce(&format!("{}-announcement", algorithm.prefix), &said);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
