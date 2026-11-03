//! Which snapshot of the search is shown, and what moves it. Pure: nothing here touches the DOM, so every rule of the
//! buttons, the range, the playback and the repair of a page that was only half written is tested without a browser.
//!
//! The controller owns the one number that matters, the index of the snapshot that is on the page. It changes that
//! number only after the [`Surface`] has drawn the snapshot, so a failed write never leaves the index ahead of the
//! page.
//! A write that fails part way marks the page dirty, and the first thing that every button does is to draw it again.

use super::{
    projection::{
        AT_FIRST, AT_LAST, NO_EARLIER_SOLUTION, NO_LATER_SOLUTION, announced_while_playing, pause_summary, project,
        summary,
    },
    solver::Trace,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controller draws on, and says to a person, which in the page is the DOM.
pub(crate) trait Surface {
    /// Draws the snapshot at `index`. When `full` is set, everything is written again. Otherwise only what differs from
    /// the snapshot that was drawn last is written.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed. What is on the page is then unknown, and the next draw must be a full one.
    fn draw(&self, trace: &Trace, index: usize, full: bool) -> Result<(), String>;

    /// Replaces the board with one for `trace`, and draws its first snapshot. Nothing is changed if this fails.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the new board could not be made.
    fn rebuild(&self, trace: &Trace) -> Result<(), String>;

    /// Says that the search has started or stopped playing.
    fn playing_changed(&self, playing: bool);

    /// Tells a person something, in the polite live region.
    fn say(&self, text: &str);

    /// Reports a write that failed.
    fn failed(&self, problem: &str);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task once after a delay, and can take it back. The page uses the browser's timers, and a test uses
/// [`ManualScheduler`], which only runs a task when it is told to.
pub(crate) trait Scheduler {
    /// Runs `task` after `delay_ms` milliseconds, and gives a handle that [`Scheduler::cancel`] takes.
    fn schedule(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> u64;

    /// Takes back the task with `handle`, if it has not run.
    fn cancel(&self, handle: u64);
}

/// A task that is waiting: its handle, its delay, and what it runs.
type Waiting = (u64, u32, Box<dyn FnOnce()>);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scheduler that runs a task only when [`ManualScheduler::advance`] is called, and so needs no clock.
#[derive(Default)]
pub(crate) struct ManualScheduler {
    next: Cell<u64>,
    tasks: RefCell<Vec<Waiting>>,
    /// Whether a cancelled task stays, as a callback that was already on its way to the browser's queue would.
    keep_cancelled: Cell<bool>,
}

impl ManualScheduler {
    /// How many tasks are waiting.
    pub(crate) fn pending(&self) -> usize {
        self.tasks.borrow().len()
    }

    /// The delay of the task that is waiting, if there is one.
    #[cfg(test)]
    pub(crate) fn delay(&self) -> Option<u32> {
        self.tasks.borrow().first().map(|&(_, delay, _)| delay)
    }

    /// Runs the oldest waiting task, and says whether there was one. The task may schedule another.
    pub(crate) fn advance(&self) -> bool {
        let task = {
            let mut tasks = self.tasks.borrow_mut();
            if tasks.is_empty() { None } else { Some(tasks.remove(0)) }
        };
        match task {
            Some((_, _, task)) => {
                task();
                true
            },
            None => false,
        }
    }

    /// Makes [`Scheduler::cancel`] leave the task waiting, to test that a stale callback does nothing.
    #[cfg(test)]
    pub(crate) fn keep_cancelled(&self) {
        self.keep_cancelled.set(true);
    }
}

impl Scheduler for ManualScheduler {
    fn schedule(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> u64 {
        let handle = self.next.get() + 1;
        self.next.set(handle);
        self.tasks.borrow_mut().push((handle, delay_ms, task));
        handle
    }

    fn cancel(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.tasks.borrow_mut().retain(|&(at, _, _)| at != handle);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The latest place that the range has been moved to and not yet drawn. Moving it many times between two frames asks
/// for one draw, of the last place.
#[derive(Default)]
pub(crate) struct SeekRequest {
    latest: Cell<Option<usize>>,
}

impl SeekRequest {
    /// Records that the range is now at `index`, and says whether a frame has to be asked for, which is so only for
    /// the first request since the last [`SeekRequest::take`].
    pub(crate) fn request(&self, index: usize) -> bool {
        self.latest.replace(Some(index)).is_none()
    }

    /// The place that was last asked for, which is forgotten.
    pub(crate) fn take(&self) -> Option<usize> {
        self.latest.take()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this snapshot, or the last one if it is beyond the end.
    Seek(usize),
    PreviousSolution,
    NextSolution,
    Play,
    Pause,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the snapshot on the page, and the playback that moves it.
pub(crate) struct Controller {
    me: Weak<Controller>,
    trace: RefCell<Rc<Trace>>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    selected: Cell<usize>,
    /// Whether the page may be out of date, since a write failed part way.
    dirty: Cell<bool>,
    playing: Cell<bool>,
    /// Counted up each time playback stops, so that a tick that was scheduled before then knows it is out of date.
    epoch: Cell<u64>,
    /// The tick that is waiting. There is never more than one.
    waiting: Cell<Option<u64>>,
    /// How many snapshots are shown in each second while playing.
    speed: Cell<u32>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// A controller for `trace`, at snapshot 0, that draws on `surface` and plays with `scheduler`. The first snapshot
    /// is drawn at once, and if that fails the page is dirty.
    pub(crate) fn new(trace: Trace, surface: Rc<dyn Surface>, scheduler: Rc<dyn Scheduler>, speed: u32) -> Rc<Self> {
        let controller = Rc::new_cyclic(|me| Self {
            me: me.clone(),
            trace: RefCell::new(Rc::new(trace)),
            surface,
            scheduler,
            selected: Cell::new(0),
            dirty: Cell::new(false),
            playing: Cell::new(false),
            epoch: Cell::new(0),
            waiting: Cell::new(None),
            speed: Cell::new(speed.max(1)),
            torn_down: Cell::new(false),
        });
        controller.draw_first();
        controller
    }

    /// The trace on the page.
    pub(crate) fn trace(&self) -> Rc<Trace> {
        Rc::clone(&self.trace.borrow())
    }

    /// The index of the snapshot that is on the page, as far as the page is known.
    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected.get()
    }

    /// Whether the page may be out of date.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Whether the search is playing.
    pub(crate) fn is_playing(&self) -> bool {
        self.playing.get()
    }

    /// How many snapshots are shown in each second while playing.
    #[cfg(test)]
    pub(crate) fn speed(&self) -> u32 {
        self.speed.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn draw_first(&self) {
        if let Err(problem) = self.surface.draw(&self.trace(), 0, true) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    /// Draws the snapshot on the page again if a write failed, and says whether the page is now right. Every action
    /// that a person takes starts with this, even one that has nowhere to go.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        match self.surface.draw(&self.trace(), self.selected.get(), true) {
            Ok(()) => {
                self.dirty.set(false);
                true
            },
            Err(problem) => {
                self.surface.failed(&problem);
                false
            },
        }
    }

    /// Stops playback, takes back the tick that is waiting, and makes any that is already on its way do nothing.
    fn stop(&self) {
        self.epoch.set(self.epoch.get() + 1);
        if let Some(handle) = self.waiting.take() {
            self.scheduler.cancel(handle);
        }
        if self.playing.replace(false) {
            self.surface.playing_changed(false);
        }
    }

    /// Asks for the next tick, which is the only one that is ever waiting.
    fn schedule_next(&self) {
        debug_assert!(self.waiting.get().is_none(), "a tick is already waiting");
        let (epoch, me) = (self.epoch.get(), self.me.clone());
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.tick(epoch);
            }
        });
        let delay = (1000 / self.speed.get()).max(1);
        self.waiting.set(Some(self.scheduler.schedule(delay, task)));
    }

    /// Shows the next snapshot, and asks for the tick after it only once the page has been written. A tick from an
    /// earlier run of playback does nothing.
    fn tick(&self, epoch: u64) {
        if epoch != self.epoch.get() || !self.playing.get() || self.torn_down.get() {
            return;
        }
        self.waiting.set(None);
        let last = self.trace().len();
        let target = (self.selected.get() + 1).min(last);
        if !self.go(target, false) {
            return;
        }
        if target >= last {
            self.stop();
        } else {
            self.schedule_next();
        }
    }

    /// Draws the snapshot at `target` and, once that works, makes it the selected one. A failure leaves the selected
    /// index where it was, marks the page dirty, stops playback and reports it.
    fn go(&self, target: usize, announce: bool) -> bool {
        let trace = self.trace();
        match self.surface.draw(&trace, target, false) {
            Ok(()) => {
                self.selected.set(target);
                if let Some(view) = project(&trace, target) {
                    if announce || (self.playing.get() && announced_while_playing(view.kind)) {
                        self.surface.say(&summary(&view));
                    }
                }
                true
            },
            Err(problem) => {
                self.dirty.set(true);
                self.stop();
                self.surface.failed(&problem);
                false
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does what a person has asked for.
    ///
    /// A page that is dirty is drawn again first, even if the action then has nowhere to go. If that fails, nothing
    /// else happens. Every action but Pause stops playback, and Play then starts it if there is further to go.
    pub(crate) fn act(&self, action: Action) {
        if self.torn_down.get() {
            return;
        }
        if action == Action::Pause {
            if self.playing.get() {
                self.stop();
                self.say_paused();
            }
            return;
        }
        if !self.repair() {
            self.stop();
            return;
        }
        let trace = self.trace();
        let (at, last) = (self.selected.get(), trace.len());
        if action == Action::Play {
            match (self.playing.get(), at >= last) {
                (true, _) => {},
                (false, true) => self.surface.say(AT_LAST),
                (false, false) => {
                    self.playing.set(true);
                    self.surface.playing_changed(true);
                    self.schedule_next();
                },
            }
            return;
        }
        self.stop();
        let (target, nowhere) = match action {
            Action::Previous => (at.checked_sub(1), AT_FIRST),
            Action::Next => ((at < last).then_some(at + 1), AT_LAST),
            Action::Restart => ((at > 0).then_some(0), AT_FIRST),
            Action::Seek(index) => (Some(index.min(last)), ""),
            Action::PreviousSolution => (trace.previous_solution(at), NO_EARLIER_SOLUTION),
            Action::NextSolution => (trace.next_solution(at), NO_LATER_SOLUTION),
            Action::Play | Action::Pause => return,
        };
        match target {
            Some(target) if target != at => {
                self.go(target, !matches!(action, Action::Seek(_)));
            },
            _ if !nowhere.is_empty() => self.surface.say(nowhere),
            _ => {},
        }
    }

    /// Says where the search is, after a person has moved the range to rest.
    pub(crate) fn announce_current(&self) {
        let trace = self.trace();
        if let Some(view) = project(&trace, self.selected.get()) {
            self.surface.say(&summary(&view));
        }
    }

    fn say_paused(&self) {
        if let Some(view) = project(&self.trace(), self.selected.get()) {
            self.surface.say(&pause_summary(&view));
        }
    }

    /// Sets how many snapshots are shown in each second. While playing, the tick that is waiting is replaced, so that
    /// the new speed starts at once.
    pub(crate) fn set_speed(&self, per_second: u32) {
        self.speed.set(per_second.max(1));
        if self.playing.get() && !self.torn_down.get() {
            self.epoch.set(self.epoch.get() + 1);
            if let Some(handle) = self.waiting.take() {
                self.scheduler.cancel(handle);
            }
            self.schedule_next();
        }
    }

    /// Moves to a board of another size. Playback stops first. If the new board cannot be made, the old one stays,
    /// as it was, and the failure is reported.
    pub(crate) fn change_size(&self, trace: Trace) {
        if self.torn_down.get() {
            return;
        }
        self.stop();
        match self.surface.rebuild(&trace) {
            Ok(()) => {
                let size = trace.size();
                *self.trace.borrow_mut() = Rc::new(trace);
                self.selected.set(0);
                self.dirty.set(false);
                self.surface
                    .say(&format!("The board is now {size} by {size}, at the first step."));
            },
            Err(problem) => self.surface.failed(&problem),
        }
    }

    /// Stops playback for good, as when the page is left. Nothing the controller was asked to do then has any effect.
    pub(crate) fn teardown(&self) {
        self.stop();
        self.torn_down.set(true);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
