//! The page: the one scene, the text and the controls. It is what the controller draws on.

use super::{
    chrome::Chrome,
    controller::{Frame, Surface},
    controls::{ControlState, Controls, Journal},
    dom::focus,
    projection::{Presentation, Target, Write, bounds, diff, full, present},
    scene::{Board, build_board},
    solver::Trace,
    view::Camera,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) struct Page {
    controls: Controls,
    chrome: Chrome,
    board: Rc<Board>,
    /// Whether the last step lets the springs move the nodes of the condensation.
    springs: bool,
    /// The presentation that is on the page, or `None` when a write failed and what is there is not known.
    committed: RefCell<Option<Presentation>>,
    /// What the controls show, or `None` when a write to them failed.
    control_state: RefCell<Option<ControlState>>,
    playing: Cell<bool>,
    /// Whether every draw is a full one, which a test asks for, to compare a page that was diffed with one that was
    /// not.
    always_full: bool,
    error_shown: Cell<bool>,
    said: Cell<u32>,
}

impl Page {
    /// Draws the controls and the scene, and finds the elements that the window declares. The page is ready to be
    /// drawn on.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the panel is missing an element, or an element could not be made.
    pub(super) fn new(
        document: &web_sys::Document,
        graph_edges: &[(usize, usize)],
        trace: &Trace,
        instance: u32,
        always_full: bool,
        springs: bool,
    ) -> Result<Self, String> {
        let chrome = Chrome::find(document)?;
        let board = build_board(
            document,
            &chrome.scene,
            graph_edges,
            instance,
            trace.snapshot(0).map_or(0, |s| s.index.len()),
        )?;
        let state = ControlState {
            bounds: bounds(trace, 0),
            follow: true,
            index: 0,
            last: trace.last(),
            speed: super::projection::Speed::Normal,
            playing: false,
        };
        let controls = Controls::build(document, &board.controls_layer()?, &state)?;
        Ok(Self {
            controls,
            chrome,
            board: Rc::new(board),
            springs,
            committed: RefCell::new(None),
            control_state: RefCell::new(None),
            playing: Cell::new(false),
            always_full,
            error_shown: Cell::new(false),
            said: Cell::new(0),
        })
    }

    pub(super) fn controls(&self) -> &Controls {
        &self.controls
    }

    pub(super) fn board(&self) -> &Board {
        &self.board
    }

    /// Gives the keyboard focus to the element of `target`.
    pub(super) fn focus_target(&self, target: Target) {
        let element = match target {
            Target::Vertex(v) => self.board.vertex_element(v),
            Target::Edge(e) => self.board.edge_element(e),
        };
        if let Some(element) = element {
            focus(&element);
        }
    }

    fn hide_error(&self) {
        if self.error_shown.replace(false) {
            let _ = self.chrome.error.set_attribute("hidden", "");
        }
    }

    fn show_error(&self, message: &str) {
        self.chrome.error.set_text_content(Some(message));
        let _ = self.chrome.error.remove_attribute("hidden");
        self.error_shown.set(true);
    }

    fn control_state(&self, frame: &Frame) -> ControlState {
        ControlState {
            bounds: bounds(frame.trace, frame.index),
            follow: frame.choices.follow,
            index: frame.index,
            last: frame.trace.last(),
            speed: frame.choices.speed,
            playing: self.playing.get(),
        }
    }

    fn apply(&self, writes: &[Write]) -> Result<(), String> {
        for write in writes {
            match write {
                Write::Vertex(v, view) => self.board.write_vertex(*v, view)?,
                Write::Edge(e, view) => self.board.write_edge(*e, view)?,
                Write::Outlines(outlines) => self.board.write_outlines(outlines)?,
                Write::Stack(rows) => self.board.write_stack(rows)?,
                Write::Roster(rows) => self.board.write_roster(rows)?,
                Write::Dag(condensation, selected) => self.board.write_dag(condensation, *selected)?,
                Write::Panel(at, text) => {
                    if let Some(field) = self.chrome.panel.get(*at) {
                        field.set_text_content(Some(text));
                        // The notice about the view is hidden while there is nothing to say.
                        if *at == 3 {
                            let _ = if text.is_empty() {
                                field.set_attribute("hidden", "")
                            } else {
                                field.remove_attribute("hidden")
                            };
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

impl Page {
    /// Lets the springs move the nodes of the condensation at the last step, and puts them back at any other. A failure
    /// to put them back is reported like a failure to draw.
    fn settle_condensation(&self, frame: &Frame) -> Result<(), String> {
        if frame.index != frame.trace.last() {
            return self.board.rest();
        }
        if self.springs { self.board.relax() } else { Ok(()) }
    }
}

impl Surface for Page {
    fn draw(&self, frame: &Frame, full_draw: bool) -> Result<(), String> {
        let next = present(
            &super::projection::Context {
                graph: frame.graph,
                edges: frame.edges,
                trace: frame.trace,
                layout: frame.layout,
                camera: frame.camera,
                viewport: frame.viewport,
            },
            frame.index,
            frame.choices,
        )
        .ok_or_else(|| format!("there is no step {}", frame.index))?;
        let everything = full_draw || self.always_full;
        // Taken, so that a write that fails leaves nothing that claims to be on the page.
        let before = self.committed.borrow_mut().take().filter(|_| !everything);
        let previous_controls = self.control_state.borrow_mut().take().filter(|_| !everything);
        let writes = match &before {
            Some(before) => diff(before, &next),
            None => full(&next),
        };
        self.apply(&writes)?;
        let state = self.control_state(frame);
        // Kept, so that a control that was changed before a write failed is put back to what the page still commits.
        let mut journal = Journal::default();
        if let Err(problem) = self.controls.write_state(&mut journal, previous_controls.as_ref(), &state) {
            journal.restore();
            return Err(problem);
        }
        *self.committed.borrow_mut() = Some(next);
        *self.control_state.borrow_mut() = Some(state);
        self.hide_error();
        self.settle_condensation(frame)
    }

    fn camera(&self) -> Camera {
        self.board.camera()
    }

    fn viewport(&self) -> (f64, f64) {
        self.board.viewport()
    }

    fn write_camera(&self, camera: Camera) -> Result<(), String> {
        self.board.set_camera(camera)
    }

    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        let mut state = self.control_state.borrow_mut();
        let mut journal = Journal::default();
        let written = self.controls.write_play(&mut journal, playing);
        match (&written, state.as_mut()) {
            (Ok(()), Some(shown)) => shown.playing = playing,
            (Err(_), _) => {
                journal.restore();
                *state = None;
            },
            (Ok(()), None) => {},
        }
        written
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
            "The page could not be drawn: {problem}. What is shown may be out of date. The next button, key or move of the \
             range draws it again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    fn playback_failed(&self, problem: &str) {
        let message = format!(
            "Play could not carry on: {problem}. The page is right, and the run is paused where it is. Press Play to try again."
        );
        self.show_error(&message);
        self.say(&message);
    }

    fn warn(&self, text: &str) {
        self.show_error(&format!("Warning: {text}"));
    }
}
