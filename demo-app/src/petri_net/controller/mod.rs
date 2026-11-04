//! Which snapshot is shown, what moves it, the manual branch, and the playback. Pure: nothing here touches the DOM, so
//! every rule of the buttons, the range, the playback, the branch and the repair of a page that was half written is
//! tested without a browser.
//!
//! The controller owns the index of the snapshot on the page and changes it only after the [`Surface`] has drawn that
//! snapshot. A write that fails part way marks the page dirty, and the first thing that every action does is to draw
//! the committed snapshot again in full, even when the action then has nowhere to go.
//!
//! There is one scene for all three scripts and for the manual branch, so a change of history draws a new snapshot on
//! the same scene. Two counters keep a late callback from acting: the playback epoch, which counts every stop, and the
//! history generation, which counts every change of history.

use super::{
    fixtures::Scenario,
    net::{FireError, Net, TransitionId},
    projection::{
        AT_FIRST, AT_LAST, FIRE_A_PLACE, FIRE_IN_SCRIPT, FIRE_NOTHING_SELECTED, NO_CHECKPOINTS_IN_MANUAL,
        PLAY_IN_MANUAL, Source, Speed, Target, View, Wiring, announced_while_playing, disabled_sentence, pause_summary,
        summary,
    },
    trace::{Branch, MAX_EVENTS, Trace, TraceError},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

mod scheduler;
pub(crate) use scheduler::{ManualScheduler, Scheduler};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the surface draws: one snapshot of one history, as seen with one view.
pub(crate) struct Frame<'a> {
    pub(crate) net: &'a Net,
    pub(crate) trace: &'a Trace,
    pub(crate) index: usize,
    pub(crate) source: Source,
    pub(crate) view: &'a View,
    /// A sentence that stays on the page until the next successful action, such as that a branch was discarded.
    pub(crate) notice: &'a str,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controller draws on and says to a person, which in the page is the DOM.
pub(crate) trait Surface {
    /// Draws `frame`. When `full` is set every element is written. Otherwise only what differs from what was drawn
    /// last.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed. What is on the page is then unknown, and the next draw must be a full one.
    fn draw(&self, frame: &Frame, full: bool) -> Result<(), String>;

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
/// What a person can ask for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this snapshot, or the last one if it is beyond the end.
    Seek(usize),
    /// Go to the named checkpoint at this position in the scenario's list. Scripts only.
    Checkpoint(usize),
    Play,
    Pause,
    /// Select a place or a transition to inspect. This never changes the snapshot, and never fires anything.
    Select(Target),
    SetSpeed(Speed),
    SetWiring(Wiring),
    SelectScenario(Scenario),
    EnterManual,
    ReturnToScript,
    /// Fire the transition that is selected, in manual mode.
    FireSelected,
    /// Select a transition and fire it, as a double click or a key on a focused transition does.
    Fire(TransitionId),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the snapshot on the page, the history it is in, the view, and the playback that moves the index.
pub(crate) struct Controller {
    me: Weak<Controller>,
    net: Net,
    script: RefCell<Rc<Trace>>,
    scenario: Cell<Scenario>,
    /// The snapshot of the script that is shown, or that the manual branch was forked from.
    script_index: Cell<usize>,
    branch: RefCell<Option<Branch>>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    view: Cell<View>,
    /// Whether the page may be out of date, since a write failed part way.
    dirty: Cell<bool>,
    /// The sentence that the page shows until the next successful action, which a repair writes again.
    notice: RefCell<String>,
    /// Counted up at every change of history, and at every firing that cuts off a branch's recorded future.
    history: Cell<u64>,
    /// Counted up at every logical stop of playback, so that a tick that was scheduled before it knows it is stale.
    epoch: Cell<u64>,
    /// The tick that is waiting. There is never more than one.
    tick: Cell<Option<u64>>,
    /// Whether a tick is being worked on, which is not a stop.
    ticking: Cell<bool>,
    /// What the page was last told about playing, so that it is told only of a change.
    shown_playing: Cell<bool>,
    /// The latest place that the range has been moved to and not yet drawn, with the history it was made against.
    pending_seek: Cell<Option<(usize, u64)>>,
    /// The frame that a seek is waiting for: its handle, and the token that its task carries.
    frame: Cell<Option<(u64, u64)>>,
    frame_tokens: Cell<u64>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// A controller that shows `scenario` at snapshot 0. The first snapshot is drawn at once, and if that fails the
    /// page is dirty.
    ///
    /// # Panics
    ///
    /// Never for the fixed scripts, which a test builds.
    pub(crate) fn new(
        net: Net,
        scenario: Scenario,
        view: View,
        surface: Rc<dyn Surface>,
        scheduler: Rc<dyn Scheduler>,
    ) -> Rc<Self> {
        let script = scenario.trace(&net).expect("a fixed script is valid");
        let controller = Rc::new_cyclic(|me| Self {
            me: me.clone(),
            net,
            script: RefCell::new(Rc::new(script)),
            scenario: Cell::new(scenario),
            script_index: Cell::new(0),
            branch: RefCell::new(None),
            surface,
            scheduler,
            view: Cell::new(view),
            dirty: Cell::new(false),
            notice: RefCell::new(String::new()),
            history: Cell::new(0),
            epoch: Cell::new(0),
            tick: Cell::new(None),
            ticking: Cell::new(false),
            shown_playing: Cell::new(false),
            pending_seek: Cell::new(None),
            frame: Cell::new(None),
            frame_tokens: Cell::new(0),
            torn_down: Cell::new(false),
        });
        if let Err(problem) = controller.paint(controller.cursor(), &view, true, "") {
            controller.dirty.set(true);
            controller.surface.failed(&problem);
        }
        controller
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn net(&self) -> &Net {
        &self.net
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn view(&self) -> View {
        self.view.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn scenario(&self) -> Scenario {
        self.scenario.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn is_manual(&self) -> bool {
        self.branch.borrow().is_some()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The history that is shown, which is the manual branch when there is one.
    pub(crate) fn source(&self) -> Source {
        match &*self.branch.borrow() {
            Some(_) => Source::Manual {
                origin: self.scenario.get(),
                fork: self.script_index.get(),
            },
            None => Source::Script(self.scenario.get()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The index of the snapshot on the page, in the coordinates of the history that is shown.
    pub(crate) fn cursor(&self) -> usize {
        match &*self.branch.borrow() {
            Some(branch) => branch.cursor(),
            None => self.script_index.get(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The index of the last snapshot of the history that is shown.
    pub(crate) fn last(&self) -> usize {
        self.with_trace(|trace| trace.last())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn script_index(&self) -> usize {
        self.script_index.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn history_generation(&self) -> u64 {
        self.history.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(crate) fn playback_epoch(&self) -> u64 {
        self.epoch.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a tick is waiting for the current epoch, which is all that playing is.
    pub(crate) fn is_playing(&self) -> bool {
        self.tick.get().is_some() || self.ticking.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn with_trace<R>(&self, f: impl FnOnce(&Trace) -> R) -> R {
        match &*self.branch.borrow() {
            Some(branch) => f(branch.trace()),
            None => f(&self.script.borrow()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the snapshot at `index` of the history that is shown, with `view` and `notice`.
    fn paint(&self, index: usize, view: &View, full: bool, notice: &str) -> Result<(), String> {
        let source = self.source();
        self.with_trace(|trace| {
            self.surface.draw(
                &Frame {
                    net: &self.net,
                    trace,
                    index,
                    source,
                    view,
                    notice,
                },
                full,
            )
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the page again in full if a write failed, and says whether it is right now. Every action starts with this.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        let notice = self.notice.borrow().clone();
        match self.paint(self.cursor(), &self.view.get(), true, &notice) {
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn show_playing(&self, playing: bool) {
        if let Err(problem) = self.surface.playing_changed(playing) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Asks for the next tick, which is the only one that is ever waiting. It carries the epoch and the history that it
    /// was asked for.
    fn schedule_next(&self) -> Result<(), String> {
        debug_assert!(self.tick.get().is_none(), "a tick is already waiting");
        let (epoch, history, me) = (self.epoch.get(), self.history.get(), self.me.clone());
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_tick(epoch, history);
            }
        });
        let delay = self.view.get().speed.interval_ms();
        self.tick.set(Some(self.scheduler.schedule_once(delay, task)?));
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn schedule_failed(&self, problem: &str) {
        self.stop_playback();
        self.surface.playback_failed(problem);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows the next snapshot, and asks for the tick after it only once the page has been written. A tick from before
    /// a stop, or from another history, does nothing.
    fn on_tick(&self, epoch: u64, history: u64) {
        if self.torn_down.get() || epoch != self.epoch.get() || history != self.history.get() {
            return;
        }
        self.tick.set(None);
        self.ticking.set(true);
        let last = self.last();
        let target = (self.cursor() + 1).min(last);
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Records `index` as the snapshot on the page. The manual branch keeps its own, and the script keeps its own.
    fn set_cursor(&self, index: usize) {
        match &mut *self.branch.borrow_mut() {
            Some(branch) => branch.set_cursor(index),
            None => self.script_index.set(index),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the snapshot at `target` and, once that works, makes it the one on the page. A failure leaves the index
    /// where it was, marks the page dirty, stops playback and reports it.
    fn go(&self, target: usize, announce: bool) -> bool {
        match self.paint(target, &self.view.get(), false, "") {
            Ok(()) => {
                self.notice.borrow_mut().clear();
                self.set_cursor(target);
                let loud = announce || (self.is_playing() && self.with_trace(|t| announced_while_playing(t, target)));
                if loud {
                    let text = self.with_trace(|trace| summary(&self.net, trace, target));
                    self.surface.say(&text);
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Commits a change of view the way an index is committed: only after the page has been drawn with it.
    ///
    /// Says whether it was committed, which a caller that goes on to act on the new view must check.
    fn commit_view(&self, view: View) -> bool {
        match self.paint(self.cursor(), &view, false, &self.notice.borrow().clone()) {
            Ok(()) => {
                self.view.set(view);
                true
            },
            Err(problem) => {
                // A tick that was waiting would draw the next snapshot onto a page that is out of date, and a tick does
                // not repair it first, so playback stops here.
                self.dirty.set(true);
                self.stop_playback();
                self.surface.failed(&problem);
                false
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
        if action == Action::Pause {
            if self.is_playing() {
                self.stop_playback();
                let text = self.with_trace(|trace| pause_summary(&self.net, trace, self.cursor()));
                self.surface.say(&text);
            }
            return;
        }
        if !self.repair() {
            self.stop_playback();
            return;
        }
        let (at, last) = (self.cursor(), self.last());
        match action {
            Action::Pause => {},
            Action::Play => self.play(at, last),
            Action::Select(target) => {
                let mut view = self.view.get();
                view.selected = Some(target);
                view.roving = target;
                self.commit_view(view);
            },
            Action::SetWiring(wiring) => {
                let mut view = self.view.get();
                view.wiring = wiring;
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
            Action::SelectScenario(scenario) => self.select_scenario(scenario),
            Action::EnterManual => self.enter_manual(),
            Action::ReturnToScript => self.return_to_script(),
            Action::FireSelected => self.fire_selected(),
            Action::Fire(t) => {
                self.stop_playback();
                let mut view = self.view.get();
                view.selected = Some(Target::Transition(t));
                view.roving = Target::Transition(t);
                // Only a selection that the page has drawn is fired. If the draw failed, the page is dirty, and the
                // next action repairs it before anything else happens.
                if self.commit_view(view) {
                    self.fire_selected();
                }
            },
            navigation => {
                self.stop_playback();
                let (target, nowhere) = match navigation {
                    Action::Previous => (at.checked_sub(1), AT_FIRST),
                    Action::Next => ((at < last).then_some(at + 1), AT_LAST),
                    Action::Restart => ((at > 0).then_some(0), AT_FIRST),
                    Action::Seek(index) => (Some(index.min(last)), ""),
                    Action::Checkpoint(n) => {
                        if self.is_manual() {
                            self.surface.say(NO_CHECKPOINTS_IN_MANUAL);
                            return;
                        }
                        match self.scenario.get().checkpoints().get(n) {
                            Some(checkpoint) => (Some(checkpoint.snapshot.min(last)), ""),
                            None => return,
                        }
                    },
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts playing if there is further to go, and says so if there is not.
    fn play(&self, at: usize, last: usize) {
        if self.is_playing() {
            return;
        }
        if self.is_manual() {
            self.surface.say(PLAY_IN_MANUAL);
            return;
        }
        if at >= last {
            self.surface.say(AT_LAST);
            return;
        }
        self.start_playing();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Schedules the first tick, and claims to be playing only once one is waiting and the button says so. If the
    /// button cannot be written, the tick is taken back and playback stays paused.
    fn start_playing(&self) {
        match self.schedule_next() {
            Ok(()) => {
                self.shown_playing.set(true);
                if let Err(problem) = self.surface.playing_changed(true) {
                    self.stop_playback();
                    self.surface.playback_failed(&problem);
                }
            },
            Err(problem) => self.surface.playback_failed(&problem),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Says where the run is, after a person has moved the range to rest.
    pub(crate) fn announce_current(&self) {
        let text = self.with_trace(|trace| summary(&self.net, trace, self.cursor()));
        self.surface.say(&text);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts a fresh branch from the snapshot of the script that is shown. Playback stops first.
    fn enter_manual(&self) {
        self.stop_playback();
        if self.is_manual() {
            self.surface.say("Already in manual mode.");
            return;
        }
        let fork = self.script_index.get();
        let Some(source) = self.script.borrow().snapshot(fork).copied() else {
            return;
        };
        *self.branch.borrow_mut() = Some(Branch::fork(&source));
        match self.paint(0, &self.view.get(), false, "") {
            Ok(()) => {
                self.notice.borrow_mut().clear();
                self.bump_history();
                self.surface.say(&format!(
                    "Manual mode. Forked from {} at snapshot {fork}. Select an enabled transition and fire it.",
                    self.scenario.get().label()
                ));
            },
            Err(problem) => {
                // Put the script back, and let the next action draw the page again.
                *self.branch.borrow_mut() = None;
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Goes back to the script at the snapshot the branch was forked from, and discards the branch.
    fn return_to_script(&self) {
        self.stop_playback();
        let Some(branch) = self.branch.borrow_mut().take() else {
            self.surface.say("Already showing the script.");
            return;
        };
        match self.paint(self.script_index.get(), &self.view.get(), false, "") {
            Ok(()) => {
                self.notice.borrow_mut().clear();
                self.bump_history();
                self.surface.say(&format!(
                    "Returned to {} at snapshot {}.",
                    self.scenario.get().label(),
                    self.script_index.get()
                ));
            },
            Err(problem) => {
                *self.branch.borrow_mut() = Some(branch);
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows `scenario` at snapshot 0 on the same scene, and discards a manual branch. A failure leaves the scenario,
    /// the branch and the index as they were.
    fn select_scenario(&self, scenario: Scenario) {
        self.stop_playback();
        self.cancel_pending_seek();
        let trace = match scenario.trace(&self.net) {
            Ok(trace) => trace,
            Err(problem) => {
                self.surface.failed(&problem.to_string());
                return;
            },
        };
        let was_manual = self.is_manual();
        let notice = if was_manual {
            format!("Manual branch discarded. {} loaded at snapshot 0.", scenario.label())
        } else {
            String::new()
        };
        // The new history is made live for the draw, and put back if the draw fails.
        let old = (
            std::mem::replace(&mut *self.script.borrow_mut(), Rc::new(trace)),
            self.scenario.replace(scenario),
            self.script_index.replace(0),
            self.branch.borrow_mut().take(),
        );
        match self.paint(0, &self.view.get(), false, &notice) {
            Ok(()) => {
                *self.notice.borrow_mut() = notice.clone();
                self.bump_history();
                self.surface.say(&if was_manual {
                    notice
                } else {
                    format!("Showing {}, at snapshot 0.", scenario.label())
                });
            },
            Err(problem) => {
                *self.script.borrow_mut() = old.0;
                self.scenario.set(old.1);
                self.script_index.set(old.2);
                *self.branch.borrow_mut() = old.3;
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Fires the selected transition in the manual branch, at the snapshot on the page.
    ///
    /// The candidate history is drawn before it is kept. A transition that is not enabled, a place, and a script all
    /// say why nothing happened and change nothing.
    fn fire_selected(&self) {
        self.stop_playback();
        if !self.is_manual() {
            self.surface.say(FIRE_IN_SCRIPT);
            return;
        }
        let t = match self.view.get().selected {
            None => {
                self.surface.say(FIRE_NOTHING_SELECTED);
                return;
            },
            Some(Target::Place(_)) => {
                self.surface.say(FIRE_A_PLACE);
                return;
            },
            Some(Target::Transition(t)) => t,
        };
        let candidate = {
            let branch = self.branch.borrow();
            let Some(branch) = branch.as_ref() else {
                return;
            };
            match branch.candidate(&self.net, t) {
                Ok(candidate) => candidate,
                Err(TraceError::Fire(FireError::Disabled(_))) => {
                    let text = branch
                        .trace()
                        .snapshot(branch.cursor())
                        .map(|s| disabled_sentence(&self.net, s, t));
                    self.surface.say(&text.unwrap_or_default());
                    return;
                },
                Err(TraceError::Budget) => {
                    self.surface.say(&format!(
                        "The manual branch is full at {MAX_EVENTS} events. Return to the script to start again."
                    ));
                    return;
                },
                Err(other) => {
                    self.surface.failed(&other.to_string());
                    return;
                },
            }
        };
        let cuts_future = self.branch.borrow().as_ref().is_some_and(|b| b.cursor() < b.trace().last());
        let index = candidate.cursor();
        let source = self.source();
        let drawn = self.surface.draw(
            &Frame {
                net: &self.net,
                trace: candidate.trace(),
                index,
                source,
                view: &self.view.get(),
                notice: "",
            },
            false,
        );
        match drawn {
            Ok(()) => {
                let text = summary(&self.net, candidate.trace(), index);
                *self.branch.borrow_mut() = Some(candidate);
                self.notice.borrow_mut().clear();
                if cuts_future {
                    self.bump_history();
                }
                self.surface.say(&text);
            },
            Err(problem) => {
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Counts a change of history, so that every tick and every seek that was asked for before it does nothing.
    fn bump_history(&self) {
        self.history.set(self.history.get() + 1);
        self.cancel_pending_seek();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The range has been moved to `index`. Playback stops at once, and the latest place is drawn at the next frame, so
    /// moving it many times between two frames draws once.
    pub(crate) fn seek_input(&self, index: usize) {
        if self.torn_down.get() {
            return;
        }
        self.stop_playback();
        let first = self.pending_seek.replace(Some((index, self.history.get()))).is_none();
        if !first {
            return;
        }
        let (history, me) = (self.history.get(), self.me.clone());
        let token = self.frame_tokens.get() + 1;
        self.frame_tokens.set(token);
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_frame(history, token);
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the place that was asked for, if this is the frame that is waiting. A frame that was taken back, or that
    /// is for another history, does nothing, even if it arrives while a newer frame is waiting.
    fn on_frame(&self, history: u64, token: u64) {
        let waiting = self.frame.get().map(|(_, token)| token);
        if self.torn_down.get() || history != self.history.get() || waiting != Some(token) {
            return;
        }
        self.frame.set(None);
        self.flush_seek(None);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the place that the range was moved to, if it is for the history that is shown. A request for another
    /// history is dropped, and `fallback` is the place that is drawn in its stead, if there is one.
    fn flush_seek(&self, fallback: Option<usize>) {
        let index = match self.pending_seek.take() {
            Some((index, history)) if history == self.history.get() => Some(index),
            _ => fallback,
        };
        if let Some(index) = index {
            self.act(Action::Seek(index));
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn cancel_pending_seek(&self) {
        self.pending_seek.set(None);
        if let Some((handle, _)) = self.frame.take() {
            self.scheduler.cancel_frame(handle);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
