//! The three scripted scenarios: the transitions that each fires, and the named checkpoints in each. Pure.
//!
//! A script is only a list of transitions. Every snapshot comes from firing them through the net, so a script cannot
//! write a marking, and it cannot wait. A blocked worker is a property of a marking and not an event.

use super::{
    net::{Net, TransitionId},
    trace::{Trace, TraceError},
};

/// A scripted scenario that a person can choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scenario {
    Walkthrough,
    Contention,
    Interleaving,
}

/// A named place in a script: what the control says, and the index of the snapshot, not of the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Checkpoint {
    pub(crate) label: &'static str,
    pub(crate) snapshot: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const fn at(label: &'static str, snapshot: usize) -> Checkpoint {
    Checkpoint { label, snapshot }
}

const WALKTHROUGH: [Checkpoint; 4] = [at("Full", 24), at("Blocked", 26), at("Released", 27), at("Empty", 45)];
const CONTENTION: [Checkpoint; 3] = [at("Both ready", 25), at("Last slot taken", 26), at("Re-enabled", 27)];
const INTERLEAVING: [Checkpoint; 5] = [
    at("Two busy", 2),
    at("Mixed work", 8),
    at("Two processing", 11),
    at("Three processing", 14),
    at("Home again", 30),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scenario {
    pub(crate) const ALL: [Self; 3] = [Self::Walkthrough, Self::Contention, Self::Interleaving];

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the scenario is called in the page's `data-pn-scenario` and in a selector.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Walkthrough => "walkthrough",
            Self::Contention => "contention",
            Self::Interleaving => "interleaving",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What a person reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Walkthrough => "Walkthrough: fill, block, release, drain",
            Self::Contention => "Contention: two producers, last free slot",
            Self::Interleaving => "Interleaving: overlapping worker states",
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The named checkpoints, which are the only ones there are.
    pub(crate) fn checkpoints(self) -> &'static [Checkpoint] {
        match self {
            Self::Walkthrough => &WALKTHROUGH,
            Self::Contention => &CONTENTION,
            Self::Interleaving => &INTERLEAVING,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The transitions to fire, in order.
    pub(crate) fn events(self) -> Vec<TransitionId> {
        let mut out = Vec::new();
        let triple = |out: &mut Vec<TransitionId>, i: usize| {
            out.extend([TransitionId::start(i), TransitionId::finish(i), TransitionId::put(i)]);
        };
        match self {
            Self::Walkthrough => {
                for k in 0..8 {
                    triple(&mut out, k % 4);
                }
                out.extend([TransitionId::start(0), TransitionId::finish(0)]);
                out.extend([TransitionId::take(0), TransitionId::put(0), TransitionId::complete(0)]);
                for k in 0..8 {
                    out.extend([TransitionId::take(k % 3), TransitionId::complete(k % 3)]);
                }
                triple(&mut out, 1);
                out.extend([TransitionId::take(1), TransitionId::complete(1)]);
            },
            Self::Contention => {
                for k in 0..7 {
                    triple(&mut out, k % 4);
                }
                out.extend([TransitionId::start(0), TransitionId::finish(0)]);
                out.extend([TransitionId::start(1), TransitionId::finish(1)]);
                out.extend([TransitionId::put(0), TransitionId::take(0), TransitionId::put(1)]);
            },
            Self::Interleaving => {
                use TransitionId as T;
                out.extend([
                    T::start(0),
                    T::start(1),
                    T::start(2),
                    T::finish(0),
                    T::finish(1),
                    T::put(0),
                    T::take(0),
                    T::start(3),
                    T::finish(2),
                    T::put(1),
                    T::take(1),
                    T::finish(3),
                    T::put(2),
                    T::take(2),
                    T::put(3),
                    T::complete(1),
                    T::start(0),
                    T::start(1),
                    T::finish(0),
                    T::finish(1),
                    T::put(1),
                    T::take(1),
                    T::put(0),
                    T::complete(0),
                    T::complete(2),
                    T::take(0),
                    T::complete(1),
                    T::take(2),
                    T::complete(0),
                    T::complete(2),
                ]);
            },
        }
        out
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The trace, which comes from firing [`Scenario::events`] through `net`.
    ///
    /// # Errors
    ///
    /// Returns an error only if a script is wrong, which a test rules out.
    pub(crate) fn trace(self, net: &Net) -> Result<Trace, TraceError> {
        Trace::run(net, &self.events())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
