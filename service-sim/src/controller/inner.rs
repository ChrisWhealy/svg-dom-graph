//! What a controller and its callbacks share: the host, the scheduler and the state.

use super::{Factory, Host, Outcome, Presence, SeekJob, Staged, State, describe, model_and_projection};
use crate::projection::View;
use crate::scheduler::Scheduler;
use service_sim_engine::Engine;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

/// What a controller and its callbacks share: the host, the scheduler and the state.
pub(super) struct Inner<H: Host, S: Scheduler> {
    pub(super) host: RefCell<H>,
    pub(super) scheduler: S,
    pub(super) state: RefCell<State>,
}

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
    pub(super) fn claim_navigation(&self) {
        self.stop_and_invalidate();
        // A retry is the latest request that failed to be scheduled, and a later request has superseded it. A hide does
        // not, so it is not cleared by the stop above, and a retry that was asked for is captured by the caller before
        // it claims.
        self.state.borrow_mut().retry = None;
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops playback and takes back everything that was scheduled, and makes every callback already on its way stale.
    pub(super) fn stop_and_invalidate(&self) {
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
    pub(super) fn stop_playing(&self) {
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
    pub(super) fn full_repair(&self) -> bool {
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
    pub(super) fn boundary(&self) -> Outcome {
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
    pub(super) fn step_next(self: &Rc<Self>) -> Outcome {
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
    pub(super) fn prepare_ahead(&self, count: u64) -> Result<Vec<String>, String> {
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
    pub(super) fn play(self: &Rc<Self>) -> Outcome {
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
    pub(super) fn restart_timer(self: &Rc<Self>) -> Outcome {
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
    pub(super) fn start_seek(self: &Rc<Self>, target: u64) -> Outcome {
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
    pub(super) fn select_job(self: &Rc<Self>, id: Option<u64>) -> Outcome {
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
    pub(super) fn repair(&self) -> Outcome {
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
    pub(super) fn switch_scenario(self: &Rc<Self>, factory: Factory, engine: Engine) -> Outcome {
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
    pub(super) fn presence_changed(self: &Rc<Self>) {
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
