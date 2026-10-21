//! The CRC-32 panel:
//!
//! - `panel-crc32` / `#crc32-diagram` — [`build_crc32_demo`]: the CRC-32 of typed text, a byte at a time, with a button
//!   that opens the scene in a window of its own: [`build_crc32_window`].
//!
//! The pieces of the demo:
//!
//! - `algorithm`: the CRC-32 calculation, and finding a flipped bit from the syndrome. Plain Rust.
//! - `flips`: the bits flipped between the sender and the receiver.
//! - `transmission`: what the receiver makes of them.
//! - `words`: the explanation beside each box.
//! - `walk` and `progress`: the steps of the calculation, and drawing the boxes a step has not reached faint.
//! - `column`: laying one column of boxes out on an axis.
//! - `diagram`: drawing the scene.
//! - `controls`: the buttons in the scene that introduce an error.

mod algorithm;
mod column;
mod controls;
mod diagram;
mod flips;
mod progress;
mod transmission;
mod walk;
mod words;

use crate::{
    base64::encode::utf8_prefix,
    source_file,
    util::{clear_error, document, fit_window_to_stage, report_error, required_element, stringify, wire_open_window},
};
use flips::Flips;
use std::cell::{Cell, RefCell};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, Selection, SelectionStride, SelectionToolbarOptions, Side},
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the calculation, the steps, then the scene that draws it.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("crc32/algorithm.rs", "algorithm.rs"),
    source_file!("crc32/flips.rs", "flips.rs"),
    source_file!("crc32/transmission.rs", "transmission.rs"),
    source_file!("crc32/words.rs", "words.rs"),
    source_file!("crc32/walk/mod.rs", "walk/mod.rs"),
    source_file!("crc32/progress.rs", "progress.rs"),
    source_file!("crc32/column.rs", "column.rs"),
    source_file!("crc32/diagram.rs", "diagram.rs"),
    source_file!("crc32/controls.rs", "controls.rs"),
    source_file!("crc32/mod.rs", "mod.rs"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    // Keeps the demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does. Every change draws a new one.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
    // Whether the user is part way through composing text with an input method editor.
    static COMPOSING: Cell<bool> = const { Cell::new(false) };
    // The bits flipped in transmission, in the data and in the CRC. They stay across redraws.
    static FLIPS: RefCell<Flips> = RefCell::new(Flips::default());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page [`build_crc32_demo`]'s button opens.
const WINDOW_PAGE: &str = "crc32-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-crc32";

const STAGE_ID: &str = "crc32-diagram-stage";

/// The id of the banner in the window page that a failure is shown in.
const ERROR_ID: &str = "crc32-window-error";
const DIAGRAM_ID: &str = "crc32-diagram";

/// The text the demo opens on.
const INITIAL_TEXT: &str = "Has this text been corrupted during transmission?";

/// How many bytes of the typed text the checksum covers. Each byte is a step in the diagram, so the limit keeps the
/// diagram to a manageable size. Text past it is not covered, and the cut falls between characters.
const INPUT_LEN: usize = 64;

/// How wide the text box is when there is no hex dump to be as wide as, and the note on its right.
const INPUT_WIDTH: f64 = 580.0;
const NOTE_WIDTH: f64 = 260.0;

/// How wide the explanation beside each box is: on the left of the sender's column, which has the most to say, and on
/// the right of the receiver's.
const SENDER_EXPLANATION_WIDTH: f64 = 570.0;
const RECEIVER_EXPLANATION_WIDTH: f64 = 380.0;

/// About how tall the text box is. The library draws it at a height of its own.
const TYPICAL_INPUT_HEIGHT: f64 = 70.0;

/// Where the first row starts: below the stepping toolbar, which is fixed to the top edge.
const TOP: f64 = 64.0;

/// The gap between two columns, wide enough for a connector, and the room east of the last for the zoom toolbar.
const GAP: f64 = 60.0;
const TOOLBAR_ROOM: f64 = 80.0;

/// How much the diagram is scaled down, text and all. It is the largest of the demos, and it has a window to itself.
const SCALE: f64 = 0.85;

/// The id of the live region in the window page that announces a change of the error.
const ANNOUNCEMENT_ID: &str = "crc32-announcement";

/// Finds the text box that holds the text to checksum.
const TEXT_BOX: &str = "#crc32-diagram g[aria-label^=\"Text to checksum\"] input";

/// The space between one row of boxes and the next, wide enough for a connector.
const ROW_GAP: f64 = 50.0;

/// How many values the table index, the table entry and the register lay out on a row.
const INDEX_COLUMNS: usize = 8;
const WORD_COLUMNS: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the callbacks need to find again: the scene on the page, and what it shows.
struct Demo {
    scene: Scene,
    /// The text the scene was drawn for.
    text: String,
    /// The scene's text input node.
    input: NodeId,
    /// The walk position the scene was drawn at.
    position: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `text` at walk position `position`, wires its stepping toolbar, and replaces whatever scene was
/// there. It keeps the zoom and pan, and the keyboard focus on a button that the old scene had. Returns the new text
/// input node.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
fn build(text: &str, position: usize) -> Result<NodeId, String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let focused = focused_button();

    // A bit can only be flipped in a byte that exists. Those left over from longer text are dropped.
    let flips = FLIPS.with_borrow_mut(|flips| {
        flips.keep_bytes_below(utf8_prefix(text, INPUT_LEN).len());
        flips.clone()
    });
    let drawn = diagram::draw(text, position, &flips)?;
    if let Some(view) = view {
        drawn.scene.set_view(view).map_err(stringify)?;
    }
    drawn
        .scene
        .show_selection_toolbar(
            drawn.step,
            SelectionToolbarOptions::new(Side::North).with_stride(SelectionStride::new(walk::STRIDE, "8")),
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

    let input = drawn.input;
    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene: drawn.scene,
            text: text.to_owned(),
            input,
            position: drawn.position,
        });
    });
    if let Some(label) = focused {
        focus_button(&label);
    }
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`. A failure is shown in the window's own error banner, since the
/// toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(position: usize) {
    let text = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.text.clone()));
    report(
        text.ok_or_else(|| "the diagram is not built".to_owned())
            .and_then(|text| build(&text, position).map(|_| ())),
        &format!("This step failed (position {position})"),
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" button was pressed. Puts the diagram back to the start of the walk, and removes every
/// flipped bit, so that the walk starts again from nothing.
fn restart() {
    FLIPS.with_borrow_mut(|flips| *flips = Flips::default());
    go(0);
    controls::announce(&FLIPS.with_borrow(Flips::describe));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible name of the toolbar button that has keyboard focus, if one in the diagram does.
fn focused_button() -> Option<String> {
    let document = document().ok()?;
    let active = document.active_element()?;
    let diagram = document.get_element_by_id(DIAGRAM_ID)?;
    if diagram.contains(Some(&active)) {
        active.get_attribute("aria-label")
    } else {
        None
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives keyboard focus to the button in the diagram named `label`, if there is one and it can take focus. A button
/// that is disabled now is left alone.
fn focus_button(label: &str) {
    let Ok(document) = document() else { return };
    let selector = format!("#{DIAGRAM_ID} g[aria-label=\"{label}\"]");
    if let Ok(Some(button)) = document.query_selector(&selector) {
        if let Ok(button) = button.dyn_into::<web_sys::SvgElement>() {
            let _ = button.focus();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box now holds `text`. Draws the scene again for it, unless the user is composing text with an input method
/// editor. The scene is replaced along with its text box, and a composition does not survive that. So the scene waits
/// until the composition ends: see [`composition_ended`].
fn typed(id: NodeId, text: &str) {
    if !COMPOSING.get() {
        redraw(id, text);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again for `text`, starting the walk again, then gives the new text box the keyboard focus, with the
/// caret where it was, so that typing carries on in it. A failure is shown in the window's own error banner, since the
/// text box's own callback has nowhere to return it to, and the banner clears again at the next change that works.
fn redraw(id: NodeId, text: &str) {
    let caret = DEMO.with_borrow(|slot| slot.as_ref().and_then(|d| d.scene.text_input_caret(id).ok()));
    let result = build(text, 0).and_then(|input| {
        DEMO.with_borrow(|slot| match slot {
            Some(demo) => demo.scene.focus_text_input(input, caret).map_err(stringify),
            None => Ok(()),
        })
    });
    report(result, "Could not draw the text");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a failure in the window's own error banner, or clears the banner after a success.
///
/// A scene that fails part way has already had its old one cleared. Without this the diagram would be left incomplete
/// with no sign of why.
fn report(result: Result<(), String>, what: &str) {
    match result {
        Ok(()) => clear_error(ERROR_ID),
        Err(e) => report_error(ERROR_ID, &format!("{what}: {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The user has finished composing. Draws the scene for the text the text box holds now, if it is not the text the
/// scene was drawn for. The browser can send the last `input` event either before or after `compositionend`, so this
/// looks once the event has been handled, and draws nothing if the `input` event has already done so.
fn composition_ended() {
    let value = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(TEXT_BOX).ok().flatten())
        .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value());
    let input = DEMO.with_borrow(|slot| slot.as_ref().filter(|d| Some(&d.text) != value.as_ref()).map(|d| d.input));
    if let (Some(input), Some(value)) = (input, value) {
        redraw(input, &value);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Watches the stage for the start and end of a composition. Composition events bubble, and the stage outlives every
/// scene that is drawn in it, so these two listeners are attached once.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
fn watch_composition(document: &web_sys::Document) -> Result<(), String> {
    let stage = required_element(document, STAGE_ID)?;
    let started = Closure::<dyn FnMut()>::new(|| COMPOSING.set(true));
    let ended = Closure::<dyn FnMut()>::new(|| {
        COMPOSING.set(false);
        // Not at once: the text box may not hold the composed text until the events that follow have been handled.
        if let Some(window) = web_sys::window() {
            let later = Closure::once_into_js(composition_ended);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
        }
    });
    for (name, listener) in [("compositionstart", &started), ("compositionend", &ended)] {
        stage
            .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach the {name} listener: {e:?}"))?;
    }
    started.forget();
    ended.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the CRC-32 scene in `crc32-window.html`, and fits the window to it. The sender's calculation goes down the
/// page, and the receiver's goes back up.` The diagram is scaled down to 85%, text and all, since it is a large demo.
///
/// The sender's column, from the top:
///
/// 1. The text input node holds the text to checksum. Each change works the checksum out again.
/// 2. The bytes of the text as a hex dump, and under it the table index of each byte: the low byte of the register
///    XORed with the input byte.
/// 3. The register starts as all ones. For each byte, the table's entry at its index is XORed with the register shifted
///    right by a byte, to make the register after that byte. The table's entry is the result of shifting one byte
///    through the register a bit at a time, XORing in the polynomial `0xEDB88320` each time a one falls out. The note
///    shows those eight shifts for the first byte.
/// 4. After the last byte, the register is XORed with all ones to flip its bits. That is the CRC-32.
///
/// The receiver's column, from the bottom, so that the data flows up it. Its top row, the corrected bytes, is level
/// with the sender's input bytes, to compare the two:
///
/// 5. The bytes that arrived. The user flips one bit on the way, chosen at random from the data and the CRC together,
///    with the button to the right of the length note at the top of the diagram.
/// 6. The receiver works out the CRC of what arrived, by the same calculation.
/// 7. The syndrome is the CRC that arrived XORed with the CRC the receiver worked out. Zero means no error.
/// 8. Every single flipped bit of the data gives a syndrome of its own, and it does not depend on what the data holds.
///    The receiver finds the bit whose syndrome matches. A syndrome of a single one is a flipped bit of the CRC itself.
///    A syndrome that no single bit gives means that more bits changed: that is detected, and cannot be corrected.
/// 9. The bit is flipped back, and the corrected bytes have the CRC that was sent.
///
/// # Stepping through it
///
/// A selection toolbar at the top steps through the calculation, as the other demos do. The boxes that a step has not
/// reached are drawn faint, and the one it has just reached is marked. At the start nothing is worked out. Each step
/// takes one byte through the register, then the CRC is made, the bytes arrive, and the receiver's boxes are reached
/// one at a time. See [`walk`] for the position of each step. Editing the text starts the walk again, and flipping a
/// bit moves it on to the arrival, since the error cannot be seen before then.
///
/// # The layout
///
/// The boxes of each column are on one vertical axis, so each is centred over the one it connects to. A box that only
/// helps, such as the initial register and the final mask, is beside the one it feeds and level with it. The notes are
/// on the outside. Nothing is draggable, since the layout is the point.
///
/// Every value is worked out in plain Rust by [`compute`] and [`locate`]. The library does no checksumming. A node
/// holds the same number of values for as long as it lives, so each change draws the scene again, as the Base64 demo
/// does. Text past [`INPUT_LEN`] bytes is not covered by the checksum, and the cut falls between characters.
///
/// A scene is not drawn while the user is composing text with an input method editor, since drawing it replaces the
/// text box and ends the composition. It is drawn when the composition ends.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
pub(crate) fn build_crc32_window() -> Result<(), String> {
    let document = document()?;
    watch_composition(&document)?;
    controls::watch_buttons(&document)?;
    build(INITIAL_TEXT, 0)?;
    fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The CRC-32 scene is the largest of the demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `crc32-window.html`, which runs [`build_crc32_window`] there. A named window
/// is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#crc32-open`, or if a listener could not be attached to it.
pub(crate) fn build_crc32_demo() -> Result<(), String> {
    wire_open_window(&document()?, "crc32-open", WINDOW_PAGE, WINDOW_NAME)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
