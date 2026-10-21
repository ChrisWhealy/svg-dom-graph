//! The Hamming(7,4) panel:
//!
//! - `panel-hamming` / `#hamming-diagram` — [`build_hamming_demo`]: four data bits protected by three parity bits, so
//!   that one flipped bit can be found and put right. A button opens the scene in a window of its own:
//!   [`build_hamming_window`].
//!
//! The pieces of the demo:
//!
//! - `code`: the code itself, in plain Rust.
//! - `words`: the note under each box.
//! - `walk` and `progress`: the steps of the calculation, and drawing the boxes a step has not reached faint.
//! - `snake`: laying the path of boxes out as compactly as it can be.
//! - `diagram`: drawing the scene.
//! - `controls`: the buttons in the scene that choose the data and introduce an error.

mod code;
mod controls;
mod diagram;
mod progress;
mod snake;
mod walk;
mod words;

use crate::{
    source_file,
    util::{clear_error, document, fit_window_to_stage, report_error, stringify, wire_open_window},
};
use code::{CODEWORD_LEN, Transmission};
use std::cell::RefCell;
use svg_dom_graph::scene::{Scene, Selection, SelectionToolbarOptions, Side};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the code, the layout, the steps, then the scene that draws it.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("hamming/code/mod.rs", "code/mod.rs"),
    source_file!("hamming/snake/mod.rs", "snake/mod.rs"),
    source_file!("hamming/walk/mod.rs", "walk/mod.rs"),
    source_file!("hamming/words/mod.rs", "words/mod.rs"),
    source_file!("hamming/progress.rs", "progress.rs"),
    source_file!("hamming/diagram.rs", "diagram.rs"),
    source_file!("hamming/controls.rs", "controls.rs"),
    source_file!("hamming/mod.rs", "mod.rs"),
];

/// The page [`build_hamming_demo`]'s button opens, and the window's own name, so a second click reuses it.
const WINDOW_PAGE: &str = "hamming-window.html";
const WINDOW_NAME: &str = "svg-dom-graph-hamming";

const STAGE_ID: &str = "hamming-diagram-stage";
const DIAGRAM_ID: &str = "hamming-diagram";

/// The id of the banner in the window page that a failure is shown in.
const ERROR_ID: &str = "hamming-window-error";

/// The id of the live region in the window page that announces a change.
const ANNOUNCEMENT_ID: &str = "hamming-announcement";

/// The usual width of the note under a box, and the margin round the scene.
const NOTE_WIDTH: f64 = 340.0;
const MARGIN: f64 = 20.0;

/// Where the controls start: below the stepping toolbar, which is fixed to the top edge.
const TOP: f64 = 64.0;

/// The room east of the scene for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

/// The data the demo opens on: the one in most explanations of the code.
const INITIAL_DATA: [u8; 4] = [1, 0, 1, 1];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the user has chosen, which stays across redraws.
struct State {
    /// The four data bits, d1 to d4.
    data: [u8; 4],
    /// The position, from 1, of the bit flipped in transmission, if one is.
    error: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the callbacks need to find again: the scene on the page, and where the walk is.
struct Demo {
    scene: Scene,
    /// The walk position the scene was drawn at.
    position: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    // Keeps the demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does. Every change draws a new one.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
    static STATE: RefCell<State> = const { RefCell::new(State { data: INITIAL_DATA, error: None }) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for what the user has chosen at walk position `position`, wires its stepping toolbar, and replaces
/// whatever scene was there. It keeps the zoom and pan, and the keyboard focus on a button that the old scene had.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
fn build(position: usize) -> Result<(), String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let focused = focused_selector();
    let (data, error) = STATE.with_borrow(|state| (state.data, state.error));
    let transmission = Transmission::new(data, error);

    let drawn = diagram::draw(&transmission, position, &controls::describe(transmission.error))?;
    if let Some(view) = view {
        drawn.scene.set_view(view).map_err(stringify)?;
    }
    drawn
        .scene
        .show_selection_toolbar(
            drawn.step,
            SelectionToolbarOptions::new(Side::North),
            |_scene, _node, transition| match transition.to {
                Some(position) => go(position),
                // "Restart" and "Previous" from the first position are the same transition, so both land here, and the
                // demo cannot tell them apart. Stepping back from the start is therefore a restart.
                None => restart(),
            },
        )
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    drawn
        .scene
        .set_selection(drawn.step, Selection::Cell(drawn.position))
        .map_err(stringify)?;
    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene: drawn.scene,
            position: drawn.position,
        });
    });
    if let Some(selector) = focused {
        focus(&selector);
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again at `position`, and gives the new button of the same `action` the keyboard focus, as the old
/// one had it. Without that, a keyboard user would find that focus had gone back to the start of the page. Says whether
/// the scene could be drawn.
fn redraw(action: &str, position: usize) -> bool {
    let result = build(position);
    let worked = result.is_ok();
    report(result, "Could not draw the change");
    if worked {
        focus(&format!("#{DIAGRAM_ID} g[data-action=\"{action}\"]"));
    }
    worked
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the diagram at walk position `position`, and says whether that worked. A failure is shown in the window's own
/// error banner, since the toolbar's own callback has nowhere to return it to. The banner clears again at the next step
/// that works.
fn show_position(position: usize) -> bool {
    let result = build(position);
    let worked = result.is_ok();
    report(result, &format!("This step failed (position {position})"));
    worked
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`, and announces it. The toolbar's own buttons have fixed names, so
/// nothing else tells a screen reader where the walk has got to.
fn go(position: usize) {
    if show_position(position) {
        controls::announce(&walk::announcement(position));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" button was pressed, or "Previous" from the first position, which the toolbar reports in the
/// same way. Puts the diagram back to the start of the walk, and removes the flipped bit, so that the walk starts again
/// from nothing. The data bits stay as they are.
///
/// There is one announcement, saying both that the error is gone and where the walk is. Two in a row would leave a
/// screen reader saying only the second.
fn restart() {
    STATE.with_borrow_mut(|state| state.error = None);
    if show_position(0) {
        controls::announce(&format!("{} {}", controls::describe(None), walk::announcement(0)));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The position of the flipped bit that the page's address asks for, as in `hamming-window.html?error=5`, if it asks for
/// one that exists. The page opens with that bit already flipped. It gives a browser test the one position it wants,
/// where the button picks at random. There is no control for it on the page.
fn error_in_address() -> Option<usize> {
    let search = web_sys::window()?.location().search().ok()?;
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("error="))
        .and_then(|n| n.parse().ok())
        .filter(|position| (1..=CODEWORD_LEN).contains(position))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a failure in the window's own error banner, or clears the banner after a success.
fn report(result: Result<(), String>, what: &str) {
    match result {
        Ok(()) => clear_error(ERROR_ID),
        Err(e) => report_error(ERROR_ID, &format!("{what}: {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A selector for the button in the diagram that has keyboard focus, if one does: by its action if it is one of this
/// demo's own buttons, and otherwise by its accessible name, as a toolbar button is.
fn focused_selector() -> Option<String> {
    let document = document().ok()?;
    let active = document.active_element()?;
    document.get_element_by_id(DIAGRAM_ID)?.contains(Some(&active)).then_some(())?;
    if let Some(action) = active.get_attribute("data-action") {
        return Some(format!("#{DIAGRAM_ID} g[data-action=\"{action}\"]"));
    }
    active
        .get_attribute("aria-label")
        .map(|label| format!("#{DIAGRAM_ID} g[aria-label=\"{label}\"]"))
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Hamming(7,4) scene in `hamming-window.html`, at the start of the walk, wires its stepping toolbar and
/// its buttons, and resizes the window to fit the scene.
///
/// The path of boxes is the calculation:
///
/// 1. The four data bits. The buttons along the top set each one.
/// 2. The three parity bits, each making a group of bits hold an even number of 1s.
/// 3. The codeword of seven bits, with the parity bits at positions 1, 2 and 4.
/// 4. The received codeword, in which the button "Flip a random bit" has flipped one bit on the way.
/// 5. The check bits, which are the three groups worked out again over the bits that arrived.
/// 6. The syndrome, which is the three checks read as a binary number.
/// 7. The error position, which is that number: the position of the flipped bit, or 0 if none.
/// 8. The corrected codeword, with the bit at that position flipped back.
/// 9. The recovered data, read from positions 3, 5, 6 and 7.
///
/// That is nine boxes, and the calculation passes through all of them, so the scene is laid out as a path, not as
/// columns chosen in advance. [`snake::best_columns`] chooses how many boxes go along a row, and the path runs along
/// one row and back along the next, so that it takes as little room as it can. Each box has a note under it that says
/// how its bits are worked out, and the connector between two boxes along a row is straight. At the end of a row it
/// loops round the outside.
///
/// # Stepping through it
///
/// A selection toolbar at the top steps through the path, as the other demos do. The boxes that a step has not reached
/// are drawn faint, and the one it has just reached is marked. "Restart" starts again, and removes the flipped bit.
/// Flipping a bit moves the walk on to the received codeword, if it has not got that far, since the error cannot be
/// seen before then.
///
/// # Errors
///
/// Returns `Err` if the window page is missing a part, or if any library call fails.
pub(crate) fn build_hamming_window() -> Result<(), String> {
    let document = document()?;
    controls::watch_buttons(&document)?;
    if let Some(position) = error_in_address() {
        STATE.with_borrow_mut(|state| state.error = Some(position));
    }
    build(0)?;
    fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gallery panel keeps only the description, plus a button that opens the scene in a window of its own:
/// `hamming-window.html`, which runs [`build_hamming_window`] there. A named window is reused. Pressing the button
/// again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#hamming-open`, or if a listener could not be attached to it.
pub(crate) fn build_hamming_demo() -> Result<(), String> {
    wire_open_window(&document()?, "hamming-open", WINDOW_PAGE, WINDOW_NAME)
}
