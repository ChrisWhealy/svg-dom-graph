//! The session coordinator: replacing the surface and controller that are on the page by new ones, without risking the
//! picture that is there.
//!
//! The surface is built for one number of servers and never changes shape, so a scenario with another number needs a
//! new surface and a new controller (design-specs/dess_demo.pdf, section 7). That is simulator session management and
//! not a gallery algorithm, so it is here, with no browser in it. What does touch the page is behind one trait,
//! [`SessionHost`], which a gallery binds to its DOM and a test binds to a fake.
//!
//! A replacement is a transaction with a fixed order.
//!
//! 1. **Preflight.** The request is checked, and a request that is not valid is rejected with nothing changed, not even
//!    the old playback.
//! 2. **Stage.** The old playback and any pending seek are invalidated, since a request owns navigation. A candidate
//!    session is built at its first slice, and the saved playback speed is applied while it is paused.
//! 3. **Validate.** A controller that was made is not a controller that drew: its first draw may have failed and left
//!    it dirty, so it is checked clean, at slice 0, on a live host, and then by the host's own readback.
//! 4. **Publish.** The host hands the page from the old session to the new one: visibility, pointer and focus. If any
//!    step fails, what was done is undone. A handoff that reports success is then read back by the host
//!    ([`SessionHost::verify_published`]), since a candidate that is still staged is as live to the controller as one
//!    that is shown, and only the page knows which it is.
//! 5. **Commit.** Only now does the stable target point at the new controller, and the old one is disposed, once, with
//!    its host removed.
//!
//! Any failure before the commit rolls back: the candidate is disposed and removed, the old session is where it was,
//! and it is repaired if the handoff may have disturbed its page. The old simulation cursor, selection and scenario are
//! unchanged. The old *playback* is not promised: a request invalidates it at the start, and a cancelled timer is not
//! restored.
//!
//! A scenario with the same number of servers needs none of this. The controller switches its own history in place, and
//! that is a transaction of its own (`Controller::switch_scenario`), so it is used directly.
//!
//! The hooks that a gallery installs once, a mutation observer and the page's lifecycle listeners, must reach whichever
//! controller is current. They hold a [`TargetHook`], which holds the [`Target`] weakly, so nothing owns anything in a
//! circle and a hook outlives a replacement without pointing at a controller that is gone.

use crate::{
    controller::{Controller, Factory, Host, Outcome, Presence},
    scheduler::Scheduler,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

/// The most servers that a surface is built for.
pub const MAX_SERVERS: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a scenario is known to come to, which is checked by running it and not assumed. The totals are exact, and the
/// ceilings are what a run may not exceed while it is being counted, so that a scenario that does not end is refused
/// after a bounded amount of work and not after a measured amount of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expected {
    pub slices: u64,
    pub pops: u64,
    pub max_slices: u64,
    pub max_pops: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a scenario's run was found to come to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub slices: u64,
    pub pops: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a scenario was refused before anything was built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightError {
    /// The surface is built for one to four servers.
    Servers(usize),
    /// The engine has a different number of servers from the one that the spec says, so the surface would be the wrong
    /// shape.
    EngineServers { found: usize, declared: usize },
    /// The engine could not be made or stepped.
    Engine(String),
    /// The run went past a ceiling before it ended.
    Ceiling { what: &'static str, limit: u64 },
    /// The run ended, and its totals are not the ones that the scenario is known by.
    Mismatch { found: Extent, expected: Extent },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scenario that can be shown: its identity, the shape of the surface that it needs, how to make its engine, and what
/// it is known to come to.
#[derive(Clone)]
pub struct Spec {
    pub id: String,
    pub label: String,
    pub servers: usize,
    pub factory: Factory,
    pub expected: Expected,
}

impl Spec {
    /// Runs the scenario's engine to its natural end, counting, and checks it against what it is known to come to. It
    /// builds nothing and draws nothing, and the number that it gives is the exact end of the run, which
    /// `Controller::last_slice` does not know until it has been reached.
    ///
    /// # Errors
    ///
    /// Returns a [`PreflightError`] if the server count is not one to four, the engine fails or has other servers than
    /// declared, a ceiling is passed, or the totals are not the expected ones.
    pub fn preflight(&self) -> Result<Extent, PreflightError> {
        if !(1..=MAX_SERVERS).contains(&self.servers) {
            return Err(PreflightError::Servers(self.servers));
        }
        let mut engine = (self.factory)().map_err(|e| PreflightError::Engine(e.message()))?;
        let found = engine.model(0, None).servers.len();
        if found != self.servers {
            return Err(PreflightError::EngineServers { found, declared: self.servers });
        }
        let mut slices = 0;
        while engine.step().map_err(|e| PreflightError::Engine(e.message()))?.is_some() {
            slices += 1;
            if slices > self.expected.max_slices {
                return Err(PreflightError::Ceiling {
                    what: "slices",
                    limit: self.expected.max_slices,
                });
            }
            if engine.counters().pops > self.expected.max_pops {
                return Err(PreflightError::Ceiling {
                    what: "pops",
                    limit: self.expected.max_pops,
                });
            }
        }
        let found = Extent {
            slices,
            pops: engine.counters().pops,
        };
        let expected = Extent {
            slices: self.expected.slices,
            pops: self.expected.pops,
        };
        if found == expected {
            Ok(found)
        } else {
            Err(PreflightError::Mismatch { found, expected })
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A candidate session: the controller, drawn at its first slice, and what the host needs to find its page again.
pub struct Candidate<H: Host + 'static, S: Scheduler + 'static, T> {
    pub controller: Controller<H, S>,
    pub token: T,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that touches the page, which a gallery binds to its DOM and a test binds to a fake.
// A candidate is named by three associated types of the trait that builds it, which is long and says exactly what it
// is.
#[allow(clippy::type_complexity)]
pub trait SessionHost {
    type H: Host + 'static;
    type S: Scheduler + 'static;
    /// What identifies one session's page: its host element, and whatever is needed to show it, hide it and remove it.
    type Token;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stages a host that is connected, measurable and out of sight and reach, builds a surface into it, and makes a
    /// controller at the first slice. It may return a controller that is dirty, since the first draw can fail after the
    /// controller exists. The saved speed is applied by the coordinator.
    ///
    /// # Errors
    ///
    /// Returns a message if anything could not be made. What was made before the failure is the host's to remove.
    fn build(&mut self, spec: &Spec) -> Result<Candidate<Self::H, Self::S, Self::Token>, String>;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A readback of the candidate's page, which the controller cannot give: the data cells are on the page, there is
    /// one scene, and no id is duplicated.
    ///
    /// # Errors
    ///
    /// Returns a message if the page is not what a draw at slice 0 gives.
    fn verify(&mut self, candidate: &Candidate<Self::H, Self::S, Self::Token>) -> Result<(), String>;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Hands the page from `from`, if there is one, to `to`: what is shown, what takes the pointer and where the focus
    /// is. It is several steps, and it may fail part way.
    ///
    /// # Errors
    ///
    /// Returns a message if a step failed. The coordinator then calls [`undo_publish`](Self::undo_publish).
    fn publish(&mut self, from: Option<&Self::Token>, to: &Self::Token) -> Result<(), String>;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Reads back, from the page and not from what [`publish`](Self::publish) did, what a completed handoff leaves. The
    /// new page is shown and no longer staged, it is the only scene that is exposed to the keyboard, the pointer and
    /// the accessibility tree, the old one is hidden, and the focus is on a valid target in the new one.
    ///
    /// The controller cannot tell a staged host from a shown one: an `opacity:0` host that is `inert` is as `Live` to
    /// it as a visible host is. So a `publish` that returns `Ok` without having removed the staging, shown the page or
    /// moved the focus passes every check that the coordinator can make itself, and only this readback sees it.
    ///
    /// # Errors
    ///
    /// Returns a message naming what is not as a completed handoff leaves it. The coordinator then undoes the handoff
    /// and rolls back.
    fn verify_published(&mut self, from: Option<&Self::Token>, to: &Self::Token) -> Result<(), String>;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts the page back as `from` had it, after a [`publish`](Self::publish) that failed or was refused, whatever
    /// part of it was done.
    ///
    /// # Errors
    ///
    /// Returns a message if it could not be put back, which is reported and does not stop the rollback.
    fn undo_publish(&mut self, from: Option<&Self::Token>, to: &Self::Token) -> Result<(), String>;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes a candidate that was not published, with everything that was made for it.
    fn discard(&mut self, token: Self::Token);

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes the page of a session that has been replaced, once the replacement is live.
    fn finalize(&mut self, token: Self::Token);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one place that the current controller is kept, and the only thing that a replacement changes. It is held
/// strongly by the coordinator, and the hooks that a page installs hold it weakly.
pub struct Target<H: Host + 'static, S: Scheduler + 'static> {
    current: Rc<RefCell<Option<Controller<H, S>>>>,
}

impl<H: Host + 'static, S: Scheduler + 'static> Target<H, S> {
    fn new() -> Self {
        Self {
            current: Rc::new(RefCell::new(None)),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current controller, if there is one. The handle is cloned out under a short borrow, so that what is done
    /// with it cannot meet the borrow again.
    #[must_use]
    pub fn current(&self) -> Option<Controller<H, S>> {
        self.current.borrow().clone()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A hook for a callback that has to reach the current controller, which does not keep the target alive.
    #[must_use]
    pub fn hook(&self) -> TargetHook<H, S> {
        TargetHook {
            target: Rc::downgrade(&self.current),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn replace(&self, controller: Option<Controller<H, S>>) -> Option<Controller<H, S>> {
        self.current.replace(controller)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a mutation observer and the page's lifecycle listeners hold. Whatever has gone is a no-op, and a hook that
/// outlives the coordinator does nothing and starts nothing.
pub struct TargetHook<H: Host + 'static, S: Scheduler + 'static> {
    target: Weak<RefCell<Option<Controller<H, S>>>>,
}

// Written out, since a derive would ask the host and the scheduler to be `Clone`, and the hook holds neither.
impl<H: Host + 'static, S: Scheduler + 'static> Clone for TargetHook<H, S> {
    fn clone(&self) -> Self {
        Self { target: self.target.clone() }
    }
}

impl<H: Host + 'static, S: Scheduler + 'static> TargetHook<H, S> {
    /// The current controller, if the coordinator is still there and has one. The handle is cloned out under a short
    /// borrow, so that a handler that calls it cannot meet the borrow again.
    #[must_use]
    pub fn controller(&self) -> Option<Controller<H, S>> {
        self.target.upgrade().and_then(|t| t.borrow().clone())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The panel was shown or hidden: the current controller is told.
    pub fn presence_changed(&self) {
        if let Some(c) = self.controller() {
            c.presence_changed();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The page is being left: the current controller stops.
    pub fn suspend(&self) {
        if let Some(c) = self.controller() {
            c.suspend();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a replacement went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The candidate could not be built.
    Build,
    /// The candidate was built and is not clean, or is not where it should be, or its page is not what a draw gives.
    Validate,
    /// The handoff of the page failed, or the candidate was not live after it.
    Publish,
}

/// What a rollback did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rollback {
    pub stage: Stage,
    pub reason: String,
    /// Whether the handoff could not be undone, which leaves the page for a person to see and the old session still the
    /// active one.
    pub undo_failed: bool,
    /// What the repair of the old session's page gave, if one was made.
    pub repair: Option<Outcome>,
}

/// What a request for another scenario came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchOutcome {
    /// The request was not valid, and nothing was changed.
    Rejected(PreflightError),
    /// The scenario had the same number of servers, and the controller switched its history itself.
    InPlace(Outcome),
    /// A new session is on the page, and the old one is gone.
    Replaced,
    /// The old session is on the page as it was.
    RolledBack(Rollback),
}

/// Where the coordinator is in a request for another scenario, and where the last one that was acted on ended.
///
/// A request of either kind sets it: a replacement, which passes through `Staging` and `Publishing`, and a switch in
/// place. It ends as `Committed` if the scenario is now the one on the page, and as `RolledBack` if the scenario that
/// was on the page still is. A request that is rejected before anything was done changes nothing, so it leaves the
/// state of the request before it. `Idle` is before the first request and after [`Coordinator::dispose`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Staging,
    Publishing,
    Committed,
    RolledBack,
}

struct Active<T> {
    token: T,
    spec: Spec,
}

/// Owns the sessions of one panel and replaces them transactionally.
pub struct Coordinator<F: SessionHost> {
    host: F,
    target: Target<F::H, F::S>,
    active: Option<Active<F::Token>>,
    state: State,
}

impl<F: SessionHost> Coordinator<F> {
    #[must_use]
    pub fn new(host: F) -> Self {
        Self {
            host,
            target: Target::new(),
            active: None,
            state: State::Idle,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The stable target that the page's hooks are made from.
    #[must_use]
    pub fn target(&self) -> &Target<F::H, F::S> {
        &self.target
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The host, which a gallery reads its DOM through and a test reads its record of events through.
    #[must_use]
    pub fn host(&self) -> &F {
        &self.host
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The identity of the scenario that is on the page, if one is.
    #[must_use]
    pub fn active_id(&self) -> Option<&str> {
        self.active.as_ref().map(|a| a.spec.id.as_str())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows `spec`, replacing the scenario that is on the page if there is one.
    pub fn switch(&mut self, spec: Spec) -> SwitchOutcome {
        if let Err(problem) = spec.preflight() {
            return SwitchOutcome::Rejected(problem);
        }
        let old = self.target.current();
        // The same number of servers: the controller's own transaction. It never touches the surface, and a failure
        // leaves everything as it was.
        if let (Some(controller), Some(active)) = (&old, &self.active) {
            if active.spec.servers == spec.servers {
                return match controller.switch_scenario(Rc::clone(&spec.factory)) {
                    Ok(outcome) => {
                        // The scenario on the page is the new one only if the controller published it.
                        let done = outcome == Outcome::Done;
                        if done {
                            if let Some(active) = self.active.as_mut() {
                                active.spec = spec;
                            }
                        }
                        self.state = if done { State::Committed } else { State::RolledBack };
                        SwitchOutcome::InPlace(outcome)
                    },
                    Err(e) => {
                        self.state = State::RolledBack;
                        SwitchOutcome::RolledBack(Rollback {
                            stage: Stage::Build,
                            reason: e.message(),
                            undo_failed: false,
                            repair: None,
                        })
                    },
                };
            }
        }
        self.replace(spec, old)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn replace(&mut self, spec: Spec, old: Option<Controller<F::H, F::S>>) -> SwitchOutcome {
        self.state = State::Staging;
        // The request owns navigation from here. The old playback and any seek that was pending are invalidated, and
        // they are not restored if the replacement fails: what is promised is the old committed slice, selection and
        // scenario, and not the timer that was running.
        let speed = old.as_ref().map(Controller::speed_ms);
        if let Some(old) = &old {
            old.suspend();
        }
        let candidate = match self.host.build(&spec) {
            Ok(candidate) => candidate,
            Err(reason) => return self.rolled_back(Stage::Build, reason, None, false, &old),
        };
        if let Some(ms) = speed {
            candidate.controller.set_speed(ms);
        }
        if let Err(reason) = Self::validate(&candidate).and_then(|()| self.host.verify(&candidate)) {
            return self.rolled_back(Stage::Validate, reason, Some(candidate), false, &old);
        }
        self.state = State::Publishing;
        let published = self.host.publish(self.active.as_ref().map(|a| &a.token), &candidate.token);
        if let Err(reason) = published {
            return self.rolled_back(Stage::Publish, reason, Some(candidate), true, &old);
        }
        // The handoff reported success. It is read back from the page, and the candidate has to be the live, clean
        // scene that it was before it.
        let read_back = self
            .host
            .verify_published(self.active.as_ref().map(|a| &a.token), &candidate.token);
        if let Err(reason) = read_back.and_then(|()| Self::validate(&candidate)) {
            return self.rolled_back(Stage::Publish, reason, Some(candidate), true, &old);
        }
        // The commit boundary: the target and the metadata change together, and only here.
        let replaced = self.active.replace(Active { token: candidate.token, spec });
        let previous = self.target.replace(Some(candidate.controller));
        if let Some(previous) = previous {
            previous.dispose();
        }
        if let Some(replaced) = replaced {
            self.host.finalize(replaced.token);
        }
        self.state = State::Committed;
        SwitchOutcome::Replaced
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A candidate is clean, at its first slice, and on a live host.
    fn validate(candidate: &Candidate<F::H, F::S, F::Token>) -> Result<(), String> {
        let c = &candidate.controller;
        if c.is_dirty() {
            return Err("the first draw of the new scene failed, and left it dirty".to_owned());
        }
        if c.committed_slice() != 0 {
            return Err(format!("the new scene is at slice {} and not at slice 0", c.committed_slice()));
        }
        let presence = c.with_host(|h| h.presence());
        if presence != Presence::Live {
            return Err(format!("the new scene's host is {presence:?} and not Live"));
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn rolled_back(
        &mut self,
        stage: Stage,
        reason: String,
        candidate: Option<Candidate<F::H, F::S, F::Token>>,
        handed_off: bool,
        old: &Option<Controller<F::H, F::S>>,
    ) -> SwitchOutcome {
        let mut undo_failed = false;
        if let Some(candidate) = candidate {
            if handed_off {
                undo_failed = self
                    .host
                    .undo_publish(self.active.as_ref().map(|a| &a.token), &candidate.token)
                    .is_err();
            }
            // The candidate goes, with its own page, and nothing that it scheduled can come back to anything.
            candidate.controller.dispose();
            self.host.discard(candidate.token);
        }
        // The old picture is repaired if the handoff may have disturbed it, or if it was not clean to begin with.
        let repair = old
            .as_ref()
            .and_then(|old| (handed_off || old.is_dirty()).then(|| old.repair()));
        self.state = State::RolledBack;
        SwitchOutcome::RolledBack(Rollback {
            stage,
            reason,
            undo_failed,
            repair,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Ends the active session for good: its controller is disposed and its page removed. It is safe to call twice.
    pub fn dispose(&mut self) {
        if let Some(controller) = self.target.replace(None) {
            controller.dispose();
        }
        if let Some(active) = self.active.take() {
            self.host.finalize(active.token);
        }
        self.state = State::Idle;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
