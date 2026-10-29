//! The QR code panel, so far the Phase 0 spike of the design in `design-specs/qr-code-generator.pdf`:
//!
//! - `panel-qr-code` / `#qr-code-diagram` — [`build_qr_code_demo`]: a 21 x 21 matrix drawn with a fixed handful of SVG
//!   elements, stepped through a scripted sequence of states.
//!
//! The spike answers one question: can a symbol of 441 modules be shown with a fixed number of elements, where a module
//! may have a colour, a role, both or neither? It does not encode anything. The bits it places are a fixed sample.
//!
//! The buttons, the list of roles and the text are all elements of the scene, so they zoom and pan with the matrix.
//!
//! [`grid`] is the matrix and its rules, [`function`] is the fixed layout of a Version 1 symbol, [`paths`] turns a grid
//! into path data, [`text`] wraps text into lines, and [`viewport`] writes the matrix to the scene. [`slot`] is what
//! they write through, and [`state`] holds the position that all of them are drawn from. Everything but the DOM writes
//! has tests.

mod function;
mod grid;
mod paths;
mod slot;
mod spike;
mod state;
mod text;
mod viewport;

use demo_common::{
    svg_buttons::{self, Button},
    util::{MARGIN, clear_error, document, ensure_svg_in, report_error, stringify},
};
use grid::{BitGrid, CellState, QrModuleKind, SIZE};
use spike::spike_steps;
use state::State;
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::scene::{Scene, Side, ToolbarOptions};
use text::{BlockLayout, TextBlock};
use viewport::{MODULE_PIXELS, Viewport};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the grid, the layout, the path data, the steps and the scene's parts.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("qr_code/grid.rs", "grid.rs"),
    crate::source_file!("qr_code/function.rs", "function.rs"),
    crate::source_file!("qr_code/paths.rs", "paths.rs"),
    crate::source_file!("qr_code/spike.rs", "spike.rs"),
    crate::source_file!("qr_code/slot.rs", "slot.rs"),
    crate::source_file!("qr_code/state.rs", "state.rs"),
    crate::source_file!("qr_code/text.rs", "text.rs"),
    crate::source_file!("qr_code/viewport.rs", "viewport.rs"),
    crate::source_file!("qr_code/mod.rs", "mod.rs"),
];

const STAGE_ID: &str = "qr-code-diagram-stage";
const DIAGRAM_ID: &str = "qr-code-diagram";
const MATRIX_TEXT_ID: &str = "qr-code-matrix-text";
const ANNOUNCEMENT_ID: &str = "qr-code-announcement";
const ERROR_ID: &str = "qr-code-error";

/// The actions of the buttons that move through the steps.
const PREV: &str = "prev";
const NEXT: &str = "next";
const RESTART: &str = "restart";

/// The action of the button that outlines no role. The other role buttons are `role-` and the role's own key.
const NO_ROLE: &str = "role-none";

/// The room east of the scene's content for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

/// The left column holds the buttons, the matrix and what the step did.
const MATRIX_TOP: f64 = 76.0;
const BUTTON_HEIGHT: f64 = 36.0;
const BUTTON_GAP: f64 = 12.0;
const CAPTION_TOP: f64 = MATRIX_TOP + (SIZE * MODULE_PIXELS) as f64 + 28.0;
const LINE_HEIGHT: f64 = 20.0;
const CAPTION_LINES: usize = 4;
const STATS_TOP: f64 = CAPTION_TOP + CAPTION_LINES as f64 * LINE_HEIGHT + 8.0;
const STATS_LINES: usize = 2;
/// Wide enough for the matrix, which is 420 pixels, at the size the text is drawn.
const LEFT_COLUMN_CHARS: usize = 54;

/// The right column holds the list of roles, then the key, then the explanation of the chosen role.
const RIGHT_LEFT: f64 = MARGIN + (SIZE * MODULE_PIXELS) as f64 + 40.0;
const ROLE_BUTTON_WIDTH: f64 = 220.0;
const ROLE_BUTTON_HEIGHT: f64 = 30.0;
const ROLE_BUTTON_GAP: f64 = 6.0;
const KEY_LINES: usize = 3;
const EXPLANATION_LINES: usize = 9;
const RIGHT_COLUMN_WIDTH: f64 = 460.0;
const RIGHT_COLUMN_CHARS: usize = 56;

/// The fill and the outline of a button, chosen or not.
const CHOSEN: (&str, &str) = ("#bcd9f7", "#2b5d9b");
const NOT_CHOSEN: (&str, &str) = ("#f1f1f1", "#767676");

/// Said about modules in general, which no role explains.
const KEY: &str = "A pale blue module has no colour yet. A blue outline marks the modules the step changed. An orange \
                   outline marks the modules of the role you chose.";

/// Said when no role is chosen.
const NO_ROLE_EXPLANATION: &str = "No role is chosen. Choose one from the list to outline its modules in the matrix, \
                                   and to read what it is for.";

thread_local! {
    // Keeps the spike alive for the page's lifetime. The listeners reach it from here.
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    // Keeps the scene alive. Nothing here draws it again, so nothing else holds it.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The action of the button that outlines `role`, or [`NO_ROLE`] for `None`.
fn role_action(role: Option<QrModuleKind>) -> String {
    role.map_or_else(|| NO_ROLE.to_owned(), |kind| format!("role-{}", kind.key()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the walk is, and how much the viewport had to write to get there.
fn stats_text(position: &str, elements: u32, writes: usize) -> String {
    format!("{position}. The matrix is {elements} elements, and this change wrote {writes} of their values.")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's own parts that show the state, which [`State::apply`] writes.
struct Display {
    viewport: Viewport,
    layer: web_sys::Element,
    caption: TextBlock,
    stats: TextBlock,
    role_name: TextBlock,
    role_text: TextBlock,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Changes the spike's state with `change`, then draws it. A failure is shown on the page and leaves the state as
/// `change` made it, so that the next change draws the whole of it again.
fn update(change: impl FnOnce(&mut State)) {
    let result = STATE.with_borrow_mut(|slot| match slot.as_mut() {
        Some(state) => {
            change(state);
            state.apply()
        },
        None => Ok(()),
    });
    match result {
        Ok(()) => clear_error(ERROR_ID),
        Err(message) => report_error(ERROR_ID, &message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Does what the button `action` says. A button that is disabled has nothing to do, so it changes nothing.
fn press(action: &str) {
    match action {
        PREV => update(|state| state.at = spike::prev(state.at)),
        NEXT => update(|state| state.at = spike::next(state.at, state.steps.len())),
        RESTART => update(|state| state.at = None),
        NO_ROLE => update(|state| state.role = None),
        other => {
            if let Some(role) = other.strip_prefix("role-").and_then(QrModuleKind::from_key) {
                update(|state| state.role = Some(role));
            }
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Phase 0 spike of the QR code demo: a 21 x 21 matrix, drawn with six SVG elements however many of its
/// modules have a colour, and stepped through five scripted states by Prev, Next and Restart.
///
/// Everything is in the scene: the three step buttons, the list of roles, the matrix and the text. So they all zoom and
/// pan together, and a button works from the keyboard. The scene is built once and never drawn again. A change writes
/// only the values that differ.
///
/// A module has a colour and a role, and either can come first. The format modules are reserved with the function
/// patterns, but only coloured in the last step. The data modules have neither until their codeword is placed. A module
/// without a colour is drawn pale blue, so it cannot be taken for a light one. Choosing a role outlines its modules and
/// explains what the role is for. The highlight outlines the modules a step changed. Both are paths of their own, so
/// changing them leaves the three base paths alone.
///
/// The bits placed are a fixed sample, and nothing is encoded. The matrix is plain SVG in the scene's layer, because
/// the spike must not need a new public API.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
pub(crate) fn build_qr_code_demo() -> Result<(), String> {
    let document = document()?;
    let steps = spike_steps().map_err(|e| e.to_string())?;

    let height = (CAPTION_TOP + (CAPTION_LINES + STATS_LINES) as f64 * LINE_HEIGHT + 2.0 * MARGIN).max(660.0);
    let size = Size::new(RIGHT_LEFT + RIGHT_COLUMN_WIDTH + MARGIN + TOOLBAR_ROOM, height);
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, size)?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    let layer = svg_buttons::layer(&document, DIAGRAM_ID)?;

    // The buttons that move through the steps are along the top of the left column.
    let mut left = MARGIN;
    for (label, action, width) in [("Prev", PREV, 100.0), ("Next", NEXT, 100.0), ("Restart", RESTART, 110.0)] {
        add_button(
            &document,
            &layer,
            Point::new(left, MARGIN),
            Size::new(width, BUTTON_HEIGHT),
            label,
            action,
            None,
        )?;
        left += width + BUTTON_GAP;
    }

    // The list of roles is the top of the right column. Each is a toggle button, and only one is pressed.
    let mut top = MARGIN;
    for role in std::iter::once(None).chain(QrModuleKind::ALL.map(Some)) {
        let label = role.map_or("No role", QrModuleKind::label);
        add_button(
            &document,
            &layer,
            Point::new(RIGHT_LEFT, top),
            Size::new(ROLE_BUTTON_WIDTH, ROLE_BUTTON_HEIGHT),
            label,
            &role_action(role),
            Some(false),
        )?;
        top += ROLE_BUTTON_HEIGHT + ROLE_BUTTON_GAP;
    }

    let block = |x: f64, y: f64, max_lines: usize, width: usize, bold: bool| BlockLayout {
        x,
        y,
        line_height: LINE_HEIGHT,
        max_lines,
        width,
        bold,
    };
    let viewport = Viewport::new(&document, &layer, (MARGIN, MATRIX_TOP), MATRIX_TEXT_ID)?;
    let caption = TextBlock::new(
        &document,
        &layer,
        block(MARGIN, CAPTION_TOP, CAPTION_LINES, LEFT_COLUMN_CHARS, false),
    )?;
    let stats = TextBlock::new(
        &document,
        &layer,
        block(MARGIN, STATS_TOP, STATS_LINES, LEFT_COLUMN_CHARS, false),
    )?;
    let key = TextBlock::new(
        &document,
        &layer,
        block(RIGHT_LEFT, top + 16.0, KEY_LINES, RIGHT_COLUMN_CHARS, false),
    )?;
    key.set(KEY)?;
    let name_top = top + 16.0 + KEY_LINES as f64 * LINE_HEIGHT + 20.0;
    let role_name = TextBlock::new(&document, &layer, block(RIGHT_LEFT, name_top, 1, RIGHT_COLUMN_CHARS, true))?;
    let role_text = TextBlock::new(
        &document,
        &layer,
        block(
            RIGHT_LEFT,
            name_top + LINE_HEIGHT + 4.0,
            EXPLANATION_LINES,
            RIGHT_COLUMN_CHARS,
            false,
        ),
    )?;

    let state = State {
        steps,
        at: None,
        role: None,
        empty: BitGrid::new(SIZE, SIZE),
        display: Display {
            viewport,
            layer,
            caption,
            stats,
            role_name,
            role_text,
        },
    };
    state.apply()?;
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    svg_buttons::watch(&document, STAGE_ID, press)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
