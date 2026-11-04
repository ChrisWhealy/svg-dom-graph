//! The history of a run: one snapshot for the starting marking and one more for each event. Pure.
//!
//! A snapshot is authoritative. What the page highlights and says is worked out from it and is never kept. The counters
//! of Puts, Takes and Completes belong to the history and not to the net, and they let a test audit the buffer.

use super::net::{FireError, Marking, Net, TransitionId, Violation};

/// The most events that a history can hold. A firing that would pass it is refused before anything changes.
pub(crate) const MAX_EVENTS: usize = 10_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many times each audited transition has fired since the history began.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Totals {
    pub(crate) puts: u64,
    pub(crate) takes: u64,
    pub(crate) completes: u64,
}

impl Totals {
    /// The counters after `t` has fired.
    fn after(self, t: TransitionId) -> Self {
        use super::net::TransitionKind::{Complete, Put, Take};
        let mut next = self;
        match t.kind() {
            Put => next.puts += 1,
            Take => next.takes += 1,
            Complete => next.completes += 1,
            _ => {},
        }
        next
    }
}

/// The transition that fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Event {
    pub(crate) transition: TransitionId,
}

/// A marking, the counters at that point, and the event that led to it, which the starting snapshot lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) marking: Marking,
    pub(crate) totals: Totals,
    pub(crate) last: Option<Event>,
}

impl Snapshot {
    /// The snapshot at the start of a script: the initial marking and zero counters.
    pub(crate) fn initial() -> Self {
        Self {
            marking: Marking::initial(),
            totals: Totals::default(),
            last: None,
        }
    }

    /// Checks the marking, and the two equations that tie the counters to it.
    ///
    /// # Errors
    ///
    /// Returns the first rule that is broken.
    pub(crate) fn check(&self) -> Result<(), Violation> {
        self.marking.check()?;
        if self.totals.puts.checked_sub(self.totals.takes) != Some(u64::from(self.marking.used())) {
            return Err(Violation::BufferAccounting);
        }
        if self.totals.takes.checked_sub(self.totals.completes) != Some(self.marking.processing() as u64) {
            return Err(Violation::ProcessingAccounting);
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why an event could not be added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TraceError {
    /// The transition could not fire.
    Fire(FireError),
    /// The next snapshot broke a rule that ties the counters to the marking.
    Invariant(Violation),
    /// The history already holds [`MAX_EVENTS`] events.
    Budget,
}

impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fire(e) => write!(f, "{e}"),
            Self::Invariant(v) => write!(f, "{v}"),
            Self::Budget => write!(f, "the history is full at {MAX_EVENTS} events"),
        }
    }
}

impl std::error::Error for TraceError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The snapshots of a run, in order. There is always at least one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trace {
    snapshots: Vec<Snapshot>,
}

impl Trace {
    /// A history that begins at `first`.
    pub(crate) fn new(first: Snapshot) -> Self {
        Self { snapshots: vec![first] }
    }

    /// The history that fires `events` one after another from the initial snapshot.
    ///
    /// # Errors
    ///
    /// Returns the error of the first event that could not be added.
    pub(crate) fn run(net: &Net, events: &[TransitionId]) -> Result<Self, TraceError> {
        let mut trace = Self::new(Snapshot::initial());
        for &t in events {
            trace.push(net, t)?;
        }
        Ok(trace)
    }

    /// The index of the last snapshot, which is the number of events.
    pub(crate) fn last(&self) -> usize {
        self.snapshots.len() - 1
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub(crate) fn snapshot(&self, index: usize) -> Option<&Snapshot> {
        self.snapshots.get(index)
    }

    #[cfg(test)]
    pub(crate) fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }

    /// The snapshot before the one at `index`, which there is not for the first.
    pub(crate) fn before(&self, index: usize) -> Option<&Snapshot> {
        index.checked_sub(1).and_then(|at| self.snapshots.get(at))
    }

    /// Fires `t` at the last snapshot and adds the result. Nothing is added if the firing fails.
    ///
    /// # Errors
    ///
    /// Returns why the event was not added. The budget is checked before anything else.
    pub(crate) fn push(&mut self, net: &Net, t: TransitionId) -> Result<(), TraceError> {
        if self.last() >= MAX_EVENTS {
            return Err(TraceError::Budget);
        }
        let current = self.snapshots[self.last()];
        let next = Snapshot {
            marking: net.fire(&current.marking, t).map_err(TraceError::Fire)?,
            totals: current.totals.after(t),
            last: Some(Event { transition: t }),
        };
        next.check().map_err(TraceError::Invariant)?;
        self.snapshots.push(next);
        Ok(())
    }

    /// Drops every snapshot after `index`.
    pub(crate) fn truncate_after(&mut self, index: usize) {
        self.snapshots.truncate(index + 1);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A history that a person grows by firing transitions, started from a snapshot of a script.
///
/// Its first snapshot is index 0 of its own, a different coordinate space from the script's. The cursor is the snapshot
/// that is shown, and the tip is the last one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Branch {
    trace: Trace,
    cursor: usize,
}

impl Branch {
    /// A branch whose first snapshot is a copy of `source`, counters included, and whose event list is empty.
    pub(crate) fn fork(source: &Snapshot) -> Self {
        Self {
            trace: Trace::new(Snapshot { last: None, ..*source }),
            cursor: 0,
        }
    }

    pub(crate) fn trace(&self) -> &Trace {
        &self.trace
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.trace.last());
    }

    /// The history that results from firing `t` at the cursor: the snapshots after the cursor are dropped and the new
    /// one follows it. The branch itself is not changed, so that a page can draw the candidate before it is kept.
    ///
    /// # Errors
    ///
    /// Returns why the event could not be added. The budget is checked against the shortened history.
    pub(crate) fn candidate(&self, net: &Net, t: TransitionId) -> Result<Self, TraceError> {
        let mut next = self.clone();
        next.trace.truncate_after(next.cursor);
        next.trace.push(net, t)?;
        next.cursor = next.trace.last();
        Ok(next)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
