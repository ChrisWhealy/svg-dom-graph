//! The page: the board that is drawn, the view that was drawn last, and the square that the keys act on.

use super::{
    board::{BoardParts, apply, arrange, build_board},
    chrome::Chrome,
    controller::Surface,
    controls::Controls,
    dom::{add, focus, make, set},
    layout::{BOARD_LEFT, TOP},
    projection::{Boundaries, View, board_text, cell_origin, clamp_cell, project, seek_text, status},
    solver::Trace,
};
use std::cell::{Cell, RefCell};

/// Reads one of the flags of [`Boundaries`].
type Flag = fn(&Boundaries) -> bool;

/// The buttons that can be at an end, with what each is at an end of.
const ENDS: [(&str, Flag); 6] = [
    ("restart", |b| b.restart),
    ("previous", |b| b.previous),
    ("next", |b| b.next),
    ("play", |b| b.play),
    ("previous-solution", |b| b.previous_solution),
    ("next-solution", |b| b.next_solution),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page: the board that is drawn, the view that was drawn last, and the square that the keys act on. It is what
/// the controller draws on.
pub(super) struct Page {
    /// The group that the board is put in, and replaced in. The controls are beside it in the scene and stay.
    host: web_sys::Element,
    document: web_sys::Document,
    controls: Controls,
    board: RefCell<BoardParts>,
    chrome: Chrome,
    /// The view that is on the page, or `None` when a write failed and what is there is not known.
    committed: RefCell<Option<View>>,
    /// The square that has `tabindex=0`, which the focus ring is on while the grid has the focus.
    active: Cell<(usize, usize)>,
    ring_shown: Cell<bool>,
    playing: Cell<bool>,
    speed: Cell<u32>,
    error_shown: Cell<bool>,
    said: Cell<u32>,
}

impl Page {
    /// Draws the controls and the layers of the first board in `svg`, and finds the elements that the panel declares.
    /// The page is ready to be drawn on, and has drawn no snapshot yet.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the panel is missing an element, or the board could not be made.
    pub(super) fn new(
        document: &web_sys::Document,
        svg: &web_sys::Element,
        trace: &Trace,
        speed: u32,
    ) -> Result<Self, String> {
        let controls = Controls::build(document, svg, trace.size(), speed, trace.len())?;
        let host = make(
            document,
            "g",
            &[("class", "nq-host"), ("transform", &format!("translate({BOARD_LEFT} {TOP})"))],
        )?;
        add(svg, &host)?;
        let parts = build_board(document, trace.size())?;
        add(&host, &parts.root)?;
        let chrome = Chrome::find(document)?;
        chrome.info.set_text_content(Some(&board_text(
            trace.size(),
            trace.len(),
            trace.solution_count(),
            trace.allocated_bytes(),
        )));
        let page = Self {
            host,
            document: document.clone(),
            controls,
            board: RefCell::new(parts),
            chrome,
            committed: RefCell::new(None),
            active: Cell::new((0, 0)),
            ring_shown: Cell::new(false),
            playing: Cell::new(false),
            speed: Cell::new(speed),
            error_shown: Cell::new(false),
            said: Cell::new(0),
        };
        page.set_active((0, 0));
        page.show_playback();
        Ok(page)
    }

    /// The controls that are drawn in the scene.
    pub(super) fn controls(&self) -> &Controls {
        &self.controls
    }

    /// The number of rows and of columns of the board that is drawn.
    pub(super) fn size(&self) -> usize {
        self.board.borrow().size
    }

    /// The square that the keys act on.
    pub(super) fn active(&self) -> (usize, usize) {
        self.active.get()
    }

    /// Shows how many events are shown in each second while playing.
    pub(super) fn set_speed(&self, per_second: u32) {
        self.speed.set(per_second);
        self.show_playback();
    }

    /// Writes the fields of the status for `view`. These are text, and do not fail.
    pub(super) fn write_status(&self, view: &View) {
        let text = status(view);
        let values = [
            &text.position, &text.kind, &text.cell, &text.depth, &text.reason, &text.solutions, &text.vector,
        ];
        for (field, value) in self.chrome.fields.iter().zip(values) {
            field.set_text_content(Some(value));
        }
    }

    /// Writes `aria-disabled` on the buttons that are at an end, for the ones that differ from `previous`, and puts
    /// the range at the step of `view`. The end of the range is written last, and only if it differs, so that a write
    /// that fails never leaves the range going further than the trace that the page has.
    pub(super) fn write_controls(&self, previous: Option<&View>, view: &View) -> Result<(), String> {
        let now = view.boundaries();
        let before = previous.map(View::boundaries);
        for (action, flag) in ENDS {
            if before.is_none_or(|b| flag(&b) != flag(&now)) {
                self.controls.dim(action, flag(&now))?;
            }
        }
        self.controls.set_seek(view.index, &seek_text(view))?;
        if previous.is_none_or(|p| p.last != view.last) {
            self.controls.set_last(view.last)?;
        }
        Ok(())
    }

    /// Writes what the playback is doing in its field of the status.
    pub(super) fn show_playback(&self) {
        let text = format!(
            "{} at {} events per second",
            if self.playing.get() { "Playing" } else { "Paused" },
            self.speed.get()
        );
        self.chrome.fields[7].set_text_content(Some(&text));
    }

    pub(super) fn hit(&self, (row, col): (usize, usize)) -> Option<web_sys::Element> {
        let board = self.board.borrow();
        (row < board.size && col < board.size).then(|| board.hits[board.at(row, col)].clone())
    }

    /// Puts the focus ring on the active square, and shows it or hides it.
    pub(super) fn place_ring(&self) {
        let board = self.board.borrow();
        let (row, col) = self.active.get();
        let (x, y) = cell_origin(board.size, row, col);
        let _ = set(&board.ring, "transform", &format!("translate({x} {y})"));
        let _ = set(
            &board.ring,
            "class",
            &format!("nq-ring{}", if self.ring_shown.get() { "" } else { " nq-off" }),
        );
    }

    /// Makes `cell` the square that has `tabindex=0`, and the only one.
    pub(super) fn set_active(&self, cell: (usize, usize)) {
        let old = self.active.replace(cell);
        if old != cell {
            if let Some(hit) = self.hit(old) {
                let _ = set(&hit, "tabindex", "-1");
            }
        }
        if let Some(hit) = self.hit(cell) {
            let _ = set(&hit, "tabindex", "0");
        }
        self.place_ring();
    }

    /// Moves the active square to `cell`, and the focus with it.
    pub(super) fn move_to(&self, cell: (usize, usize)) {
        self.set_active(cell);
        if let Some(hit) = self.hit(cell) {
            focus(&hit);
        }
    }

    /// Shows or hides the ring as the grid gains and loses the focus.
    pub(super) fn show_ring(&self, shown: bool) {
        self.ring_shown.set(shown);
        self.place_ring();
    }

    pub(super) fn hide_error(&self) {
        if self.error_shown.replace(false) {
            let _ = self.chrome.error.set_attribute("hidden", "");
        }
    }

    pub(super) fn grid_has_focus(&self) -> bool {
        self.document
            .active_element()
            .and_then(|active| active.closest("[role=gridcell]").ok().flatten())
            .is_some()
    }
}

impl Surface for Page {
    fn draw(&self, trace: &Trace, index: usize, full: bool) -> Result<(), String> {
        let view = project(trace, index).ok_or_else(|| format!("there is no snapshot {index}"))?;
        // Taken, so that a write that fails leaves nothing that claims to be on the page.
        let before = self.committed.borrow_mut().take();
        let before = before.filter(|b| !full && b.size == view.size);
        apply(&self.board.borrow(), before.as_ref(), &view)?;
        self.write_controls(before.as_ref(), &view)?;
        self.write_status(&view);
        *self.committed.borrow_mut() = Some(view);
        self.hide_error();
        Ok(())
    }

    fn rebuild(&self, trace: &Trace) -> Result<(), String> {
        let size = trace.size();
        let view = project(trace, 0).ok_or_else(|| "the new board has no first snapshot".to_owned())?;
        // Everything that can fail is done before the old board goes. First the new board, apart from the page.
        let parts = build_board(&self.document, size)?;
        apply(&parts, None, &view)?;
        let had_focus = self.grid_has_focus();
        let active = clamp_cell(self.active.get(), size);
        let ring_shown = self.ring_shown.get() || had_focus;
        arrange(&parts, active, ring_shown)?;
        // Then the controls, which are in the page already, so they are put back if one fails. The end of the range is
        // the last thing written, so a failure never leaves it going further than the old trace.
        let old = self.committed.borrow().clone();
        if let Err(problem) = self.write_controls(None, &view) {
            let restored = old.as_ref().is_some_and(|old| self.write_controls(None, old).is_ok());
            if !restored {
                // What the controls show is not known, so the next draw writes all of it.
                *self.committed.borrow_mut() = None;
            }
            return Err(problem);
        }
        // From here nothing can fail.
        self.host.replace_children_with_node_1(&parts.root);
        *self.board.borrow_mut() = parts;
        self.active.set(active);
        self.ring_shown.set(ring_shown);
        self.write_status(&view);
        self.chrome.info.set_text_content(Some(&board_text(
            size,
            trace.len(),
            trace.solution_count(),
            trace.allocated_bytes(),
        )));
        *self.committed.borrow_mut() = Some(view);
        if had_focus {
            if let Some(hit) = self.hit(active) {
                focus(&hit);
            }
        }
        self.hide_error();
        Ok(())
    }

    fn playing_changed(&self, playing: bool) {
        self.playing.set(playing);
        let _ = self.controls.set_playing(playing);
        if playing {
            // A tick was scheduled, so whatever stopped playback before has been put right.
            self.hide_error();
        }
        self.show_playback();
    }

    fn say(&self, text: &str) {
        // A message that repeats the last one is still read out, since the text differs by an invisible space.
        let n = self.said.get();
        self.said.set(n.wrapping_add(1));
        let tail = if n % 2 == 0 { "" } else { "\u{a0}" };
        self.chrome.live.set_text_content(Some(&format!("{text}{tail}")));
    }

    fn failed(&self, problem: &str) {
        let message = format!(
            "The board could not be drawn: {problem}. What is shown may be out of date. The next button, key or move \
             of the range draws it again."
        );
        self.chrome.error.set_text_content(Some(&message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
        self.say(&message);
    }

    fn playback_failed(&self, problem: &str) {
        let message = format!(
            "Play could not carry on: {problem}. The board is right, and the search is paused where it is. Press Play \
             to try again."
        );
        self.chrome.error.set_text_content(Some(&message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
        self.say(&message);
    }
}
