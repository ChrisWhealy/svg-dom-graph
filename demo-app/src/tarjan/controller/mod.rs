//! Which step is shown, what moves it, what moves the source view with it, and the playback. Pure: nothing here touches
//! the DOM, so every rule of the buttons, the range, the playback, the follow and the repair of a page that was half
//! written is tested without a browser.
//!
//! The controller owns the index of the step on the page, and changes it only after the [`Surface`] has drawn that
//! step, and any pan that goes with it. A write that fails part way marks the page dirty, and the first thing that
//! every action does is to draw the committed step again in full, even when the action then has nowhere to go.
//!
//! The view is the person's own to move. Before any decision to move it, the controller compares the view that is on
//! the page with the one that it last wrote. If they differ, a person changed it, and following is turned off.

use super::{
    fixture::name,
    layout::Layout,
    projection::{
        AT_FIRST, AT_LAST, Choices, NO_EARLIER_COMPONENT, NO_LATER_COMPONENT, Speed, Target, announced_while_playing,
        pause_summary, present,
    },
    solver::{Graph, Trace},
    view::{self, Camera, Memory, Plan, Reconciled, Visibility, visibility},
};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

mod scheduler;
pub(crate) use scheduler::{ManualScheduler, Scheduler};

pub(crate) const FOLLOW_OFF: &str = "Follow active vertex off";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the surface draws: one step, as seen with one choice of what to look at and one view.
pub(crate) struct Frame<'a> {
    pub(crate) graph: &'a Graph,
    pub(crate) edges: &'a [(usize, usize)],
    pub(crate) trace: &'a Trace,
    pub(crate) layout: &'a Layout,
    pub(crate) index: usize,
    pub(crate) choices: &'a Choices,
    /// The view that the step is shown with, which is the one that the page has, or the one that is about to be
    /// written.
    pub(crate) camera: Camera,
    pub(crate) viewport: (f64, f64),
}

/// What the controller draws on and says to a person, which in the page is the DOM and the scene.
pub(crate) trait Surface {
    /// Draws `frame`. When `full` is set every element is written. Otherwise only what differs from what was drawn
    /// last.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write failed. What is on the page is then unknown, and the next draw must be a full one.
    fn draw(&self, frame: &Frame, full: bool) -> Result<(), String>;

    /// The view that the scene has now.
    fn camera(&self) -> Camera;

    /// The size of the source viewport, in screen pixels.
    fn viewport(&self) -> (f64, f64);

    /// Writes the source view.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the view could not be written, in which case it is as it was.
    fn write_camera(&self, camera: Camera) -> Result<(), String>;

    /// Says that playback started or stopped, and writes it on the Play button.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be written.
    fn playing_changed(&self, playing: bool) -> Result<(), String>;

    /// Tells a person something, in the polite live region.
    fn say(&self, text: &str);

    /// Reports a write that failed.
    fn failed(&self, problem: &str);

    /// Reports that playback could not carry on, because no tick could be scheduled. The page is right.
    fn playback_failed(&self, problem: &str);

    /// Shows a warning that does not stop anything.
    fn warn(&self, text: &str);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a person can ask for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Previous,
    Next,
    Restart,
    /// Go straight to this step, or the last one if it is beyond the end. The view stays where it is.
    Seek(usize),
    PreviousComponent,
    NextComponent,
    JumpFinal,
    Play,
    Pause,
    /// Select a vertex or an edge. This never changes the step.
    Select(Target),
    /// Pick out a component, or let go of it.
    SelectComponent(Option<usize>),
    SetSpeed(Speed),
    SetFollow(bool),
    /// Fit the region of the attention vertex.
    Fit,
}

/// What moved the step, which decides what happens to the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    /// A Play tick or a single Next or Previous, which follow when following is on.
    Follow,
    /// A jump to a closure, which centres it.
    Landmark,
    /// A slider seek or a restart, which leave the view alone.
    Stationary,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the step on the page, the view, and the playback that moves the index.
pub(crate) struct Controller {
    me: Weak<Controller>,
    graph: Rc<Graph>,
    edges: Vec<(usize, usize)>,
    trace: Rc<Trace>,
    layout: Rc<Layout>,
    surface: Rc<dyn Surface>,
    scheduler: Rc<dyn Scheduler>,
    selected: Cell<usize>,
    choices: Cell<Choices>,
    memory: Cell<Memory>,
    /// Whether a person's pan or zoom is not yet settled, so that the view is not to be moved under their hand.
    gesture: Cell<bool>,
    /// How well the attention vertex could last be seen, so that a change is said once.
    last_visibility: Cell<Option<Visibility>>,
    /// Whether the page may be out of date, since a write failed part way.
    dirty: Cell<bool>,
    /// Counted up at every logical stop of playback, so that a tick that was scheduled before it knows it is stale.
    epoch: Cell<u64>,
    tick: Cell<Option<u64>>,
    ticking: Cell<bool>,
    shown_playing: Cell<bool>,
    pending_seek: Cell<Option<usize>>,
    frame: Cell<Option<(u64, u64)>>,
    frame_tokens: Cell<u64>,
    torn_down: Cell<bool>,
}

impl Controller {
    /// A controller at step 0. The first step is drawn at once, and if that fails the page is dirty.
    pub(crate) fn new(
        graph: Graph,
        trace: Trace,
        layout: Layout,
        surface: Rc<dyn Surface>,
        scheduler: Rc<dyn Scheduler>,
    ) -> Rc<Self> {
        let camera = surface.camera();
        let controller = Rc::new_cyclic(|me| Self {
            me: me.clone(),
            edges: graph.edges(),
            graph: Rc::new(graph),
            trace: Rc::new(trace),
            layout: Rc::new(layout),
            surface,
            scheduler,
            selected: Cell::new(0),
            choices: Cell::new(Choices::first()),
            memory: Cell::new(Memory::new(camera)),
            gesture: Cell::new(false),
            last_visibility: Cell::new(None),
            dirty: Cell::new(false),
            epoch: Cell::new(0),
            tick: Cell::new(None),
            ticking: Cell::new(false),
            shown_playing: Cell::new(false),
            pending_seek: Cell::new(None),
            frame: Cell::new(None),
            frame_tokens: Cell::new(0),
            torn_down: Cell::new(false),
        });
        let choices = controller.choices.get();
        if let Err(problem) = controller.paint(0, &choices, camera, true) {
            controller.dirty.set(true);
            controller.surface.failed(&problem);
        }
        controller.last_visibility.set(controller.visibility_at(0, camera));
        controller
    }

    #[cfg(test)]
    pub(crate) fn trace(&self) -> Rc<Trace> {
        Rc::clone(&self.trace)
    }

    pub(crate) fn choices(&self) -> Choices {
        self.choices.get()
    }

    #[cfg(test)]
    pub(crate) fn index(&self) -> usize {
        self.selected.get()
    }

    #[cfg(test)]
    pub(crate) fn memory(&self) -> Memory {
        self.memory.get()
    }

    #[cfg(test)]
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    #[cfg(test)]
    pub(crate) fn playback_epoch(&self) -> u64 {
        self.epoch.get()
    }

    /// Whether a tick is waiting for the current epoch, which is all that playing is.
    pub(crate) fn is_playing(&self) -> bool {
        self.tick.get().is_some() || self.ticking.get()
    }

    /// How well the attention vertex of step `index` can be seen with `camera`.
    fn visibility_at(&self, index: usize, camera: Camera) -> Option<Visibility> {
        let v = self.trace.snapshot(index)?.attention()?;
        Some(visibility(&camera, self.surface.viewport(), &self.layout, v))
    }

    /// Draws step `index` with these choices and this view.
    fn paint(&self, index: usize, choices: &Choices, camera: Camera, full: bool) -> Result<(), String> {
        self.surface.draw(
            &Frame {
                graph: &self.graph,
                edges: &self.edges,
                trace: &self.trace,
                layout: &self.layout,
                index,
                choices,
                camera,
                viewport: self.surface.viewport(),
            },
            full,
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the page again in full if a write failed, and says whether it is right now. Every action starts with this.
    fn repair(&self) -> bool {
        if !self.dirty.get() {
            return true;
        }
        match self.paint(self.selected.get(), &self.choices.get(), self.surface.camera(), true) {
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

    fn stop_playback(&self) {
        self.epoch.set(self.epoch.get() + 1);
        if let Some(handle) = self.tick.take() {
            self.scheduler.cancel(handle);
        }
        if self.shown_playing.replace(false) {
            self.show_playing(false);
        }
    }

    fn show_playing(&self, playing: bool) {
        if let Err(problem) = self.surface.playing_changed(playing) {
            self.dirty.set(true);
            self.surface.failed(&problem);
        }
    }

    fn schedule_next(&self) -> Result<(), String> {
        debug_assert!(self.tick.get().is_none(), "a tick is already waiting");
        let (epoch, me) = (self.epoch.get(), self.me.clone());
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_tick(epoch);
            }
        });
        let delay = self.choices.get().speed.interval_ms();
        self.tick.set(Some(self.scheduler.schedule_once(delay, task)?));
        Ok(())
    }

    fn schedule_failed(&self, problem: &str) {
        self.stop_playback();
        self.surface.playback_failed(problem);
    }

    /// Shows the next step, and asks for the tick after it only once the page has been written. A tick from before a
    /// stop does nothing.
    fn on_tick(&self, epoch: u64) {
        if self.torn_down.get() || epoch != self.epoch.get() {
            return;
        }
        self.tick.set(None);
        self.ticking.set(true);
        let last = self.trace.last();
        let target = (self.selected.get() + 1).min(last);
        let moved = self.go(target, Move::Follow, false);
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
    /// Compares the view on the page with the one that was last written. If a person changed it, following is turned
    /// off and the page is drawn again with the view, which only changes the words about what can be seen.
    fn reconcile(&self) {
        let mut memory = self.memory.get();
        let outcome = memory.reconcile(self.surface.camera());
        if outcome == Reconciled::Unchanged {
            return;
        }
        self.memory.set(memory);
        let mut choices = self.choices.get();
        if outcome == Reconciled::FollowTurnedOff {
            choices.follow = false;
        }
        let camera = self.surface.camera();
        match self.paint(self.selected.get(), &choices, camera, false) {
            Ok(()) => {
                self.choices.set(choices);
                if outcome == Reconciled::FollowTurnedOff {
                    self.surface.say(FOLLOW_OFF);
                }
                self.announce_visibility(camera);
            },
            Err(problem) => {
                self.dirty.set(true);
                self.surface.failed(&problem);
            },
        }
    }

    /// Says how well the attention vertex can be seen, but only when that has changed and it is not well in view.
    fn announce_visibility(&self, camera: Camera) {
        let now = self.visibility_at(self.selected.get(), camera);
        if self.last_visibility.replace(now) != now {
            if let (Some(v), Some(state)) = (self.trace.snapshot(self.selected.get()).and_then(|s| s.attention()), now)
            {
                let text = super::projection::view_notice(Some(state), Some(v));
                if !text.is_empty() {
                    self.surface.say(&text);
                }
            }
        }
    }

    /// A person's gesture has begun or has ended. While one is unsettled the controller does not move the view.
    pub(crate) fn gesture(&self, unsettled: bool) {
        if self.torn_down.get() {
            return;
        }
        self.gesture.set(unsettled);
        if !unsettled {
            self.reconcile();
        }
    }

    /// Looks at the view after something that may have changed it, such as a wheel turn or a key.
    pub(crate) fn observe_view(&self) {
        if !self.torn_down.get() {
            self.reconcile();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves to step `target`, with whatever the move does to the view, as one transaction. A failure at any point
    /// leaves the index where it was, puts the view back as far as it can, marks the page dirty, stops playback and
    /// reports it.
    fn go(&self, target: usize, how: Move, announce: bool) -> bool {
        self.reconcile();
        let Some(snapshot) = self.trace.snapshot(target) else {
            return false;
        };
        let saved = self.surface.camera();
        let viewport = self.surface.viewport();
        let attention = snapshot.attention();
        let plan: Option<Plan> = if self.gesture.get() {
            None
        } else {
            match how {
                Move::Follow if self.choices.get().follow => view::follow(&saved, viewport, &self.layout, attention),
                Move::Landmark => snapshot
                    .components
                    .last()
                    .and_then(|members| view::destination(&saved, viewport, &self.layout, members)),
                _ => None,
            }
        };
        let camera = plan.map_or(saved, |p| p.camera);
        // A selected component that is not emitted at the target step is let go of in the same transaction.
        let mut choices = self.choices.get();
        let cleared = choices.component.filter(|&c| c > snapshot.components.len());
        if cleared.is_some() {
            choices.component = None;
        }
        if let Err(problem) = self.paint(target, &choices, camera, false) {
            self.fail(&problem);
            return false;
        }
        if let Some(plan) = plan {
            if !plan.camera.same_as(&saved) {
                if let Err(problem) = self.surface.write_camera(plan.camera) {
                    // The view is as it was, and the page was drawn for a step that is not published.
                    let _ = self.surface.write_camera(saved);
                    self.fail(&problem);
                    return false;
                }
            }
        }
        self.selected.set(target);
        self.choices.set(choices);
        if let Some(plan) = plan {
            let mut memory = self.memory.get();
            memory.committed(plan);
            self.memory.set(memory);
        }
        let loud = announce || (self.is_playing() && announced_while_playing(&self.trace, target));
        if loud {
            if let Some(p) = present(&self.context(camera), target, &choices) {
                self.surface.say(&p.status);
            }
        }
        if let Some(c) = cleared {
            self.surface.say(&format!(
                "Component C{c} does not exist at this step, so it is no longer selected."
            ));
        }
        self.announce_visibility(camera);
        true
    }

    fn fail(&self, problem: &str) {
        self.dirty.set(true);
        self.stop_playback();
        self.surface.failed(problem);
    }

    fn context(&self, camera: Camera) -> super::projection::Context<'_> {
        super::projection::Context {
            graph: &self.graph,
            edges: &self.edges,
            trace: &self.trace,
            layout: &self.layout,
            camera,
            viewport: self.surface.viewport(),
        }
    }

    /// Commits a change of what is looked at the way an index is committed: only after the page has been drawn with it.
    fn commit_choices(&self, choices: Choices) -> bool {
        match self.paint(self.selected.get(), &choices, self.surface.camera(), false) {
            Ok(()) => {
                self.choices.set(choices);
                true
            },
            Err(problem) => {
                self.fail(&problem);
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
                self.surface.say(&pause_summary(&self.trace, self.selected.get()));
            }
            return;
        }
        if !self.repair() {
            self.stop_playback();
            return;
        }
        let (at, last) = (self.selected.get(), self.trace.last());
        match action {
            Action::Pause => {},
            Action::Play => self.play(at, last),
            Action::Select(target) => {
                let mut choices = self.choices.get();
                choices.selected = Some(target);
                choices.roving = target;
                self.commit_choices(choices);
            },
            Action::SelectComponent(component) => {
                let mut choices = self.choices.get();
                let emitted = self.trace.snapshot(at).map_or(0, |s| s.components.len());
                choices.component = component.filter(|&c| c >= 1 && c <= emitted);
                self.commit_choices(choices);
            },
            Action::SetSpeed(speed) => {
                let mut choices = self.choices.get();
                choices.speed = speed;
                self.choices.set(choices);
                if self.is_playing() {
                    // The old tick is stale, and the new one has the new interval.
                    self.stop_playback();
                    self.start_playing();
                }
            },
            Action::SetFollow(on) => {
                let mut choices = self.choices.get();
                choices.follow = on;
                if self.commit_choices(choices) {
                    let mut memory = self.memory.get();
                    memory.follow = on;
                    self.memory.set(memory);
                }
            },
            Action::Fit => self.fit(),
            navigation => {
                self.stop_playback();
                let (target, how, nowhere) = match navigation {
                    Action::Previous => (at.checked_sub(1), Move::Follow, AT_FIRST),
                    Action::Next => ((at < last).then_some(at + 1), Move::Follow, AT_LAST),
                    Action::Restart => ((at > 0).then_some(0), Move::Stationary, AT_FIRST),
                    Action::Seek(index) => (Some(index.min(last)), Move::Stationary, ""),
                    Action::PreviousComponent => {
                        (self.trace.previous_closure(at), Move::Landmark, NO_EARLIER_COMPONENT)
                    },
                    Action::NextComponent => (self.trace.next_closure(at), Move::Landmark, NO_LATER_COMPONENT),
                    Action::JumpFinal => ((at < last).then_some(last), Move::Landmark, AT_LAST),
                    _ => return,
                };
                match target {
                    Some(target) if target != at => {
                        self.go(target, how, !matches!(navigation, Action::Seek(_)));
                    },
                    _ if !nowhere.is_empty() => self.surface.say(nowhere),
                    _ => {},
                }
            },
        }
    }

    /// Zooms to the attention vertex, which is put in the middle at a scale that is easy to read.
    fn fit(&self) {
        let Some(v) = self.trace.snapshot(self.selected.get()).and_then(|s| s.attention()) else {
            return;
        };
        let plan = view::zoom_to(self.surface.viewport(), &self.layout, v);
        self.reconcile();
        let saved = self.surface.camera();
        let choices = self.choices.get();
        if let Err(problem) = self.paint(self.selected.get(), &choices, plan.camera, false) {
            self.fail(&problem);
            return;
        }
        if let Err(problem) = self.surface.write_camera(plan.camera) {
            let _ = self.surface.write_camera(saved);
            self.fail(&problem);
            return;
        }
        let mut memory = self.memory.get();
        memory.committed(plan);
        self.memory.set(memory);
        self.announce_visibility(plan.camera);
        self.surface.say(&format!("Zoomed to {}.", name(v)));
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

    /// Says where the run is, after a person has moved the range to rest.
    pub(crate) fn announce_current(&self) {
        if let Some(p) = present(&self.context(self.surface.camera()), self.selected.get(), &self.choices.get()) {
            self.surface.say(&p.status);
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
        let first = self.pending_seek.replace(Some(index)).is_none();
        if !first {
            return;
        }
        let me = self.me.clone();
        let token = self.frame_tokens.get() + 1;
        self.frame_tokens.set(token);
        let task: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(controller) = me.upgrade() {
                controller.on_frame(token);
            }
        });
        match self.scheduler.request_frame(task) {
            Ok(handle) => self.frame.set(Some((handle, token))),
            Err(problem) => {
                self.surface.warn(&format!("Could not ask for a frame: {problem}"));
                self.flush_seek(None);
            },
        }
    }

    /// Draws the place that was asked for, if this is the frame that is waiting. A frame that was taken back does
    /// nothing, even if it arrives while a newer frame is waiting.
    fn on_frame(&self, token: u64) {
        let waiting = self.frame.get().map(|(_, token)| token);
        if self.torn_down.get() || waiting != Some(token) {
            return;
        }
        self.frame.set(None);
        self.flush_seek(None);
    }

    fn flush_seek(&self, fallback: Option<usize>) {
        if let Some(index) = self.pending_seek.take().or(fallback) {
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

    fn cancel_pending_seek(&self) {
        self.pending_seek.set(None);
        if let Some((handle, _)) = self.frame.take() {
            self.scheduler.cancel_frame(handle);
        }
    }

    /// Stops playback for good, as when the page is left. Nothing that was asked of the controller then has any effect.
    pub(crate) fn teardown(&self) {
        self.stop_playback();
        self.cancel_pending_seek();
        self.gesture.set(false);
        self.torn_down.set(true);
        self.scheduler.teardown();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
