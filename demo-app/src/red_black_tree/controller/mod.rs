//! Which snapshot is shown, what moves it, who is chosen, and the playback. Pure: nothing here touches the DOM, so
//! every rule of the buttons, the range, the keys, the playback and the repair of a page that was only half written is
//! tested without a browser.
//!
//! The controller owns the index of the snapshot on the page, and changes it only after the [`Surface`] has committed
//! that snapshot. A write that fails after the page was touched marks it dirty, and the first thing that every action
//! does is to commit the page again, even when the action then has nowhere to go.

use super::{
    a11y::{AT_FIRST, AT_LAST, Nav, announced_while_playing, item_name, navigate, resolve_roving, summary},
    solver::{Key, Trace},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

/// How long a step of the playback lasts. There is no speed control.
pub(crate) const STEP_MS: u32 = 500;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person has chosen and where the single tab stop is. Both are intents, which each snapshot resolves anew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct View {
    pub(crate) chosen: Option<Key>,
    pub(crate) roving: Option<Key>,
}

/// Why a commit did not happen. The difference is whether the page was touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommitError {
    /// The new tree could not be built or measured apart from the page, or could not be swapped in and the old one is
    /// back. Nothing a person can see changed.
    Detached(String),
    /// A write to the live page failed. Some of it may have changed.
    Live(String),
}

/// What the controller draws on and says to a person, which in the page is the DOM.
pub(crate) trait Surface {
    /// Builds the tree at `index` apart from the page, then swaps it in, and restores the focus to the same key.
    ///
    /// # Errors
    ///
    /// Returns which kind of failure it was.
    fn commit(&self, trace: &Trace, index: usize, view: &View) -> Result<(), CommitError>;

    /// Shows who is selected and where the tab stop is, without rebuilding anything.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed. What is on the page is then unknown.
    fn project(&self, trace: &Trace, index: usize, view: &View) -> Result<(), String>;

    /// Gives the keyboard focus to the item of `key`.
    fn focus_key(&self, key: Key);

    /// Says that playback started or stopped, and writes it on the Play button.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be written, so what it says is wrong until the next commit.
    fn playing_changed(&self, playing: bool) -> Result<(), String>;

    /// Tells a person something, in the polite live region.
    fn say(&self, text: &str);

    /// Reports a write that failed.
    fn failed(&self, problem: &str);

    /// Reports that playback could not carry on, because no tick could be scheduled. The page is right.
    fn playback_failed(&self, problem: &str);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a task after a delay, and can take it back. The page uses the browser's, and a test uses [`ManualScheduler`].
pub(crate) trait Scheduler {
    /// Runs `task` once after `delay_ms`, and gives a handle to take it back with.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the timer could not be set, in which case the task will never run.
    fn schedule_once(&self, delay_ms: u32, task: Box<dyn FnOnce()>) -> Result<u64, String>;

    /// Takes back the task with `handle`, if it has not run.
    fn cancel(&self, handle: u64);

    /// Gives up everything that the scheduler holds in the browser, and any hook that it put on the page. Nothing is
    /// scheduled after this.
    fn teardown(&self) {}
}

/// A task that is waiting: its handle and what it runs.
type Waiting = (u64, Box<dyn FnOnce()>);

/// A scheduler that runs a task only when [`ManualScheduler::advance`] is called, and so needs no clock.
#[derive(Default)]
pub(crate) struct ManualScheduler {
    next: Cell<u64>,
    timers: RefCell<Vec<Waiting>>,
    /// Whether a cancelled task stays, as a callback that was already on its way to the browser's queue would.
    keep_cancelled: Cell<bool>,
    failures: Cell<u32>,
    #[cfg(test)]
    teardowns: Cell<u32>,
}

impl ManualScheduler {
    /// The number of tasks that are waiting.
    pub(crate) fn pending(&self) -> usize {
        self.timers.borrow().len()
    }

    /// Runs the oldest waiting task, and says whether there was one. The task may schedule another.
    pub(crate) fn advance(&self) -> bool {
        let task = {
            let mut tasks = self.timers.borrow_mut();
            if tasks.is_empty() { None } else { Some(tasks.remove(0)) }
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

    /// Makes the next `count` timers fail to be scheduled.
    #[cfg(test)]
    pub(crate) fn fail_next(&self, count: u32) {
        self.failures.set(count);
    }

    /// Makes a cancelled task stay, to stand in for one that was already on its way.
    #[cfg(test)]
    pub(crate) fn keep_cancelled(&self) {
        self.keep_cancelled.set(true);
    }
}

impl Scheduler for ManualScheduler {
    fn schedule_once(&self, _: u32, task: Box<dyn FnOnce()>) -> Result<u64, String> {
        if self.failures.get() > 0 {
            self.failures.set(self.failures.get() - 1);
            return Err("the browser would not schedule it".to_owned());
        }
        let handle = self.next.get() + 1;
        self.next.set(handle);
        self.timers.borrow_mut().push((handle, task));
        Ok(handle)
    }

    fn cancel(&self, handle: u64) {
        if !self.keep_cancelled.get() {
            self.timers.borrow_mut().retain(|&(at, _)| at != handle);
        }
    }

    fn teardown(&self) {
        #[cfg(test)]
        self.teardowns.set(self.teardowns.get() + 1);
        self.timers.borrow_mut().clear();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this snapshot, or the last one if it is beyond the end. The range does this while it is still
    /// being moved, so nothing is said until it rests.
    SeekQuiet(usize),
    /// Go to the snapshot that the jump button of this number names.
    Jump(usize),
    Play,
    Pause,
    /// Select this key, which is the one that has the focus.
    Choose(Key),
    /// A key of the keyboard that moves through the tree.
    Navigate(Nav),
    /// The focus has been moved to this key by a person, and not by the page.
    FocusMoved(Key),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the snapshot on the page, who is chosen, and the playback that moves the index.
pub(crate) struct Controller {
    me: Weak<Controller>,
    trace: Rc<Trace>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    /// The jump buttons' destinations, in the order of the buttons.
    jumps: Vec<usize>,
    selected: Cell<usize>,
    view: Cell<View>,
    /// Whether a write failed after the page was touched, so that what is shown is not known.
    dirty: Cell<bool>,
    /// Counted up at every stop, so that a tick from before it does nothing.
    epoch: Cell<u64>,
    tick: Cell<Option<u64>>,
    ticking: Cell<bool>,
    /// Whether the Play button says Pause, which is only so once a tick is waiting and the button has been written.
    shown_playing: Cell<bool>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// Makes the controller for `trace`, shown at its first snapshot, whose page is already drawn.
    pub(crate) fn new(
        trace: Rc<Trace>,
        jumps: Vec<usize>,
        surface: Rc<dyn Surface>,
        scheduler: Rc<dyn Scheduler>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            me: me.clone(),
            trace,
            surface,
            scheduler,
            jumps,
            selected: Cell::new(0),
            view: Cell::new(View::default()),
            dirty: Cell::new(false),
            epoch: Cell::new(0),
            tick: Cell::new(None),
            ticking: Cell::new(false),
            shown_playing: Cell::new(false),
            torn_down: Cell::new(false),
        })
    }

    #[cfg(test)]
    pub(crate) fn index(&self) -> usize {
        self.selected.get()
    }

    pub(crate) fn view(&self) -> View {
        self.view.get()
    }

    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Whether a tick is waiting for the current epoch, which is all that playing is.
    pub(crate) fn is_playing(&self) -> bool {
        self.tick.get().is_some() || self.ticking.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Commits the page again if a write failed, and says whether it is right now. Every action starts with this.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        match self.surface.commit(&self.trace, self.selected.get(), &self.view.get()) {
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

    /// Tells the page that playback stopped. A button that could not be written is left saying the wrong thing, so the
    /// page is dirty, and the next action commits it again before anything else.
    fn show_playing(&self, playing: bool) {
        if let Err(problem) = self.surface.playing_changed(playing) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    /// Asks for the next tick, which is the only one that is ever waiting. It carries the epoch that it was asked for.
    fn schedule_next(&self) -> Result<(), String> {
        debug_assert!(self.tick.get().is_none(), "a tick is already waiting");
        let (epoch, me) = (self.epoch.get(), self.me.clone());
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_tick(epoch);
            }
        });
        self.tick.set(Some(self.scheduler.schedule_once(STEP_MS, task)?));
        Ok(())
    }

    /// Stops playback because no tick could be scheduled, and says so. The page is right, so it is not dirty.
    fn schedule_failed(&self, problem: &str) {
        self.stop_playback();
        self.surface.playback_failed(problem);
    }

    /// Shows the next snapshot, and asks for the tick after it only once the page has been committed. A tick from
    /// before a stop does nothing.
    fn on_tick(&self, epoch: u64) {
        if self.torn_down.get() || epoch != self.epoch.get() {
            return;
        }
        self.tick.set(None);
        self.ticking.set(true);
        let last = self.trace.last();
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

    /// Commits the snapshot at `target` and, once that works, makes it the selected one. A failure leaves the selected
    /// index where it was, stops playback and reports it. If the page was touched it is also marked dirty.
    fn go(&self, target: usize, announce: bool) -> bool {
        match self.surface.commit(&self.trace, target, &self.view.get()) {
            Ok(()) => {
                self.selected.set(target);
                if let Some(snapshot) = self.trace.snapshot(target) {
                    if announce || (self.is_playing() && announced_while_playing(&snapshot.event)) {
                        self.surface.say(&summary(snapshot, self.trace.last()));
                    }
                }
                true
            },
            Err(error) => {
                let (CommitError::Detached(problem) | CommitError::Live(problem)) = &error;
                if matches!(error, CommitError::Live(_)) {
                    self.dirty.set(true);
                }
                self.stop_playback();
                self.surface.failed(problem);
                false
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does what a person has asked for.
    ///
    /// A page that is dirty is committed again first, even if the action then has nowhere to go. If that fails, nothing
    /// else happens. Every action that moves the index stops playback, and so does one that cannot move it.
    pub(crate) fn act(&self, action: Action) {
        if self.torn_down.get() {
            return;
        }
        // The focus moving is a report of what already happened, and must not rebuild the page it happened in.
        if let Action::FocusMoved(key) = action {
            self.focus_moved(key);
            return;
        }
        if !self.repair() {
            return;
        }
        let (at, last) = (self.selected.get(), self.trace.last());
        match action {
            Action::Pause => {
                let was = self.is_playing();
                self.stop_playback();
                if was {
                    self.announce_current();
                }
            },
            Action::Play => self.play(at, last),
            Action::Choose(key) => self.choose(key),
            Action::Navigate(nav) => self.navigate(nav),
            Action::FocusMoved(_) => {},
            Action::Previous => self.move_to(at.checked_sub(1), AT_FIRST, true),
            Action::Restart => self.move_to((at > 0).then_some(0), AT_FIRST, true),
            Action::Next => self.move_to((at < last).then_some(at + 1), AT_LAST, true),
            Action::SeekQuiet(i) => self.move_to(Some(i.min(last)).filter(|&i| i != at), "", false),
            Action::Jump(n) => match self.jumps.get(n).copied() {
                Some(i) => self.move_to(Some(i).filter(|&i| i != at), "", true),
                None => self.stop_playback(),
            },
        }
    }

    /// Stops playback, and goes to `target` if there is one. With nowhere to go it says `nowhere`, if that is not
    /// empty.
    fn move_to(&self, target: Option<usize>, nowhere: &str, announce: bool) {
        self.stop_playback();
        match target {
            Some(t) => {
                self.go(t, announce);
            },
            None if !nowhere.is_empty() => self.surface.say(nowhere),
            None => {},
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

    /// Schedules the first tick, and claims to be playing only once one is waiting and the button says so. If the
    /// button cannot be written, the tick is taken back and playback stays paused.
    fn start_playing(&self) {
        match self.schedule_next() {
            Ok(()) => {
                self.shown_playing.set(true);
                if let Err(problem) = self.surface.playing_changed(true) {
                    // Stopping also writes the button back to Play. If that fails too, the page is dirty.
                    self.stop_playback();
                    self.surface.playback_failed(&problem);
                }
            },
            Err(problem) => self.surface.playback_failed(&problem),
        }
    }

    /// Says where the run is, after a person has moved the range to rest, or paused.
    pub(crate) fn announce_current(&self) {
        if let Some(snapshot) = self.trace.snapshot(self.selected.get()) {
            self.surface.say(&summary(snapshot, self.trace.last()));
        }
    }

    /// The range has been let go. What it rests on is said once, if the page has it.
    pub(crate) fn seek_release(&self) {
        if !self.torn_down.get() && !self.dirty.get() {
            self.announce_current();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows who is selected and where the tab stop is, and marks the page dirty if that could not be written.
    fn project(&self) {
        if let Err(problem) = self.surface.project(&self.trace, self.selected.get(), &self.view.get()) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    /// Selects `key`, the key that has the focus. It is what a person chose, and it stays that until they choose again.
    fn choose(&self, key: Key) {
        let Some(snapshot) = self.trace.snapshot(self.selected.get()) else { return };
        if !snapshot.has_key(key) {
            return;
        }
        self.view.set(View {
            chosen: Some(key),
            roving: Some(key),
        });
        self.project();
        self.surface.say(&format!("Selected {}.", item_name(snapshot, key)));
    }

    /// Moves the tab stop and the focus with a key of the keyboard. A key that goes nowhere does nothing.
    fn navigate(&self, nav: Nav) {
        let Some(snapshot) = self.trace.snapshot(self.selected.get()) else { return };
        let view = self.view.get();
        let Some(from) = resolve_roving(view.roving, snapshot) else { return };
        if let Some(to) = navigate(snapshot, from, nav) {
            self.view.set(View { roving: Some(to), ..view });
            self.project();
            self.surface.focus_key(to);
        }
    }

    /// A person put the focus on `key`. If that is not where the tab stop already is, the tab stop moves there.
    /// Reaching the tree with Tab lands on the tab stop and changes nothing.
    fn focus_moved(&self, key: Key) {
        let Some(snapshot) = self.trace.snapshot(self.selected.get()) else { return };
        let view = self.view.get();
        if resolve_roving(view.roving, snapshot) != Some(key) && snapshot.has_key(key) {
            self.view.set(View { roving: Some(key), ..view });
            self.project();
        }
    }

    /// Stops playback for good, as when the page is left. Nothing that was asked of the controller then has any effect.
    pub(crate) fn teardown(&self) {
        self.stop_playback();
        self.torn_down.set(true);
        self.scheduler.teardown();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
