//! The QR code panel: how a Version 1-L symbol is made from up to 17 ASCII characters, one step at a time.
//!
//! - `panel-qr-code` / `#qr-code-diagram` — [`build_qr_code_demo`]: the description of the demo, and a button that
//!   opens the scene in a window of its own, which runs [`build_qr_code_window`].
//!
//! The scene is a graph of the stages of the encoding, with a 21 by 21 matrix beside it. Stepping through the 62
//! snapshots changes what the nodes hold and which of their cells are dimmed, and draws the matrix as it is at that
//! position. The scene is built once and never drawn again.
//!
//! [`qr_core`] makes the symbol and has no DOM. [`snapshots`] works out the whole state of the scene at each position
//! from it, also with no DOM. [`graph`] draws the nodes and edges, [`viewport`] the matrix, and [`text`] the text.
//! [`state`] holds the position and draws all of them from it, and [`slot`] is what they write through. [`grid`],
//! [`function`] and [`paths`] are the matrix and its layout.

mod export;
mod function;
mod graph;
mod grid;
mod paths;
// Only some of the encoder is drawn, and the rest is used by its own tests.
#[allow(
    dead_code,
    reason = "the encoder has more than the scene draws, and its tests use all of it"
)]
mod qr_core;
mod retry;
mod slot;
mod snapshots;
mod state;
mod text;
mod viewport;

use demo_common::{
    svg_buttons::{self, Button},
    util::{
        CONTENT_TOP, MARGIN, add_step_driver, announce, clear_error, document, ensure_svg_in, fit_window_to_stage,
        report_error, resize_svg, step_toolbar_options, stringify, wire_open_window,
    },
};
use graph::Graph;
use grid::{BitGrid, CellState, QrModuleKind, SIZE};
use qr_core::{encode_text, input::MAX_BYTES, symbol::build_symbol};
use retry::Retry;
use snapshots::{COUNT, JUMPS, NodeKey, SnapshotSet};
use state::State;
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, SceneTitleOptions, Selection, Side, ToolbarOptions},
};
use text::{BlockLayout, TextBlock};
use viewport::{MODULE_PIXELS, Viewport};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the snapshots, the scene's parts and the matrix with its layout. The
/// encoder in `qr_core` is shown by its own tests, which are longer than any of these.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("qr_code/snapshots.rs", "snapshots.rs"),
    crate::source_file!("qr_code/graph.rs", "graph.rs"),
    crate::source_file!("qr_code/state.rs", "state.rs"),
    crate::source_file!("qr_code/retry.rs", "retry.rs"),
    crate::source_file!("qr_code/export/mod.rs", "export/mod.rs"),
    crate::source_file!("qr_code/export/download.rs", "export/download.rs"),
    crate::source_file!("qr_code/viewport.rs", "viewport.rs"),
    crate::source_file!("qr_code/text.rs", "text.rs"),
    crate::source_file!("qr_code/slot.rs", "slot.rs"),
    crate::source_file!("qr_code/paths.rs", "paths.rs"),
    crate::source_file!("qr_code/function.rs", "function.rs"),
    crate::source_file!("qr_code/grid/mod.rs", "grid/mod.rs"),
    crate::source_file!("qr_code/grid/bit_grid.rs", "grid/bit_grid.rs"),
    crate::source_file!("qr_code/grid/qr_module_kind.rs", "grid/qr_module_kind.rs"),
    crate::source_file!("qr_code/grid/grid_error.rs", "grid/grid_error.rs"),
    crate::source_file!("qr_code/mod.rs", "mod.rs"),
];

/// The window page the gallery panel opens, and the name of that window.
const WINDOW_PAGE: &str = "qr-code-window.html";
const WINDOW_NAME: &str = "qr-code-window";

const STAGE_ID: &str = "qr-code-diagram-stage";
const DIAGRAM_ID: &str = "qr-code-diagram";
const MATRIX_TEXT_ID: &str = "qr-code-matrix-text";
const ANNOUNCEMENT_ID: &str = "qr-code-announcement";
const ERROR_ID: &str = "qr-code-error";

/// The text the scene opens with.
const INITIAL_TEXT: &str = "This is a QR code";

/// The action of the button that outlines no role. The other role buttons are `role-` and the role's own key. A button
/// that jumps to a snapshot has the action `jump-` and the number of the snapshot.
const NO_ROLE: &str = "role-none";
const JUMP_ACTION: &str = "jump-";

/// The action of the button that asks the browser to save the finished symbol as an SVG file.
const EXPORT_ACTION: &str = "export";

/// The room east of the scene's content for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

/// The space between the graph and the column of the matrix.
const COLUMN_GAP: f64 = 60.0;

/// The note that says what the marked part of the bitstream is stands to the right of the bitstream, which is narrower
/// than the rest of the graph. This is the gap between them, how wide a note of the longest line is on the page, and
/// how wide and tall the note may be.
const NOTE_GAP: f64 = 40.0;
const NOTE_PIXELS: f64 = 220.0;
const NOTE_CHARS: usize = 30;
const NOTE_TITLE_LINES: usize = 2;
const NOTE_TEXT_LINES: usize = 6;

/// The right column holds the buttons that jump to a stage, the matrix and what the step did, what the stage is for,
/// and the list of roles with what the chosen role is for.
const COLUMN_WIDTH: f64 = 460.0;
const COLUMN_CHARS: usize = 54;
const LINE_HEIGHT: f64 = 20.0;
const JUMP_WIDTH: f64 = 140.0;
const JUMP_HEIGHT: f64 = 32.0;
const JUMP_GAP: f64 = 10.0;
const JUMPS_PER_ROW: usize = 3;
const ROLE_BUTTON_WIDTH: f64 = 220.0;
const ROLE_BUTTON_HEIGHT: f64 = 30.0;
const ROLE_BUTTON_GAP: f64 = 6.0;
const CAPTION_LINES: usize = 4;
const STATS_LINES: usize = 2;
const STAGE_LINES: usize = 7;
const KEY_LINES: usize = 3;
const EXPLANATION_LINES: usize = 9;

/// The fill and the outline of a button, chosen or not.
const CHOSEN: (&str, &str) = ("#bcd9f7", "#2b5d9b");
const NOT_CHOSEN: (&str, &str) = ("#f1f1f1", "#767676");

/// Said about modules in general, which no role explains.
const KEY: &str = "A pale blue module has no colour yet. A blue outline marks the modules the step changed. An orange \
                   outline marks the modules of the role you chose.";

/// Said when no role is chosen.
const NO_ROLE_EXPLANATION: &str = "No role is chosen. Choose one from the list to outline its modules in the matrix, \
                                   and to read what it is for.";

/// Said before the walk starts.
const UNSTARTED_EXPLANATION: &str = "Press Next, or choose a stage, to begin. First you may type other text, of up to \
                                     17 ASCII characters, in the box. The matrix is empty until the symbol is built.";

thread_local! {
    // Keeps the position alive for the page's lifetime. The listeners reach it from here.
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    // Keeps the scene alive. It is built once, and the listeners get a handle to it from here.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The action of the button that outlines `role`, or [`NO_ROLE`] for `None`.
fn role_action(role: Option<QrModuleKind>) -> String {
    role.map_or_else(|| NO_ROLE.to_owned(), |kind| format!("role-{}", kind.key()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the step did, and how many modules there are of each colour afterwards.
fn caption_text(label: &str, grid: &BitGrid) -> String {
    format!(
        "{label} Of {} modules, {} are dark, {} are light and {} have no colour.",
        SIZE * SIZE,
        grid.count_state(CellState::Dark),
        grid.count_state(CellState::Light),
        grid.count_state(CellState::Unassigned),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the walk is, and how much the matrix had to write to get there.
fn stats_text(position: &str, elements: u32, writes: usize) -> String {
    format!("{position}. The matrix is {elements} elements, and this change wrote {writes} of their values.")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every snapshot of `text`.
///
/// # Errors
///
/// Returns `Err` with the reason if the text is not ASCII or is more than 17 bytes, or if the symbol cannot be made.
fn make_set(text: &str) -> Result<SnapshotSet, String> {
    let encoded = encode_text(text).map_err(|e| e.to_string())?;
    let symbol = build_symbol(&encoded).map_err(|e| e.to_string())?;
    let set = snapshots::build(text, &encoded, &symbol).map_err(|e| e.to_string())?;
    if set.snapshots.len() == COUNT {
        Ok(set)
    } else {
        Err(format!("{} snapshots were made, not {COUNT}", set.snapshots.len()))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene, which the listeners cannot be given directly.
fn scene() -> Option<Scene> {
    SCENE.with_borrow(Clone::clone)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Changes the position with `change`, then draws it. A failure is shown on the page and leaves the position as
/// `change` made it, so that the next change draws the whole of it again.
fn update(scene: &Scene, change: impl FnOnce(&mut State) -> Result<(), String>) {
    let result = STATE.with_borrow_mut(|slot| match slot.as_mut() {
        Some(state) => change(state).and_then(|()| state.apply(scene)),
        None => Ok(()),
    });
    match result {
        Ok(()) => clear_error(ERROR_ID),
        Err(message) => report_error(ERROR_ID, &message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stops the browser taking more than 17 characters into the text node, whether they are typed or pasted.
///
/// The library has no limit of its own on a text node, so this is set on the `<input>` the node holds. Every character
/// that can be encoded is one byte, so 17 characters is the 17 bytes that a symbol holds. Text set by a script is not
/// limited by the browser, so [`take_text`] still refuses what is too long, and says why.
///
/// # Errors
///
/// Returns `Err` if the text node's `<input>` cannot be found or set.
fn limit_text_input(document: &web_sys::Document) -> Result<(), String> {
    document
        .query_selector(&format!("#{DIAGRAM_ID} input"))
        .map_err(|e| format!("could not look for the text input: {e:?}"))?
        .ok_or_else(|| "the text node has no input".to_owned())?
        .set_attribute("maxlength", &MAX_BYTES.to_string())
        .map_err(|e| format!("could not limit the text input: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the caret goes when the text node has had `rejected` put back to `accepted`.
///
/// The browser has put the caret after whatever was entered. The characters that were refused are no longer there, so
/// the caret moves back by as many as `rejected` has more than `accepted`, and not past the start. Both ends of a
/// selection move the same. The counts are UTF-16 code units, which is how the browser counts.
fn caret_after_refusal(caret: (u32, u32), rejected: &str, accepted: &str) -> (u32, u32) {
    let extra = rejected.encode_utf16().count().saturating_sub(accepted.encode_utf16().count());
    let extra = u32::try_from(extra).unwrap_or(u32::MAX);
    (caret.0.saturating_sub(extra), caret.1.saturating_sub(extra))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Refuses `rejected`, which the text node `node` now holds: the reason `message` is shown, and the text node is put
/// back to the last text that was accepted, with the caret where it was, so that it always agrees with the symbol on
/// show.
///
/// A character that cannot be encoded is therefore not taken, as a character past the limit of 17 is not. The reason
/// stays until the next change that works.
fn refuse_text(scene: &Scene, node: NodeId, rejected: &str, message: &str) {
    report_error(ERROR_ID, message);
    let Some(accepted) = STATE.with_borrow(|slot| slot.as_ref().map(|state| state.set.content.text.clone())) else {
        return;
    };
    let restore = || -> Result<(), String> {
        let caret = scene.text_input_caret(node).map_err(stringify)?;
        scene.set_text_input_value(node, &accepted).map_err(stringify)?;
        scene
            .focus_text_input(node, Some(caret_after_refusal(caret, rejected, &accepted)))
            .map_err(stringify)
    };
    if let Err(problem) = restore() {
        report_error(ERROR_ID, &format!("{message} The text box could not be put back: {problem}"));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Takes in new text from the text node `node`.
///
/// The new snapshots are made first, and if they cannot be, the text is refused: the reason is shown, the symbol on
/// show stays, and the text node goes back to the text that made it, see [`refuse_text`]. Nothing has changed at that
/// point.
///
/// Otherwise the new snapshots are committed, which cannot fail, and the walk is put back to the start. The nodes and
/// the anchor's selection are then brought into line with them by [`State::apply`], and the start is drawn. Nothing is
/// built again.
///
/// Bringing the nodes into line writes to several of them, and the browser can refuse any of those writes. They cannot
/// be undone, so a failure can leave some nodes holding the new text while others hold the old, for a moment. The
/// snapshots, and so the matrix, the captions and the toolbar, are always whole and agree with the text. The error is
/// shown, and the work is marked as not done. Every later draw does all of it again before anything else, which is
/// safe, since writing what a node already holds changes nothing. So the next step, jump, role or text puts the nodes
/// right.
fn take_text(scene: &Scene, node: NodeId, text: &str) {
    let set = match make_set(text) {
        Ok(set) => set,
        Err(message) => {
            refuse_text(scene, node, text, &message);
            return;
        },
    };
    update(scene, |state| {
        state.set = set;
        state.at = None;
        state.stale.require();
        Ok(())
    });
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Asks the browser to save the finished symbol as an SVG file, if the walk is at its last step, and does nothing
/// otherwise, since the button is dimmed but still gets the click.
///
/// The symbol is the one the last snapshot shows, with the quiet zone added. Nothing here changes the position or what
/// is drawn, so a failure is reported on its own and leaves the symbol on show as it was.
///
/// A click on a link only asks for a download. The browser may block it, cancel it, or wait for the person to allow it,
/// and the page is told none of that. So what is announced is that the download was requested, never that a file was
/// saved.
fn export_symbol() {
    let finished = STATE.with_borrow(|slot| {
        slot.as_ref()
            .filter(|state| state.at == Some(COUNT - 1))
            .map(|state| (state.set.snapshots[COUNT - 1].grid.clone(), state.set.content.text.clone()))
    });
    let Some((grid, text)) = finished else { return };
    let requested = export::svg(&grid, &text)
        .map_err(|problem| problem.to_string())
        .and_then(|file| export::download(&document()?, export::FILE_NAME, &file));
    match requested {
        Ok(()) => announce(
            ANNOUNCEMENT_ID,
            &format!(
                "Download of {} requested: {} by {} modules, with a quiet zone of {}.",
                export::FILE_NAME,
                export::MODULES,
                export::MODULES,
                export::QUIET_ZONE
            ),
        ),
        Err(problem) => report_error(
            ERROR_ID,
            &format!("The export failed, and the symbol on show is as it was. {problem}"),
        ),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Does what the button `action` says. A role button outlines or stops outlining a role. A jump button goes to a
/// snapshot by setting the anchor's selection, which does not call the toolbar's own callback, so the position is drawn
/// here. The export button saves the symbol, see [`export_symbol`].
fn press(action: &str) {
    if action == EXPORT_ACTION {
        export_symbol();
        return;
    }
    let Some(scene) = scene() else { return };
    if let Some(index) = action.strip_prefix(JUMP_ACTION).and_then(|n| n.parse::<usize>().ok()) {
        if index < COUNT {
            update(&scene, |state| {
                scene
                    .set_selection(state.display.step, Selection::Cell(index))
                    .map_err(stringify)?;
                state.at = Some(index);
                Ok(())
            });
        }
        return;
    }
    let role = if action == NO_ROLE {
        None
    } else {
        action.strip_prefix("role-").and_then(QrModuleKind::from_key)
    };
    if action == NO_ROLE || role.is_some() {
        update(&scene, |state| {
            state.role = role;
            Ok(())
        });
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a button of `size` with its top left corner at `origin`.
fn add_button(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    origin: Point,
    size: Size,
    label: &str,
    action: &str,
    pressed: Option<bool>,
) -> Result<(), String> {
    svg_buttons::add_button(
        document,
        layer,
        Rect { origin, size },
        &Button {
            label,
            action,
            colours: NOT_CHOSEN,
            pressed,
        },
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the QR code scene in `qr-code-window.html`, the window the gallery's own QR code panel opens. Call it once,
/// from that page's own script, after the wasm module has loaded.
///
/// The scene is a graph of the stages that make a symbol, from the text to the scores of the masks, with a matrix of 21
/// by 21 modules beside it. Each node holds as many cells for one text as for another, so it is built once. Typing in
/// the text node only gives the nodes new values, and goes back to the start. The toolbar steps through 62 snapshots,
/// and six buttons jump to the first of a stage. The role buttons outline the modules of one role and explain it.
///
/// Every position is drawn from a snapshot that was made whole, so going back shows exactly what going forward did. The
/// library draws the nodes and edges, and the matrix is plain SVG in the scene's layer, so that nothing new is asked of
/// its public interface.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the window, or if any library call fails.
pub(crate) fn build_qr_code_window() -> Result<(), String> {
    let document = document()?;
    let set = make_set(INITIAL_TEXT)?;

    // The size is only known once everything is placed, so the canvas starts large and is cut down at the end.
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(2400.0, 1800.0))?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    scene
        .show_scene_title("QR code generator", SceneTitleOptions::default())
        .map_err(stringify)?;

    let graph = Graph::build(&scene, &set.content, Point::new(MARGIN, CONTENT_TOP), |scene, node, text| {
        take_text(scene, node, text)
    })?;
    limit_text_input(&document)?;
    // The step driver is the node the toolbar steps along. It is off the canvas and hidden from assistive technology.
    let step = add_step_driver(&scene, DIAGRAM_ID, COUNT)?;
    scene
        .show_selection_toolbar(step, step_toolbar_options(), |scene, _node, transition| {
            update(scene, |state| {
                state.at = transition.to;
                Ok(())
            });
        })
        .map_err(stringify)?;

    let layer = svg_buttons::layer(&document, DIAGRAM_ID)?;
    // The note is to the right of the bitstream. The right column starts after whichever reaches further east, so that
    // the note cannot run into it, though in practice the graph does, since the bitstream is the narrower.
    let bitstream = graph.rect(&scene, NodeKey::Bitstream)?;
    let note_x = bitstream.origin.x + bitstream.size.width + NOTE_GAP;
    let left = graph.right.max(note_x + NOTE_PIXELS) + COLUMN_GAP;

    // The buttons that jump to a stage are along the top of the right column, in rows.
    let mut top = CONTENT_TOP;
    for (position, &(label, index)) in JUMPS.iter().enumerate() {
        let (row, column) = (position / JUMPS_PER_ROW, position % JUMPS_PER_ROW);
        add_button(
            &document,
            &layer,
            Point::new(
                left + column as f64 * (JUMP_WIDTH + JUMP_GAP),
                top + row as f64 * (JUMP_HEIGHT + JUMP_GAP),
            ),
            Size::new(JUMP_WIDTH, JUMP_HEIGHT),
            label,
            &format!("{JUMP_ACTION}{index}"),
            Some(false),
        )?;
    }
    top += JUMPS.len().div_ceil(JUMPS_PER_ROW) as f64 * (JUMP_HEIGHT + JUMP_GAP);

    // The button that asks for the finished symbol to be saved is under them. It is not a toggle, and it is dimmed
    // until the last step.
    add_button(
        &document,
        &layer,
        Point::new(left, top),
        Size::new(JUMP_WIDTH, JUMP_HEIGHT),
        "Export SVG",
        EXPORT_ACTION,
        None,
    )?;
    top += JUMP_HEIGHT + JUMP_GAP;

    let block = |x: f64, y: f64, max_lines: usize, bold: bool| BlockLayout {
        x,
        y,
        line_height: LINE_HEIGHT,
        max_lines,
        width: COLUMN_CHARS,
        bold,
    };
    let text_block =
        |x: f64, y: f64, max_lines: usize, bold: bool| TextBlock::new(&document, &layer, block(x, y, max_lines, bold));

    // The matrix, under its heading, then what the step did and the counts.
    text_block(left, top + 20.0, 1, true)?.set("QR matrix, 21 by 21 modules")?;
    top += 30.0;
    let viewport = Viewport::new(&document, &layer, (left, top), MATRIX_TEXT_ID)?;
    top += (SIZE * MODULE_PIXELS) as f64 + 24.0;
    let caption = text_block(left, top, CAPTION_LINES, false)?;
    top += CAPTION_LINES as f64 * LINE_HEIGHT + 4.0;
    let stats = text_block(left, top, STATS_LINES, false)?;
    top += STATS_LINES as f64 * LINE_HEIGHT + 16.0;

    // What the stage is for.
    let stage_name = text_block(left, top, 1, true)?;
    top += LINE_HEIGHT + 4.0;
    let stage_text = text_block(left, top, STAGE_LINES, false)?;
    top += STAGE_LINES as f64 * LINE_HEIGHT + 16.0;

    // The list of roles is two columns of toggle buttons. Only one is pressed.
    let roles: Vec<Option<QrModuleKind>> = std::iter::once(None).chain(QrModuleKind::ALL.map(Some)).collect();
    for (position, role) in roles.iter().enumerate() {
        let (row, column) = (position / 2, position % 2);
        add_button(
            &document,
            &layer,
            Point::new(
                left + column as f64 * (ROLE_BUTTON_WIDTH + JUMP_GAP),
                top + row as f64 * (ROLE_BUTTON_HEIGHT + ROLE_BUTTON_GAP),
            ),
            Size::new(ROLE_BUTTON_WIDTH, ROLE_BUTTON_HEIGHT),
            role.map_or("No role", QrModuleKind::label),
            &role_action(*role),
            Some(false),
        )?;
    }
    top += roles.len().div_ceil(2) as f64 * (ROLE_BUTTON_HEIGHT + ROLE_BUTTON_GAP) + 16.0;
    text_block(left, top, KEY_LINES, false)?.set(KEY)?;
    top += KEY_LINES as f64 * LINE_HEIGHT + 20.0;
    let role_name = text_block(left, top, 1, true)?;
    top += LINE_HEIGHT + 4.0;
    let role_text = text_block(left, top, EXPLANATION_LINES, false)?;
    top += EXPLANATION_LINES as f64 * LINE_HEIGHT;

    let size = Size::new(left + COLUMN_WIDTH + MARGIN + TOOLBAR_ROOM, top.max(graph.bottom) + MARGIN);
    resize_svg(&document, DIAGRAM_ID, size)?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // The note to the right of the bitstream says what the part that is marked in it holds, a part at a time.
    let note_name = TextBlock::new(
        &document,
        &layer,
        BlockLayout {
            x: note_x,
            y: bitstream.origin.y + LINE_HEIGHT,
            line_height: LINE_HEIGHT,
            max_lines: NOTE_TITLE_LINES,
            width: NOTE_CHARS,
            bold: true,
        },
    )?;
    let note_text = TextBlock::new(
        &document,
        &layer,
        BlockLayout {
            x: note_x,
            y: bitstream.origin.y + LINE_HEIGHT * (NOTE_TITLE_LINES as f64 + 1.0),
            line_height: LINE_HEIGHT,
            max_lines: NOTE_TEXT_LINES,
            width: NOTE_CHARS,
            bold: false,
        },
    )?;

    let state = State {
        set,
        at: None,
        role: None,
        stale: Retry::default(),
        display: Display {
            viewport,
            layer,
            graph,
            step,
            caption,
            stats,
            stage_name,
            stage_text,
            role_name,
            role_text,
            note_name,
            note_text,
        },
    };
    state.apply(&scene)?;
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    svg_buttons::watch(&document, STAGE_ID, press)?;
    fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's own parts that show the state, which [`State::apply`] writes.
struct Display {
    viewport: Viewport,
    layer: web_sys::Element,
    graph: Graph,
    /// The node the toolbar steps along.
    step: NodeId,
    caption: TextBlock,
    stats: TextBlock,
    stage_name: TextBlock,
    stage_text: TextBlock,
    role_name: TextBlock,
    role_text: TextBlock,
    /// The heading and the text of the note to the right of the bitstream.
    note_name: TextBlock,
    note_text: TextBlock,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The QR code scene is large. So the gallery panel keeps only the description, plus a button that opens the scene in a
/// window of its own: `qr-code-window.html`, which runs [`build_qr_code_window`] there. A named window is reused.
/// Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#qr-code-open`, or if a listener could not be attached to it.
pub(crate) fn build_qr_code_demo() -> Result<(), String> {
    wire_open_window(&document()?, "qr-code-open", WINDOW_PAGE, WINDOW_NAME)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod unit_tests;
