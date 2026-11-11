//! The controller: which slice is on the page, what moves it, and what keeps a late callback from acting.
//!
//! The controller owns a committed cursor and changes it only after the [`Host`] has drawn that slice and was still
//! there when it had (design section 6). It holds no browser, so every rule of playback, seeking, hiding, repair and
//! cancellation is tested natively with a fake host and a [`ManualScheduler`](crate::scheduler::ManualScheduler).
//!
//! Navigation has one owner at a time. A manual request (next, previous, restart, seek, retry or play) first cancels
//! playback and any pending seek, even if it ends at a boundary, so that the last request a person made is the one that
//! wins. A pause stops playback only, and is not a navigation.
//!
//! Three counters keep a stale callback inert. The playback epoch counts every stop of playback, the seek token counts
//! every replacement of a seek, and the scenario generation counts every change of history. A callback carries the
//! values that it was scheduled under and holds only a weak reference to the controller, and it does nothing if any of
//! them has moved, if the host is hidden or detached, or if the controller has been disposed. It is not promised that
//! no callback runs while the panel is hidden. It is promised that one that does is inert.

use crate::projection::{View, project};
use crate::scheduler::Scheduler;
use service_sim_engine::{Engine, Error, Model};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

pub use crate::projection::Projection;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes a fresh engine at the first slice of a scenario, as often as it is asked.
pub type Factory = Rc<dyn Fn() -> Result<Engine, Error>>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the host can be drawn on now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Connected to the document, and the panel that holds it is shown.
    Live,
    /// Connected, and the panel is hidden or the document is.
    Hidden,
    /// Not in the document. A detached element still accepts writes, so this is checked and not discovered.
    Detached,
    /// Gone for good.
    Disposed,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a draw did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawFailure {
    /// The host was not in the document when the draw looked.
    Detached,
    /// A write failed, or the projection has the wrong shape. The page may be partly written.
    Failed(String),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controller draws on and says to a person, which in a page is the DOM.
pub trait Host {
    /// Whether a draw is allowed now: a synchronous test of the connection and of the panel's visibility, which the
    /// controller makes at the entry of every callback, before it schedules, and immediately before and after every
    /// draw.
    fn presence(&self) -> Presence;

    /// Draws `projection`. With `full` every field is written, and otherwise what differs from the last draw, as the
    /// host knows it.
    ///
    /// # Errors
    ///
    /// Returns a [`DrawFailure`] if a write failed. What is on the page is then unknown, and the next draw has to be a
    /// full one.
    fn draw(&mut self, projection: &Projection, full: bool) -> Result<(), DrawFailure>;

    /// Tells a person something, in a polite live region.
    fn announce(&mut self, text: &str);

    /// Reports a failure, once for each episode.
    fn report(&mut self, text: &str);

    /// Says that playback started or stopped, for the button that shows it.
    fn playing_changed(&mut self, playing: bool);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a request came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The slice that was asked for is on the page and is the committed one.
    Done,
    /// There was nowhere to go: the first slice or the last. A page that was dirty was repaired first.
    AtBoundary,
    /// A seek was asked for and will arrive at a frame.
    Pending,
    /// The host is hidden, detached or disposed, so nothing was drawn and nothing was committed.
    Unavailable,
    /// A write or a schedule failed, or the engine refused. The cursor is where it was.
    Failed,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controller's own engine holds. The slice that its counters say is read from the engine, and the slice that
/// the controller believes it is at is the controller's own record, so a disagreement between them is a defect of the
/// bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEngine {
    /// The slice that the controller recorded when it last stepped or replaced the engine.
    pub recorded_at: u64,
    /// The slice that the engine's own counters say it has done.
    pub slice: u64,
    /// The hash that the engine makes of its whole state, as it does at the end of that slice.
    pub sha256: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A slice that has been worked out and has not been committed.
struct Staged {
    slice: u64,
    model: Model,
    projection: Projection,
    /// The selection that the projection was made for, which is published with it.
    view: View,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A seek that is being replayed, a slice or a frame's worth at a time.
struct SeekJob {
    target: u64,
    engine: Engine,
    at: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct State {
    factory: Factory,
    /// The engine, which may be ahead of the committed slice after a draw that failed. It is only ever read at the
    /// slice it is at.
    engine: Engine,
    engine_at: u64,
    staged: Option<Staged>,
    /// Slices worked out in advance by [`Controller::prepare_ahead`], oldest first, each for the view it was made for.
    ahead: VecDeque<Staged>,
    committed_slice: u64,
    committed_model: Model,
    committed: Projection,
    view: View,
    last_slice: Option<u64>,
    generation: u64,
    epoch: u64,
    seek_token: u64,
    timer: Option<u64>,
    frame: Option<u64>,
    playing: bool,
    dirty: bool,
    disposed: bool,
    reported: bool,
    announced: bool,
    speed_ms: u32,
    chunk: Option<u64>,
    seek: Option<SeekJob>,
    retry: Option<u64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Inner<H: Host, S: Scheduler> {
    host: RefCell<H>,
    scheduler: S,
    state: RefCell<State>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn model_and_projection(engine: &Engine, view: &View) -> (Model, Projection) {
    let model = crate::projection::extract(engine, view);
    let projection = project(&model, view);
    (model, projection)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The controller. It is cheap to clone, and every clone is the same controller.
pub struct Controller<H: Host + 'static, S: Scheduler + 'static> {
    inner: Rc<Inner<H, S>>,
}

impl<H: Host + 'static, S: Scheduler + 'static> Clone for Controller<H, S> {
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

impl<H: Host + 'static, S: Scheduler + 'static> Controller<H, S> {
    /// Makes a controller at the first slice and draws it in full. A draw that fails leaves the controller dirty, and
    /// the failure is reported.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the engine cannot be made.
    pub fn new(host: H, scheduler: S, factory: Factory, view: View) -> Result<Self, Error> {
        let engine = factory()?;
        let (committed_model, committed) = model_and_projection(&engine, &view);
        let state = State {
            factory,
            engine,
            engine_at: 0,
            staged: None,
            ahead: VecDeque::new(),
            committed_slice: 0,
            committed_model,
            committed,
            view,
            last_slice: None,
            generation: 0,
            epoch: 0,
            seek_token: 0,
            timer: None,
            frame: None,
            playing: false,
            dirty: true,
            disposed: false,
            reported: false,
            announced: false,
            speed_ms: 1_000,
            chunk: None,
            seek: None,
            retry: None,
        };
        let me = Self {
            inner: Rc::new(Inner {
                host: RefCell::new(host),
                scheduler,
                state: RefCell::new(state),
            }),
        };
        me.inner.full_repair();
        Ok(me)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The slice on the page, which is the last one that was drawn and committed.
    #[must_use]
    pub fn committed_slice(&self) -> u64 {
        self.inner.state.borrow().committed_slice
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the page should hold, which is the committed slice's projection.
    #[must_use]
    pub fn committed(&self) -> Projection {
        self.inner.state.borrow().committed.clone()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The model of the committed slice, which is what the controller holds and checks a draw against. It is read here
    /// as it is, and is not worked out again from the run.
    #[must_use]
    pub fn committed_model(&self) -> Model {
        self.inner.state.borrow().committed_model.clone()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the controller's own engine holds now. A slice is worked out before it is drawn, so after a draw that
    /// failed the engine is one slice ahead of the committed one, and that is a state that the controller keeps on
    /// purpose.
    #[must_use]
    pub fn live_engine(&self) -> LiveEngine {
        let st = self.inner.state.borrow();
        let model = crate::projection::extract(&st.engine, &st.view);
        LiveEngine {
            recorded_at: st.engine_at,
            slice: model.slice,
            sha256: st.engine.checkpoint(model.tick, model.slice),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The time between the slices of playback, in milliseconds, which is kept while it is not playing.
    #[must_use]
    pub fn speed_ms(&self) -> u32 {
        self.inner.state.borrow().speed_ms
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether [`dispose`](Self::dispose) has been called.
    #[must_use]
    pub fn is_disposed(&self) -> bool {
        self.inner.state.borrow().disposed
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether the page may differ from the committed projection, because a draw failed or the host went.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.state.borrow().dirty
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.inner.state.borrow().playing
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.inner.state.borrow().epoch
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[must_use]
    pub fn seek_token(&self) -> u64 {
        self.inner.state.borrow().seek_token
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.state.borrow().generation
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The job that the committed projection has selected, if any. It changes only when a draw with that selection was
    /// committed.
    #[must_use]
    pub fn selected(&self) -> Option<u64> {
        self.inner.state.borrow().view.selected
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The number of the last slice of the run, once it has been reached.
    #[must_use]
    pub fn last_slice(&self) -> Option<u64> {
        self.inner.state.borrow().last_slice
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The seek that failed to ask for a frame, which [`retry`](Self::retry) asks for again.
    #[must_use]
    pub fn pending_retry(&self) -> Option<u64> {
        self.inner.state.borrow().retry
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs `f` on the host, which a test reads what was drawn from.
    pub fn with_host<R>(&self, f: impl FnOnce(&mut H) -> R) -> R {
        f(&mut self.inner.host.borrow_mut())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The scheduler, for a test that moves its clock.
    #[must_use]
    pub fn scheduler(&self) -> &S {
        &self.inner.scheduler
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes a seek replay a few slices a frame, so that a test can cancel it between frames. `None` replays a whole
    /// seek in one frame. A chunk of zero would ask for a frame for ever without doing any work, so it is taken as one.
    pub fn force_chunk(&self, slices: Option<u64>) {
        self.inner.state.borrow_mut().chunk = slices.map(|n| n.max(1));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Changes the time between the slices of playback. If it is playing, the next slice is scheduled again at the new
    /// delay and the old timer is taken back, so that a slice does not come twice.
    pub fn set_speed(&self, ms: u32) -> Outcome {
        {
            let mut st = self.inner.state.borrow_mut();
            st.speed_ms = ms;
            if !st.playing {
                return Outcome::Done;
            }
        }
        self.inner.restart_timer()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows the next slice. Nowhere to go at the last one, and then a dirty page is repaired.
    pub fn next(&self) -> Outcome {
        self.inner.claim_navigation();
        self.inner.step_next()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows the slice before the committed one, by replaying the run to it at a frame.
    pub fn previous(&self) -> Outcome {
        self.inner.claim_navigation();
        let at = self.inner.state.borrow().committed_slice;
        if at == 0 {
            return self.inner.boundary();
        }
        self.inner.start_seek(at - 1)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows the first slice, by replaying nothing at a frame.
    pub fn restart(&self) -> Outcome {
        self.inner.claim_navigation();
        if self.inner.state.borrow().committed_slice == 0 {
            return self.inner.boundary();
        }
        self.inner.start_seek(0)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows slice `target`, by replaying the run to it at a frame. A target past the end is the last slice.
    pub fn seek(&self, target: u64) -> Outcome {
        self.inner.claim_navigation();
        self.inner.start_seek(target)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Asks again for the frame of a seek that could not be asked for.
    pub fn retry(&self) -> Outcome {
        let target = self.inner.state.borrow().retry;
        match target {
            Some(t) => {
                self.inner.claim_navigation();
                self.inner.start_seek(t)
            },
            None => Outcome::Done,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts playback. It begins from the committed slice, and the first slice comes after one delay.
    pub fn play(&self) -> Outcome {
        self.inner.claim_navigation();
        self.inner.play()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops playback, and a timer that was waiting is taken back.
    pub fn pause(&self) {
        self.inner.stop_playing();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Selects a job, or none, which changes what the page says and not the slice.
    pub fn select_job(&self, id: Option<u64>) -> Outcome {
        self.inner.select_job(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the committed slice again in full, which is what a dirty page needs.
    pub fn repair(&self) -> Outcome {
        self.inner.repair()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Changes the history: a new engine, from its first slice. Everything that was scheduled is invalidated.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the engine cannot be made, in which case nothing changed.
    pub fn switch_scenario(&self, factory: Factory) -> Result<Outcome, Error> {
        let engine = factory()?;
        Ok(self.inner.switch_scenario(factory, engine))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The hint that the panel was shown or hidden: the same stop and invalidation as for a hidden page, and a repair
    /// of a dirty page when it is shown. It never starts playback.
    pub fn presence_changed(&self) {
        self.inner.presence_changed();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops playback and invalidates what was scheduled, as for a page that is being left. The page may still read as
    /// visible, so this does not ask. It never starts anything, and the page is not marked dirty.
    pub fn suspend(&self) {
        self.inner.stop_and_invalidate();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out the next `count` slices after the committed one, and keeps them, so that the `next` calls that follow
    /// draw and commit them and do not step the engine or build a projection. It exists so that a measurement of `next`
    /// can be the render and the commit alone. It changes no committed state, and a slice that is kept is used only if
    /// it is still the next one for the view that is committed then.
    ///
    /// Returns how many slices were worked out, which is fewer than `count` at the end of the run.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the engine could not be stepped, or replayed to the committed slice.
    pub fn prepare_ahead(&self, count: u64) -> Result<u64, String> {
        self.inner.prepare_ahead(count).map(|checkpoints| checkpoints.len() as u64)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// As [`prepare_ahead`](Self::prepare_ahead), and gives the engine's own hash of its state after each slice that it
    /// worked out, in order. They are the hashes of the slices that the `next` calls then commit, taken from the engine
    /// as it stepped.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the engine could not be stepped, or replayed to the committed slice.
    pub fn prepare_ahead_with_checkpoints(&self, count: u64) -> Result<Vec<String>, String> {
        self.inner.prepare_ahead(count)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Ends the controller for good. What was scheduled is taken back, and a late callback does nothing.
    pub fn dispose(&self) {
        {
            self.inner.state.borrow_mut().disposed = true;
        }
        self.inner.stop_and_invalidate();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl<H: Host + 'static, S: Scheduler + 'static> Inner<H, S> {
    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether a draw is allowed now. This is the synchronous predicate, and it is asked of the host every time.
    fn live(&self) -> bool {
        !self.state.borrow().disposed && self.host.borrow().presence() == Presence::Live
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn announce_unavailable(&self) {
        let first = {
            let mut st = self.state.borrow_mut();
            !std::mem::replace(&mut st.announced, true)
        };
        if first {
            self.host
                .borrow_mut()
                .announce("The simulator is not available, and is paused.");
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn report_once(&self, text: &str) {
        let first = {
            let mut st = self.state.borrow_mut();
            !std::mem::replace(&mut st.reported, true)
        };
        if first {
            self.host.borrow_mut().report(text);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes a manual navigation the only owner of the slice that is shown: playback is stopped and a seek that was
    /// pending is cancelled, and every callback of either that is already on its way is stale. It is done first, before
    /// the request is looked at, so that a request that ends at a boundary has still replaced what came before it, and
    /// the last request that a person made is the one that wins. Playback's own ticks do not come through here, since a
    /// tick must not stop the playback that it is part of.
    fn claim_navigation(&self) {
        self.stop_and_invalidate();
        // A retry is the latest request that failed to be scheduled, and a later request has superseded it. A hide does
        // not, so it is not cleared by the stop above, and a retry that was asked for is captured by the caller before
        // it claims.
        self.state.borrow_mut().retry = None;
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops playback and takes back everything that was scheduled, and makes every callback already on its way stale.
    fn stop_and_invalidate(&self) {
        let (timer, frame, was_playing) = {
            let mut st = self.state.borrow_mut();
            st.epoch += 1;
            st.seek_token += 1;
            st.seek = None;
            (st.timer.take(), st.frame.take(), std::mem::replace(&mut st.playing, false))
        };
        if let Some(h) = timer {
            self.scheduler.cancel(h);
        }
        if let Some(h) = frame {
            self.scheduler.cancel_frame(h);
        }
        if was_playing {
            self.host.borrow_mut().playing_changed(false);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops playback and nothing else, so that a seek that is on its way still arrives.
    fn stop_playing(&self) {
        let (timer, was_playing) = {
            let mut st = self.state.borrow_mut();
            st.epoch += 1;
            (st.timer.take(), std::mem::replace(&mut st.playing, false))
        };
        if let Some(h) = timer {
            self.scheduler.cancel(h);
        }
        if was_playing {
            self.host.borrow_mut().playing_changed(false);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the committed projection in full, if the host is there to draw on, and checks that it still was when the
    /// draw was done. Success clears the dirty mark. It never starts playback.
    fn full_repair(&self) -> bool {
        if !self.live() {
            return false;
        }
        let projection = self.state.borrow().committed.clone();
        let result = self.host.borrow_mut().draw(&projection, true);
        // A repair is a draw, and a host can leave in the middle of one. It is treated as the commit treats it:
        // playback and every callback on its way are stopped at once, the page is marked dirty, and the host is
        // announced as unavailable.
        if !self.live() {
            self.stop_and_invalidate();
            self.state.borrow_mut().dirty = true;
            self.announce_unavailable();
            return false;
        }
        match result {
            Ok(()) => {
                self.state.borrow_mut().dirty = false;
                true
            },
            Err(failure) => {
                self.state.borrow_mut().dirty = true;
                self.report_once(&describe(&failure));
                false
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The commit of design section 6.1: check, draw, check, and only then move the cursor.
    fn render_then_commit(self: &Rc<Self>, target: Staged) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            self.announce_unavailable();
            return Outcome::Unavailable;
        }
        if self.state.borrow().dirty && !self.full_repair() {
            self.stop_and_invalidate();
            return if self.live() { Outcome::Failed } else { self.unavailable() };
        }
        if !self.live() {
            self.stop_and_invalidate();
            self.announce_unavailable();
            return Outcome::Unavailable;
        }
        let result = self.host.borrow_mut().draw(&target.projection, false);
        // The draw is synchronous and a host can leave in the middle of it, so the guard is asked again, and a write
        // that went through is no evidence that the page is attached.
        if !self.live() {
            self.stop_and_invalidate();
            self.state.borrow_mut().dirty = true;
            self.announce_unavailable();
            return Outcome::Unavailable;
        }
        if let Err(failure) = result {
            self.stop_and_invalidate();
            self.state.borrow_mut().dirty = true;
            self.report_once(&describe(&failure));
            // One immediate, bounded repair of what was committed, and no retry after it.
            self.full_repair();
            return Outcome::Failed;
        }
        let mut st = self.state.borrow_mut();
        st.committed_slice = target.slice;
        st.committed_model = target.model;
        st.committed = target.projection;
        st.view = target.view;
        st.staged = None;
        st.reported = false;
        st.announced = false;
        Outcome::Done
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn unavailable(&self) -> Outcome {
        self.announce_unavailable();
        Outcome::Unavailable
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A request that has nowhere to go: a dirty page is repaired first, and then nothing else happens. A repair that
    /// fails makes the request a failure, and not a boundary, so that a caller can tell a no-op from a recovery that
    /// did not work. One that cannot be made because the host went is unavailable.
    fn boundary(&self) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        if self.state.borrow().dirty && !self.full_repair() {
            return if self.live() { Outcome::Failed } else { self.unavailable() };
        }
        Outcome::AtBoundary
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replays the run from its first slice to `to`, into a new engine. Staging happens here and never touches the
    /// committed state.
    fn replay(&self, to: u64) -> Result<Engine, String> {
        let factory = Rc::clone(&self.state.borrow().factory);
        let mut engine = factory().map_err(|e| e.message())?;
        for _ in 0..to {
            if engine.step().map_err(|e| e.message())?.is_none() {
                return Err("the run is shorter than the slice that was asked for".to_owned());
            }
        }
        Ok(engine)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn step_next(self: &Rc<Self>) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        let (committed, last) = {
            let st = self.state.borrow();
            (st.committed_slice, st.last_slice)
        };
        if last.is_some_and(|l| committed >= l) {
            return self.boundary();
        }
        let target = committed + 1;
        let staged = {
            let mut st = self.state.borrow_mut();
            st.staged.take().filter(|s| s.slice == target)
        };
        let staged = staged.or_else(|| self.take_ahead(target));
        let staged = match staged {
            Some(s) => s,
            None => match self.stage_after(committed) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    self.state.borrow_mut().last_slice = Some(committed);
                    return self.boundary();
                },
                Err(problem) => {
                    self.stop_and_invalidate();
                    self.report_once(&problem);
                    return Outcome::Failed;
                },
            },
        };
        // The staged slice is kept, so that a draw that fails and is asked for again does not step the engine twice.
        let copy = Staged {
            slice: staged.slice,
            model: staged.model.clone(),
            projection: staged.projection.clone(),
            view: staged.view,
        };
        let outcome = self.render_then_commit(staged);
        if outcome != Outcome::Done {
            self.state.borrow_mut().staged = Some(copy);
        }
        outcome
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The slice `target` if it was worked out in advance for the view that is committed now, and `None` otherwise. A
    /// slice that was worked out for another view, or that the cursor has passed, is dropped with everything after it,
    /// since it would not be what stepping to it gives.
    fn take_ahead(&self, target: u64) -> Option<Staged> {
        let mut st = self.state.borrow_mut();
        while st.ahead.front().is_some_and(|s| s.slice < target) {
            st.ahead.pop_front();
        }
        match st.ahead.front() {
            Some(s) if s.slice == target && s.view == st.view => st.ahead.pop_front(),
            _ => {
                st.ahead.clear();
                None
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out the next `count` slices after the committed one and keeps them, so that the `next` calls that follow
    /// draw and commit them without working them out. The engine is stepped here, and what a later `next` costs is then
    /// the render and the commit.
    fn prepare_ahead(&self, count: u64) -> Result<Vec<String>, String> {
        let mut checkpoints = Vec::new();
        let committed = self.state.borrow().committed_slice;
        {
            let mut st = self.state.borrow_mut();
            st.ahead.clear();
            st.staged = None;
        }
        if self.state.borrow().engine_at != committed {
            let engine = self.replay(committed)?;
            let mut st = self.state.borrow_mut();
            st.engine = engine;
            st.engine_at = committed;
        }
        let mut made = 0;
        while made < count {
            let mut st = self.state.borrow_mut();
            let Some(record) = st.engine.step().map_err(|e| e.message())? else {
                break;
            };
            checkpoints.push(record.checkpoint);
            made += 1;
            st.engine_at = committed + made;
            let (model, projection) = model_and_projection(&st.engine, &st.view);
            let view = st.view;
            st.ahead.push_back(Staged {
                slice: committed + made,
                model,
                projection,
                view,
            });
        }
        Ok(checkpoints)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out the slice after `committed`, from the engine, which is brought to `committed` first if it is not
    /// there.
    fn stage_after(&self, committed: u64) -> Result<Option<Staged>, String> {
        if self.state.borrow().engine_at != committed {
            let engine = self.replay(committed)?;
            let mut st = self.state.borrow_mut();
            st.engine = engine;
            st.engine_at = committed;
        }
        let mut st = self.state.borrow_mut();
        let stepped = st.engine.step().map_err(|e| e.message())?;
        if stepped.is_none() {
            return Ok(None);
        }
        st.engine_at = committed + 1;
        let (model, projection) = model_and_projection(&st.engine, &st.view);
        Ok(Some(Staged {
            slice: committed + 1,
            model,
            projection,
            view: st.view,
        }))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn play(self: &Rc<Self>) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        if self.state.borrow().dirty && !self.full_repair() {
            return if self.live() { Outcome::Failed } else { Outcome::Unavailable };
        }
        {
            let st = self.state.borrow();
            if st.last_slice.is_some_and(|l| st.committed_slice >= l) {
                return Outcome::AtBoundary;
            }
        }
        // A play that is already playing starts again, so that there is never more than one timer.
        self.stop_playing();
        {
            let mut st = self.state.borrow_mut();
            st.epoch += 1;
            st.playing = true;
        }
        self.host.borrow_mut().playing_changed(true);
        self.restart_timer()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Schedules the next slice of playback under the current epoch, taking back a timer that was waiting.
    fn restart_timer(self: &Rc<Self>) -> Outcome {
        let old = self.state.borrow_mut().timer.take();
        if let Some(h) = old {
            self.scheduler.cancel(h);
        }
        {
            let mut st = self.state.borrow_mut();
            st.epoch += 1;
        }
        self.schedule_tick()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn schedule_tick(self: &Rc<Self>) -> Outcome {
        let (epoch, generation, delay) = {
            let st = self.state.borrow();
            (st.epoch, st.generation, st.speed_ms)
        };
        let weak = self.weak();
        let task = Box::new(move || {
            if let Some(inner) = weak.upgrade() {
                inner.on_tick(epoch, generation);
            }
        });
        match self.scheduler.schedule_once(delay, task) {
            Ok(handle) => {
                self.state.borrow_mut().timer = Some(handle);
                Outcome::Done
            },
            Err(problem) => {
                // The last committed display stays, and playback stops.
                self.stop_playing();
                self.report_once(&problem);
                Outcome::Failed
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn on_tick(self: &Rc<Self>, epoch: u64, generation: u64) {
        {
            let mut st = self.state.borrow_mut();
            if st.disposed || st.epoch != epoch || st.generation != generation || !st.playing {
                return;
            }
            st.timer = None;
        }
        if !self.live() {
            self.stop_and_invalidate();
            self.announce_unavailable();
            return;
        }
        match self.step_next() {
            Outcome::Done => {
                let at_end = {
                    let st = self.state.borrow();
                    st.last_slice.is_some_and(|l| st.committed_slice >= l)
                };
                if at_end || !self.live() {
                    self.stop_playing();
                } else {
                    // The next timer is scheduled only after a slice was committed.
                    self.schedule_tick();
                }
            },
            Outcome::AtBoundary => self.stop_playing(),
            _ => {},
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn start_seek(self: &Rc<Self>, target: u64) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        let (committed, last) = {
            let st = self.state.borrow();
            (st.committed_slice, st.last_slice)
        };
        let target = last.map_or(target, |l| target.min(l));
        if target == committed {
            return self.boundary();
        }
        // The caller claimed the navigation, so the seek before this one was cancelled, and a frame that is on its way
        // for it is stale.
        self.state.borrow_mut().retry = None;
        let factory = Rc::clone(&self.state.borrow().factory);
        let engine = match factory() {
            Ok(e) => e,
            Err(e) => {
                self.report_once(&e.message());
                return Outcome::Failed;
            },
        };
        {
            let mut st = self.state.borrow_mut();
            st.seek = Some(SeekJob { target, engine, at: 0 });
        }
        self.request_seek_frame()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn request_seek_frame(self: &Rc<Self>) -> Outcome {
        let (token, generation) = {
            let st = self.state.borrow();
            (st.seek_token, st.generation)
        };
        let weak = self.weak();
        let task = Box::new(move || {
            if let Some(inner) = weak.upgrade() {
                inner.on_seek_frame(token, generation);
            }
        });
        match self.scheduler.request_frame(task) {
            Ok(handle) => {
                self.state.borrow_mut().frame = Some(handle);
                Outcome::Pending
            },
            Err(problem) => {
                // The cursor stays, and a person can ask again. A seek is not hidden behind a synchronous fallback.
                let target = {
                    let mut st = self.state.borrow_mut();
                    st.retry = st.seek.take().map(|s| s.target);
                    st.retry
                };
                let _ = target;
                self.report_once(&problem);
                Outcome::Failed
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn on_seek_frame(self: &Rc<Self>, token: u64, generation: u64) {
        {
            let mut st = self.state.borrow_mut();
            if st.disposed || st.seek_token != token || st.generation != generation || st.seek.is_none() {
                return;
            }
            st.frame = None;
        }
        if !self.live() {
            self.stop_and_invalidate();
            self.announce_unavailable();
            return;
        }
        // The work is done on the job, which is not the committed state, so a seek that is replaced half way changes
        // nothing that is shown.
        let finished = {
            let mut st = self.state.borrow_mut();
            let budget = st.chunk.unwrap_or(u64::MAX);
            let mut job = st.seek.take().expect("a seek job");
            let mut done = 0;
            let mut ended = false;
            let mut failed = None;
            while job.at < job.target && done < budget {
                match job.engine.step() {
                    Ok(Some(_)) => {
                        job.at += 1;
                        done += 1;
                    },
                    // The run has no more slices, which is its natural end, and the last slice is where the replay got
                    // to.
                    Ok(None) => {
                        ended = true;
                        break;
                    },
                    // The engine refused to go on. That says nothing about where the run ends, so nothing is recorded
                    // of it.
                    Err(e) => {
                        failed = Some(e.message());
                        break;
                    },
                }
            }
            if let Some(problem) = failed {
                // The seek is over, and it changed nothing that is shown or known: the cursor, the committed projection
                // and the last slice are as they were. The job is dropped with the engine that failed.
                Err(problem)
            } else {
                if ended {
                    job.target = job.at;
                    st.last_slice = Some(job.at);
                }
                let reached = job.at >= job.target;
                st.seek = Some(job);
                Ok(reached)
            }
        };
        let finished = match finished {
            Ok(reached) => reached,
            Err(problem) => {
                self.report_once(&problem);
                return;
            },
        };
        if !finished {
            self.request_seek_frame();
            return;
        }
        let job = self.state.borrow_mut().seek.take().expect("a seek job");
        let view = self.state.borrow().view;
        let (model, projection) = model_and_projection(&job.engine, &view);
        {
            let mut st = self.state.borrow_mut();
            st.engine = job.engine;
            st.engine_at = job.at;
        }
        let outcome = self.render_then_commit(Staged {
            slice: job.at,
            model,
            projection,
            view,
        });
        let _ = outcome;
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn select_job(self: &Rc<Self>, id: Option<u64>) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        // The selection is staged with the projection that it makes, and is published by the commit and not before it.
        // A draw that fails leaves the view that was shown, so that a later slice is not drawn with a selection that
        // was never on the page.
        let committed = self.state.borrow().committed_slice;
        let view = View { selected: id };
        if self.state.borrow().engine_at != committed {
            match self.replay(committed) {
                Ok(engine) => {
                    let mut st = self.state.borrow_mut();
                    st.engine = engine;
                    st.engine_at = committed;
                },
                Err(problem) => {
                    self.report_once(&problem);
                    return Outcome::Failed;
                },
            }
        }
        let (model, projection) = model_and_projection(&self.state.borrow().engine, &view);
        self.render_then_commit(Staged {
            slice: committed,
            model,
            projection,
            view,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn repair(&self) -> Outcome {
        if !self.live() {
            self.stop_and_invalidate();
            return self.unavailable();
        }
        if self.full_repair() {
            Outcome::Done
        } else if self.live() {
            Outcome::Failed
        } else {
            // The host left in the middle of the repair, which the repair has already announced.
            Outcome::Unavailable
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Changes the history in a transaction. The new scenario is staged and drawn in full, and it replaces the old one
    /// only if that draw succeeded while the host was still there. Otherwise the old scenario is what is committed, and
    /// the page is repaired to it. A switch is a navigation, so playback and a pending seek are cancelled whether or
    /// not it succeeds.
    fn switch_scenario(self: &Rc<Self>, factory: Factory, engine: Engine) -> Outcome {
        self.claim_navigation();
        if !self.live() {
            return self.unavailable();
        }
        let view = View::default();
        let (model, projection) = model_and_projection(&engine, &view);
        let result = self.host.borrow_mut().draw(&projection, true);
        if !self.live() {
            self.state.borrow_mut().dirty = true;
            return self.unavailable();
        }
        if let Err(failure) = result {
            self.state.borrow_mut().dirty = true;
            self.report_once(&describe(&failure));
            // One immediate repair, of the scenario that is still committed.
            self.full_repair();
            return Outcome::Failed;
        }
        let mut st = self.state.borrow_mut();
        st.generation += 1;
        st.factory = factory;
        st.engine = engine;
        st.engine_at = 0;
        st.staged = None;
        st.ahead.clear();
        st.committed_slice = 0;
        st.committed_model = model;
        st.committed = projection;
        st.view = view;
        st.last_slice = None;
        st.retry = None;
        st.dirty = false;
        st.reported = false;
        st.announced = false;
        Outcome::Done
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn presence_changed(self: &Rc<Self>) {
        if !self.live() {
            self.stop_and_invalidate();
            self.state.borrow_mut().dirty = true;
            self.announce_unavailable();
            return;
        }
        self.state.borrow_mut().announced = false;
        if self.state.borrow().dirty {
            self.full_repair();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn describe(failure: &DrawFailure) -> String {
    match failure {
        DrawFailure::Detached => "The simulator's host is not in the page.".to_owned(),
        DrawFailure::Failed(what) => format!("The simulator could not draw: {what}"),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
