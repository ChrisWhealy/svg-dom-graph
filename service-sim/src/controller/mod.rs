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

mod draw_failure;
mod host;
mod inner;
mod live_engine;
mod outcome;
mod presence;
mod seek_job;
mod staged;
mod state;

use crate::projection::{View, project};
use crate::scheduler::Scheduler;
use service_sim_engine::{Engine, Error, Model};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

pub use crate::projection::Projection;
pub use draw_failure::DrawFailure;
pub use host::Host;
use inner::Inner;
pub use live_engine::LiveEngine;
pub use outcome::Outcome;
pub use presence::Presence;
use seek_job::SeekJob;
use staged::Staged;
use state::State;

/// Makes a fresh engine at the first slice of a scenario, as often as it is asked.
pub type Factory = Rc<dyn Fn() -> Result<Engine, Error>>;

fn model_and_projection(engine: &Engine, view: &View) -> (Model, Projection) {
    let model = crate::projection::extract(engine, view);
    let projection = project(&model, view);
    (model, projection)
}

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
    /// The slice that a seek that is still being replayed is going to, so that a page can say what was asked for and
    /// what is still the committed one. It is `None` once the seek has been committed, superseded or abandoned.
    #[must_use]
    pub fn pending_seek(&self) -> Option<u64> {
        self.inner.state.borrow().seek.as_ref().map(|job| job.target)
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

fn describe(failure: &DrawFailure) -> String {
    match failure {
        DrawFailure::Detached => "The simulator's host is not in the page.".to_owned(),
        DrawFailure::Failed(what) => format!("The simulator could not draw: {what}"),
    }
}

#[cfg(test)]
mod unit_tests;
