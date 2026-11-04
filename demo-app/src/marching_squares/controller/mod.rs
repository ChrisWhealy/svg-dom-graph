//! Which state is on the page, and what moves it. Pure: nothing here touches the DOM, so every rule of the buttons, the
//! range, the selectors, the threshold edit, the switch of field and the repair of a page that was only half written is
//! tested without a browser.
//!
//! The controller owns the committed state, and changes it only after the [`Surface`] has drawn the new one. A write
//! that fails after the page was touched marks it dirty, and the first thing that every action does is to draw the
//! committed state again, even when the action then has nowhere to go.

use super::{
    field::{FieldId, LEVELS},
    projection::{Show, State, project},
    solver::{SolverError, Trace},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

pub(crate) const AT_FIRST: &str = "Already at the first step.";
pub(crate) const AT_LAST: &str = "Already at the last step.";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a draw did not happen. The difference is whether the page was touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommitError {
    /// Nothing a person can see changed: the new scene could not be built, or the swap failed and the old one is back.
    Detached(String),
    /// A write to the live page failed, and some of it may have changed.
    Live(String),
}

/// What kind of draw it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawKind {
    /// The plot items, the inspector and every control, in the scene that is on the page.
    Step,
    /// A new scene, built apart and swapped in, which keeps the zoom and the pan if `keep_view`.
    Scene { keep_view: bool },
}

/// The numbers that a draw is tagged with, which the diagnostic footer shows.
///
/// The generation is the committed one: it counts the traces and scenes that are on the page, and a request from before
/// it does nothing. The attempt counts every try to replace the trace or the scene, including one that failed, so no
/// two tries share a number, and a try that is made again after a failure is told apart from the one that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Meta {
    pub(crate) generation: u64,
    pub(crate) attempt: u64,
    pub(crate) token: u64,
}

/// What the controller draws on and says to a person, which in the page is the DOM.
pub(crate) trait Surface {
    /// Draws `state` of `trace`.
    ///
    /// # Errors
    ///
    /// Returns which kind of failure it was.
    fn draw(&self, trace: &Trace, state: &State, kind: DrawKind, meta: Meta) -> Result<(), CommitError>;

    /// Puts the selectors and the number back to `state`, after an edit that was refused.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed.
    fn sync_controls(&self, state: &State) -> Result<(), String>;

    /// Tells a person something, in the polite live region.
    fn say(&self, text: &str);

    /// Reports a refused edit or a write that failed.
    fn failed(&self, problem: &str);

    /// Shows a warning that does not stop anything, such as a frame that could not be asked for.
    fn warn(&self, text: &str);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task at the next animation frame, and can take it back. The page uses the browser's, and a test uses
/// [`ManualScheduler`].
pub(crate) trait Scheduler {
    /// Runs `task` at the next animation frame, and gives a handle to take it back with.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the frame could not be asked for, in which case the task will never run.
    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the frame with `handle`, if it has not come.
    fn cancel_frame(&self, handle: u64);

    /// Gives up everything that the scheduler holds in the browser, and any hook that it put on the page.
    fn teardown(&self) {}
}

type Waiting = (u64, Box<dyn FnOnce()>);

/// A scheduler that runs a frame only when [`ManualScheduler::advance_frame`] is called, and so needs no clock.
#[derive(Default)]
pub(crate) struct ManualScheduler {
    next: Cell<u64>,
    frames: RefCell<Vec<Waiting>>,
    /// Whether a cancelled frame stays, as a callback that was already on its way to the browser's queue would.
    keep_cancelled: Cell<bool>,
    failures: Cell<u32>,
    #[cfg(test)]
    teardowns: Cell<u32>,
}

impl ManualScheduler {
    pub(crate) fn pending_frames(&self) -> usize {
        self.frames.borrow().len()
    }

    /// Runs the oldest waiting frame, and says whether there was one.
    pub(crate) fn advance_frame(&self) -> bool {
        let task = {
            let mut frames = self.frames.borrow_mut();
            if frames.is_empty() { None } else { Some(frames.remove(0)) }
        };
        match task {
            Some((_, task)) => {
                task();
                true
            },
            None => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn teardowns(&self) -> u32 {
        self.teardowns.get()
    }

    #[cfg(test)]
    pub(crate) fn fail_next_frames(&self, count: u32) {
        self.failures.set(count);
    }

    #[cfg(test)]
    pub(crate) fn keep_cancelled(&self) {
        self.keep_cancelled.set(true);
    }
}

impl Scheduler for ManualScheduler {
    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        if self.failures.get() > 0 {
            self.failures.set(self.failures.get() - 1);
            return Err("the browser would not give a frame".to_owned());
        }
        let handle = self.next.get() + 1;
        self.next.set(handle);
        self.frames.borrow_mut().push((handle, task));
        Ok(handle)
    }

    fn cancel_frame(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.frames.borrow_mut().retain(|&(at, _)| at != handle);
        }
    }

    fn teardown(&self) {
        #[cfg(test)]
        self.teardowns.set(self.teardowns.get() + 1);
        self.frames.borrow_mut().clear();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person can ask for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this many cells visited, or all of them if it is beyond the end, and say where it is.
    Seek(usize),
    SetShow(Show),
    SetInspected(usize),
    /// Set the level of the inspected slot.
    SetThreshold(f64),
    SetField(FieldId),
    /// Choose a cell, or none.
    Select(Option<(usize, usize)>),
    /// Build the scene again from the state, and keep the view.
    Rebuild,
    /// Draw the page again if a write failed.
    Retry,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The committed state, the trace it comes from, and the requests that are waiting.
pub(crate) struct Controller {
    me: Weak<Controller>,
    trace: RefCell<Rc<Trace>>,
    state: Cell<State>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    /// Whether a write failed after the page was touched, so that what is shown is not known.
    dirty: Cell<bool>,
    /// Counted up whenever the trace or the scene is replaced, so that a request from before does nothing.
    generation: Cell<u64>,
    /// Counted up at every try to replace the trace or the scene, before it is drawn, so a failed try is counted too.
    attempts: Cell<u64>,
    /// Counted up for each frame that is asked for, so that a frame that was taken back but arrives anyway is not taken
    /// for the one that is waiting.
    token: Cell<u64>,
    /// The step that the range has been moved to and the frame is waiting to draw.
    pending: Cell<Option<usize>>,
    /// The frame that is waiting: its handle and its token.
    frame: Cell<Option<(u64, u64)>>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// Makes the controller for `trace`, shown at `state`, whose page is already drawn.
    pub(crate) fn new(trace: Trace, state: State, surface: Rc<dyn Surface>, scheduler: Rc<dyn Scheduler>) -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            me: me.clone(),
            trace: RefCell::new(Rc::new(trace)),
            state: Cell::new(state),
            surface,
            scheduler,
            dirty: Cell::new(false),
            generation: Cell::new(0),
            attempts: Cell::new(0),
            token: Cell::new(0),
            pending: Cell::new(None),
            frame: Cell::new(None),
            torn_down: Cell::new(false),
        })
    }

    pub(crate) fn state(&self) -> State {
        self.state.get()
    }

    pub(crate) fn trace(&self) -> Rc<Trace> {
        Rc::clone(&self.trace.borrow())
    }

    pub(crate) fn meta(&self) -> Meta {
        Meta {
            generation: self.generation.get(),
            attempt: self.attempts.get(),
            token: self.token.get(),
        }
    }

    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the committed state again if a write failed, and says whether the page is right now.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        match self.surface.draw(&self.trace(), &self.state.get(), DrawKind::Step, self.meta()) {
            Ok(()) => {
                self.dirty.set(false);
                true
            },
            Err(CommitError::Detached(problem) | CommitError::Live(problem)) => {
                self.surface.failed(&problem);
                false
            },
        }
    }

    /// Takes back a seek that is waiting for its frame, and the frame.
    fn cancel_pending(&self) {
        self.pending.set(None);
        if let Some((handle, _)) = self.frame.take() {
            self.scheduler.cancel_frame(handle);
        }
    }

    /// Draws `next` of `trace` in `kind` and, once that works, makes them the committed state. A failure leaves the
    /// committed state, and marks the page dirty if it was touched.
    fn commit(&self, trace: Option<Trace>, next: State, kind: DrawKind, announce: bool) -> bool {
        let new_trace = trace.map(Rc::new);
        let shown = new_trace.clone().unwrap_or_else(|| self.trace());
        let replaces = new_trace.is_some() || matches!(kind, DrawKind::Scene { .. });
        let generation = self.generation.get() + u64::from(replaces);
        // A try to replace is counted before it is drawn, so that one that fails is never counted again by the next.
        if replaces {
            self.attempts.set(self.attempts.get() + 1);
        }
        let meta = Meta {
            generation,
            attempt: self.attempts.get(),
            token: self.token.get(),
        };
        match self.surface.draw(&shown, &next, kind, meta) {
            Ok(()) => {
                if let Some(t) = new_trace {
                    *self.trace.borrow_mut() = t;
                }
                self.state.set(next);
                if replaces {
                    self.generation.set(generation);
                    self.cancel_pending();
                }
                if announce {
                    self.surface.say(&project(&shown, &next).status);
                }
                true
            },
            Err(error) => {
                let (CommitError::Detached(problem) | CommitError::Live(problem)) = &error;
                if matches!(error, CommitError::Live(_)) {
                    self.dirty.set(true);
                }
                self.surface.failed(problem);
                false
            },
        }
    }

    /// Goes to `step` cells visited, from the committed state.
    fn go_to(&self, step: usize, announce: bool) {
        let state = self.state.get();
        let last = self.trace().grid.cell_count();
        let step = step.min(last);
        if step != state.step {
            self.commit(None, State { step, ..state }, DrawKind::Step, announce);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does what a person has asked for.
    ///
    /// A page that is dirty is drawn again first, even if the action then has nowhere to go. If that fails, nothing
    /// else happens.
    pub(crate) fn act(&self, action: Action) {
        if self.torn_down.get() {
            return;
        }
        if !self.repair() {
            return;
        }
        let state = self.state.get();
        let last = self.trace().grid.cell_count();
        match action {
            Action::Retry => {},
            Action::Previous => match state.step.checked_sub(1) {
                Some(step) => self.go_to(step, true),
                None => self.surface.say(AT_FIRST),
            },
            Action::Restart => {
                if state.step == 0 {
                    self.surface.say(AT_FIRST);
                } else {
                    self.go_to(0, true);
                }
            },
            Action::Next => {
                if state.step >= last {
                    self.surface.say(AT_LAST);
                } else {
                    self.go_to(state.step + 1, true);
                }
            },
            Action::Seek(step) => self.go_to(step, true),
            Action::SetShow(show) => {
                if show != state.show {
                    self.commit(None, State { show, ..state }, DrawKind::Step, true);
                }
            },
            Action::SetInspected(slot) => {
                if slot < LEVELS && slot != state.inspected {
                    self.commit(None, State { inspected: slot, ..state }, DrawKind::Step, true);
                }
            },
            Action::Select(cell) => {
                let across = self.trace().grid.cells_across();
                let down = self.trace().grid.height - 1;
                if cell.is_some_and(|(r, c)| r >= down || c >= across) {
                    self.surface.failed("that cell is not in the field");
                } else if cell != state.selected {
                    self.commit(None, State { selected: cell, ..state }, DrawKind::Step, true);
                }
            },
            Action::SetThreshold(value) => self.set_threshold(value),
            Action::SetField(field) => self.set_field(field),
            Action::Rebuild => {
                self.commit(None, state, DrawKind::Scene { keep_view: true }, false);
            },
        }
    }

    /// Replaces the level of the inspected slot. A value that is refused leaves the committed state, and the number
    /// goes back to it.
    fn set_threshold(&self, value: f64) {
        let state = self.state.get();
        let mut levels = state.levels;
        levels[state.inspected] = value;
        if levels == state.levels {
            return;
        }
        match Trace::build(self.trace().grid.clone(), levels) {
            Ok(trace) => {
                self.commit(Some(trace), State { levels, ..state }, DrawKind::Step, true);
            },
            Err(problem) => self.refuse(&problem),
        }
    }

    /// Switches to `field`, shown at its last step. The zoom and the pan are kept only between fields that share axes.
    fn set_field(&self, field: FieldId) {
        let state = self.state.get();
        if field == state.field {
            return;
        }
        let built = field.grid().and_then(|grid| Trace::build(grid, field.levels()));
        match built {
            Ok(trace) => {
                let step = trace.grid.cell_count();
                let next = State {
                    step,
                    selected: None,
                    ..State::first(field)
                };
                let next = State {
                    show: state.show,
                    inspected: state.inspected,
                    ..next
                };
                self.commit(
                    Some(trace),
                    next,
                    DrawKind::Scene {
                        keep_view: state.field.same_extent(field),
                    },
                    true,
                );
            },
            Err(problem) => self.refuse(&problem),
        }
    }

    /// Says why an edit was refused, and puts the controls back to the committed state.
    fn refuse(&self, problem: &SolverError) {
        self.surface.failed(&problem.to_string());
        if self.surface.sync_controls(&self.state.get()).is_err() {
            self.dirty.set(true);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The range has been moved to `step`. The latest place is drawn at the next frame, so moving it many times between
    /// two frames draws once. The frame carries the generation and a token of its own, and does nothing unless both are
    /// still current.
    pub(crate) fn seek_input(&self, step: usize) {
        if self.torn_down.get() {
            return;
        }
        let first = self.pending.replace(Some(step)).is_none();
        if !first {
            return;
        }
        let (generation, me) = (self.generation.get(), self.me.clone());
        let token = self.token.get() + 1;
        self.token.set(token);
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_frame(generation, token);
            }
        });
        match self.scheduler.request_frame(task) {
            Ok(handle) => self.frame.set(Some((handle, token))),
            Err(problem) => {
                // No frame will come, so the latest place is drawn now, once, and a person is warned.
                self.surface.warn(&format!("Could not ask for a frame: {problem}"));
                self.flush_pending();
            },
        }
    }

    fn on_frame(&self, generation: u64, token: u64) {
        let waiting = self.frame.get().map(|(_, token)| token);
        if self.torn_down.get() || generation != self.generation.get() || waiting != Some(token) {
            return;
        }
        self.frame.set(None);
        self.flush_pending();
    }

    /// Draws the place that the range was moved to, after repairing the page, and says nothing.
    fn flush_pending(&self) {
        if let Some(step) = self.pending.take() {
            if self.repair() {
                self.go_to(step, false);
            }
        }
    }

    /// The range has been let go at `step`. What is waiting is dropped, `step` is drawn now, and it is said once.
    pub(crate) fn seek_release(&self, step: usize) {
        if self.torn_down.get() {
            return;
        }
        self.cancel_pending();
        let before = self.state.get().step;
        self.act(Action::Seek(step));
        if self.state.get().step == before && !self.dirty.get() {
            let state = self.state.get();
            self.surface.say(&project(&self.trace(), &state).status);
        }
    }

    /// Stops everything for good, as when the page is left. Nothing that was asked of the controller then has any
    /// effect.
    pub(crate) fn teardown(&self) {
        self.cancel_pending();
        self.torn_down.set(true);
        self.scheduler.teardown();
    }
}

#[cfg(test)]
mod unit_tests;
