//! Which snapshot is shown, what moves it, and the playback. Pure: nothing here touches the DOM, so every rule of the
//! buttons, the range, the playback, the switch of graph and the repair of a page that was only half written is tested
//! without a browser.
//!
//! The controller owns the index of the snapshot on the page, and changes it only after the [`Surface`] has drawn that
//! snapshot. A write that fails part way marks the page dirty, and the first thing that every action does is to draw
//! the page again in full, even when the action then has nowhere to go.

use super::{
    fixtures::Fixture,
    projection::{
        AT_FIRST, AT_LAST, NO_EARLIER_AUGMENTATION, NO_LATER_AUGMENTATION, Pref, Speed, Target, View,
        announced_while_playing, pause_summary, summary,
    },
    solver::Trace,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a switch of graph did not happen. The difference is whether the page was touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SwitchError {
    /// The new graph could not be built or drawn apart from the page. Nothing live was touched.
    Detached(String),
    /// A write to the live page failed. Some controls may have been changed, and are put back as far as can be.
    Live(String),
}

/// What the controller draws on and says to a person, which in the page is the DOM.
pub(crate) trait Surface {
    /// Draws the snapshot at `index` with `view`. When `full` is set every element is written. Otherwise only what
    /// differs from what was drawn last is.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed. What is on the page is then unknown, and the next draw must be a full one.
    fn draw(&self, trace: &Trace, index: usize, view: &View, full: bool) -> Result<(), String>;

    /// Replaces the graph on the page with `fixture`, drawn at its first snapshot, and takes the controls to match.
    ///
    /// Everything that can fail is done before the old graph goes. The live controls are written with a journal, and
    /// put back if the write that follows fails. The generation is the one that the new graph's IDs carry.
    ///
    /// # Errors
    ///
    /// Returns which kind of failure it was.
    fn switch(&self, fixture: &'static Fixture, trace: &Trace, view: &View, generation: u64)
    -> Result<(), SwitchError>;

    /// Says that playback started or stopped, and writes it on the Play button.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be written, so what it says is wrong until the next full draw.
    fn playing_changed(&self, playing: bool) -> Result<(), String>;

    /// Tells a person something, in the polite live region.
    fn say(&self, text: &str);

    /// Reports a write that failed.
    fn failed(&self, problem: &str);

    /// Reports that playback could not carry on, because no tick could be scheduled. The page is right.
    fn playback_failed(&self, problem: &str);

    /// Shows a warning that does not stop anything, such as a frame that could not be asked for.
    fn warn(&self, text: &str);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task after a delay or at the next frame, and can take it back. The page uses the browser's, and a test uses
/// [`ManualScheduler`], which runs a task only when it is told to.
pub(crate) trait Scheduler {
    /// Runs `task` after `delay_ms` milliseconds, and gives a handle that [`Scheduler::cancel`] takes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the task could not be scheduled, in which case it will never run.
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the task with `handle`, if it has not run.
    fn cancel(&self, handle: u64);

    /// Runs `task` at the next animation frame.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the frame could not be asked for, in which case the task will never run.
    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the frame with `handle`, if it has not come.
    fn cancel_frame(&self, handle: u64);

    /// Gives up everything that the scheduler holds in the browser: the tasks that are waiting, and any hook that it
    /// put on the page. Nothing is scheduled after this.
    fn teardown(&self) {}
}

/// A task that is waiting: its handle, its delay, and what it runs.
type Waiting = (u64, u32, Box<dyn FnOnce()>);

/// A scheduler that runs a task only when [`ManualScheduler::advance`] or [`ManualScheduler::advance_frame`] is called,
/// and so needs no clock.
#[derive(Default)]
pub(crate) struct ManualScheduler {
    next: Cell<u64>,
    timers: RefCell<Vec<Waiting>>,
    frames: RefCell<Vec<Waiting>>,
    /// Whether a cancelled task stays, as a callback that was already on its way to the browser's queue would.
    keep_cancelled: Cell<bool>,
    timer_failures: Cell<u32>,
    frame_failures: Cell<u32>,
    #[cfg(test)]
    teardowns: Cell<u32>,
}

impl ManualScheduler {
    /// How many timers are waiting, and how many frames.
    pub(crate) fn pending(&self) -> usize {
        self.timers.borrow().len()
    }

    pub(crate) fn pending_frames(&self) -> usize {
        self.frames.borrow().len()
    }

    /// The delay of the timer that is waiting, if there is one.
    #[cfg(test)]
    pub(crate) fn delay(&self) -> Option<u32> {
        self.timers.borrow().first().map(|&(_, delay, _)| delay)
    }

    fn run_first(list: &RefCell<Vec<Waiting>>) -> bool {
        let task = {
            let mut tasks = list.borrow_mut();
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

    /// Runs the oldest waiting timer, and says whether there was one. The task may schedule another.
    pub(crate) fn advance(&self) -> bool {
        Self::run_first(&self.timers)
    }

    /// Runs the oldest waiting frame, and says whether there was one.
    pub(crate) fn advance_frame(&self) -> bool {
        Self::run_first(&self.frames)
    }

    #[cfg(test)]
    pub(crate) fn teardowns(&self) -> u32 {
        self.teardowns.get()
    }

    /// Makes the next `count` timers fail to be scheduled.
    #[cfg(test)]
    pub(crate) fn fail_next_timers(&self, count: u32) {
        self.timer_failures.set(count);
    }

    /// Makes the next `count` frames fail to be asked for.
    #[cfg(test)]
    pub(crate) fn fail_next_frames(&self, count: u32) {
        self.frame_failures.set(count);
    }

    /// Makes cancelling leave the task waiting, to test that a stale callback does nothing.
    #[cfg(test)]
    pub(crate) fn keep_cancelled(&self) {
        self.keep_cancelled.set(true);
    }

    fn push(
        &self,
        list: &RefCell<Vec<Waiting>>,
        failures: &Cell<u32>,
        delay: u32,
        task: Box<dyn FnOnce()>,
    ) -> Result<u64, String> {
        if failures.get() > 0 {
            failures.set(failures.get() - 1);
            return Err("the browser would not schedule it".to_owned());
        }
        let handle = self.next.get() + 1;
        self.next.set(handle);
        list.borrow_mut().push((handle, delay, task));
        Ok(handle)
    }
}

impl Scheduler for ManualScheduler {
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.push(&self.timers, &self.timer_failures, delay_ms, task)
    }

    fn cancel(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.timers.borrow_mut().retain(|&(at, _, _)| at != handle);
        }
    }

    fn request_frame(&self, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        self.push(&self.frames, &self.frame_failures, 0, task)
    }

    fn cancel_frame(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.frames.borrow_mut().retain(|&(at, _, _)| at != handle);
        }
    }

    fn teardown(&self) {
        #[cfg(test)]
        self.teardowns.set(self.teardowns.get() + 1);
        self.timers.borrow_mut().clear();
        self.frames.borrow_mut().clear();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person can ask for.
#[derive(Clone, Copy)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this snapshot, or the last one if it is beyond the end.
    Seek(usize),
    PreviousAugmentation,
    NextAugmentation,
    JumpToCut,
    Play,
    Pause,
    Toggle(Pref),
    /// Focus a vertex or an edge. This is informational, and never changes the index.
    Focus(Target),
    SetSpeed(Speed),
    SwitchFixture(&'static Fixture),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the snapshot on the page, the view, and the playback that moves the index.
pub(crate) struct Controller {
    me: Weak<Controller>,
    trace: RefCell<Rc<Trace>>,
    fixture: Cell<&'static Fixture>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    selected: Cell<usize>,
    view: Cell<View>,
    /// Whether the page may be out of date, since a write failed part way.
    dirty: Cell<bool>,
    /// Counted up each time a new graph is committed, and the number in the ID of everything drawn for it.
    generation: Cell<u64>,
    /// Counted up at every logical stop of playback, so that a tick that was scheduled before it knows it is stale.
    epoch: Cell<u64>,
    /// The tick that is waiting. There is never more than one.
    tick: Cell<Option<u64>>,
    /// Whether a tick is being worked on, which is not a stop.
    ticking: Cell<bool>,
    /// What the page was last told about playing, so that it is told only of a change.
    shown_playing: Cell<bool>,
    /// The latest place that the range has been moved to and not yet drawn, with the generation it was made against.
    pending_seek: Cell<Option<(usize, u64)>>,
    /// The frame that a seek is waiting for: its handle, and the token that its task carries.
    frame: Cell<Option<(u64, u64)>>,
    /// Counted up for each frame that is asked for, so that a frame that was taken back but arrives anyway is not taken
    /// for the one that is waiting.
    frame_tokens: Cell<u64>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// A controller for `fixture`, at snapshot 0. The first snapshot is drawn at once, and if that fails the page is
    /// dirty.
    pub(crate) fn new(
        fixture: &'static Fixture,
        trace: Trace,
        view: View,
        surface: Rc<dyn Surface>,
        scheduler: Rc<dyn Scheduler>,
    ) -> Rc<Self> {
        let controller = Rc::new_cyclic(|me| Self {
            me: me.clone(),
            trace: RefCell::new(Rc::new(trace)),
            fixture: Cell::new(fixture),
            surface,
            scheduler,
            selected: Cell::new(0),
            view: Cell::new(view),
            dirty: Cell::new(false),
            generation: Cell::new(0),
            epoch: Cell::new(0),
            tick: Cell::new(None),
            ticking: Cell::new(false),
            shown_playing: Cell::new(false),
            pending_seek: Cell::new(None),
            frame: Cell::new(None),
            frame_tokens: Cell::new(0),
            torn_down: Cell::new(false),
        });
        if let Err(problem) = controller.surface.draw(&controller.trace(), 0, &view, true) {
            controller.dirty.set(true);
            controller.surface.failed(&problem);
        }
        controller
    }

    pub(crate) fn trace(&self) -> Rc<Trace> {
        Rc::clone(&self.trace.borrow())
    }

    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected.get()
    }

    pub(crate) fn view(&self) -> View {
        self.view.get()
    }

    pub(crate) fn fixture(&self) -> &'static Fixture {
        self.fixture.get()
    }

    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// The generation of the graph that is on the page.
    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation.get()
    }

    /// How many times playback has stopped. A tick that carries an older number does nothing.
    #[cfg(test)]
    pub(crate) fn playback_epoch(&self) -> u64 {
        self.epoch.get()
    }

    /// Whether a tick is waiting for the current epoch, which is all that playing is.
    pub(crate) fn is_playing(&self) -> bool {
        self.tick.get().is_some() || self.ticking.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the page again in full if a write failed, and says whether it is right now. Every action starts with this.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        match self.surface.draw(&self.trace(), self.selected.get(), &self.view.get(), true) {
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

    /// Stops playback, and counts the stop, whether or not anything was waiting and whether or not taking it back
    /// worked.
    fn stop_playback(&self) {
        self.epoch.set(self.epoch.get() + 1);
        if let Some(handle) = self.tick.take() {
            self.scheduler.cancel(handle);
        }
        if self.shown_playing.replace(false) {
            self.show_playing(false);
        }
    }

    /// Tells the page that playback started or stopped. A button that could not be written is left saying the wrong
    /// thing, so the page is dirty, and the next action draws it again before anything else.
    fn show_playing(&self, playing: bool) {
        if let Err(problem) = self.surface.playing_changed(playing) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    /// Asks for the next tick, which is the only one that is ever waiting. It carries the epoch and the generation that
    /// it was asked for.
    fn schedule_next(&self) -> Result<(), String> {
        debug_assert!(self.tick.get().is_none(), "a tick is already waiting");
        let (epoch, generation, me) = (self.epoch.get(), self.generation.get(), self.me.clone());
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_tick(epoch, generation);
            }
        });
        let delay = self.view.get().speed.interval_ms();
        self.tick.set(Some(self.scheduler.schedule_once(delay, task)?));
        Ok(())
    }

    /// Stops playback because no tick could be scheduled, and says so. The page is right, so it is not dirty.
    fn schedule_failed(&self, problem: &str) {
        self.stop_playback();
        self.surface.playback_failed(problem);
    }

    /// Shows the next snapshot, and asks for the tick after it only once the page has been written. A tick from before
    /// a stop, or from another graph, does nothing.
    fn on_tick(&self, epoch: u64, generation: u64) {
        if self.torn_down.get() || epoch != self.epoch.get() || generation != self.generation.get() {
            return;
        }
        self.tick.set(None);
        self.ticking.set(true);
        let last = self.trace().last();
        let target = (self.selected.get() + 1).min(last);
        let moved = self.go(target, false);
        self.ticking.set(false);
        if !moved {
            return;
        }
        if target >= last {
            self.stop_playback();
        } else if let Err(problem) = self.schedule_next() {
            self.schedule_failed(&problem);
        }
    }

    /// Draws the snapshot at `target` and, once that works, makes it the selected one. A failure leaves the selected
    /// index where it was, marks the page dirty, stops playback and reports it.
    fn go(&self, target: usize, announce: bool) -> bool {
        let trace = self.trace();
        match self.surface.draw(&trace, target, &self.view.get(), false) {
            Ok(()) => {
                self.selected.set(target);
                if let Some(snapshot) = trace.snapshot(target) {
                    if announce || (self.is_playing() && announced_while_playing(&snapshot.event)) {
                        self.surface.say(&summary(&trace, snapshot));
                    }
                }
                true
            },
            Err(problem) => {
                self.dirty.set(true);
                self.stop_playback();
                self.surface.failed(&problem);
                false
            },
        }
    }

    /// Commits a change of view the way an index is committed: only after the page has been drawn with it.
    fn commit_view(&self, view: View) {
        match self.surface.draw(&self.trace(), self.selected.get(), &view, false) {
            Ok(()) => self.view.set(view),
            Err(problem) => {
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does what a person has asked for.
    ///
    /// A page that is dirty is drawn again first, even if the action then has nowhere to go. If that fails, nothing
    /// else happens. Every action that moves the index stops playback, and so does one that cannot move it.
    pub(crate) fn act(&self, action: Action) {
        if self.torn_down.get() {
            return;
        }
        if matches!(action, Action::Pause) {
            if self.is_playing() {
                self.stop_playback();
                if let Some(snapshot) = self.trace().snapshot(self.selected.get()) {
                    self.surface.say(&pause_summary(&self.trace(), snapshot));
                }
            }
            return;
        }
        if !self.repair() {
            self.stop_playback();
            return;
        }
        let trace = self.trace();
        let (at, last) = (self.selected.get(), trace.last());
        match action {
            Action::Pause => {},
            Action::Play => self.play(at, last),
            Action::Toggle(pref) => {
                let mut view = self.view.get();
                view.prefs = view.prefs.toggled(pref);
                self.commit_view(view);
            },
            Action::Focus(target) => {
                let mut view = self.view.get();
                view.focused = Some(target);
                view.roving = target;
                self.commit_view(view);
            },
            Action::SetSpeed(speed) => {
                let mut view = self.view.get();
                view.speed = speed;
                self.view.set(view);
                if self.is_playing() {
                    // The old tick is stale, and the new one has the new interval.
                    self.stop_playback();
                    self.start_playing();
                }
            },
            Action::SwitchFixture(fixture) => self.switch(fixture),
            navigation => {
                self.stop_playback();
                let (target, nowhere) = match navigation {
                    Action::Previous => (at.checked_sub(1), AT_FIRST),
                    Action::Next => ((at < last).then_some(at + 1), AT_LAST),
                    Action::Restart => ((at > 0).then_some(0), AT_FIRST),
                    Action::Seek(index) => (Some(index.min(last)), ""),
                    Action::PreviousAugmentation => (trace.previous_augmentation(at), NO_EARLIER_AUGMENTATION),
                    Action::NextAugmentation => (trace.next_augmentation(at), NO_LATER_AUGMENTATION),
                    Action::JumpToCut => ((at < last).then_some(last), AT_LAST),
                    _ => return,
                };
                match target {
                    Some(target) if target != at => {
                        self.go(target, !matches!(navigation, Action::Seek(_)));
                    },
                    _ if !nowhere.is_empty() => self.surface.say(nowhere),
                    _ => {},
                }
            },
        }
    }

    /// Starts playing if there is further to go, and says so if there is not.
    fn play(&self, at: usize, last: usize) {
        if self.is_playing() {
            return;
        }
        if at >= last {
            self.surface.say(AT_LAST);
            return;
        }
        self.start_playing();
    }

    /// Schedules the first tick, and claims to be playing only once one is waiting.
    fn start_playing(&self) {
        match self.schedule_next() {
            Ok(()) => {
                self.shown_playing.set(true);
                self.show_playing(true);
            },
            Err(problem) => self.surface.playback_failed(&problem),
        }
    }

    /// Says where the search is, after a person has moved the range to rest.
    pub(crate) fn announce_current(&self) {
        let trace = self.trace();
        if let Some(snapshot) = trace.snapshot(self.selected.get()) {
            self.surface.say(&summary(&trace, snapshot));
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The range has been moved to `index`. Playback stops at once, and the latest place is drawn at the next frame, so
    /// moving it many times between two frames draws once.
    pub(crate) fn seek_input(&self, index: usize) {
        if self.torn_down.get() {
            return;
        }
        self.stop_playback();
        let first = self.pending_seek.replace(Some((index, self.generation.get()))).is_none();
        if !first {
            return;
        }
        let (generation, me) = (self.generation.get(), self.me.clone());
        let token = self.frame_tokens.get() + 1;
        self.frame_tokens.set(token);
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
                self.flush_seek(None);
            },
        }
    }

    /// Draws the place that was asked for, if this is the frame that is waiting. A frame that was taken back, or that
    /// is for another graph, does nothing, even if it arrives while a newer frame is waiting.
    fn on_frame(&self, generation: u64, token: u64) {
        let waiting = self.frame.get().map(|(_, token)| token);
        if self.torn_down.get() || generation != self.generation.get() || waiting != Some(token) {
            return;
        }
        self.frame.set(None);
        self.flush_seek(None);
    }

    /// Draws the place that the range was moved to, if it is for the graph that is on the page. A request for another
    /// graph is dropped, and `fallback` is the place that is drawn in its stead, if there is one.
    fn flush_seek(&self, fallback: Option<usize>) {
        let pending = self.pending_seek.take();
        let index = match pending {
            Some((index, generation)) if generation == self.generation.get() => Some(index),
            _ => fallback,
        };
        if let Some(index) = index {
            self.act(Action::Seek(index));
        }
    }

    /// The range has been let go at `index`. What is waiting is drawn now, and once the page has it, it is said once.
    pub(crate) fn seek_release(&self, index: usize) {
        if self.torn_down.get() {
            return;
        }
        if let Some((handle, _)) = self.frame.take() {
            self.scheduler.cancel_frame(handle);
        }
        self.stop_playback();
        self.flush_seek(Some(index));
        if !self.dirty.get() {
            self.announce_current();
        }
    }

    /// Takes back a seek that is waiting for its frame, and the frame.
    fn cancel_pending_seek(&self) {
        self.pending_seek.set(None);
        if let Some((handle, _)) = self.frame.take() {
            self.scheduler.cancel_frame(handle);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Switches to `fixture`. Playback stops first, whether or not the switch works. The new graph is built and drawn
    /// apart from the page, and committed only if every write to the live page works. A failure leaves the old graph,
    /// its index and its generation as they were, and a failure that touched the live page leaves it dirty.
    fn switch(&self, fixture: &'static Fixture) {
        if fixture.key == self.fixture.get().key {
            return;
        }
        let trace = match Trace::build(&fixture.graph()) {
            Ok(trace) => trace,
            Err(problem) => {
                self.stop_playback();
                self.cancel_pending_seek();
                self.surface.failed(&problem.to_string());
                return;
            },
        };
        let previous = self.view.get();
        let view = View::first(previous.prefs, previous.speed);
        let generation = self.generation.get() + 1;
        match self.surface.switch(fixture, &trace, &view, generation) {
            Ok(()) => {
                *self.trace.borrow_mut() = Rc::new(trace);
                self.fixture.set(fixture);
                self.view.set(view);
                self.selected.set(0);
                self.generation.set(generation);
                self.dirty.set(false);
                self.stop_playback();
                self.cancel_pending_seek();
                self.surface.say(&format!("Showing {}, at the first step.", fixture.label));
            },
            Err(error) => {
                self.stop_playback();
                self.cancel_pending_seek();
                match error {
                    SwitchError::Detached(problem) => self.surface.failed(&problem),
                    SwitchError::Live(problem) => {
                        self.dirty.set(true);
                        self.surface.failed(&problem);
                    },
                }
            },
        }
    }

    /// Stops playback for good, as when the page is left. Nothing that was asked of the controller then has any effect.
    pub(crate) fn teardown(&self) {
        self.stop_playback();
        self.cancel_pending_seek();
        self.torn_down.set(true);
        self.scheduler.teardown();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
