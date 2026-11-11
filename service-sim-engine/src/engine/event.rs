//! The heap orders events by `(tick, rank, sequence)`, which is unique since the sequence is.

use super::{Kind, Payload};

#[derive(Debug, Clone)]
pub(super) struct Event {
    pub(super) tick: u64,
    pub(super) rank: u8,
    pub(super) seq: u64,
    pub(super) kind: Kind,
    pub(super) payload: Payload,
}

/// The heap orders events by `(tick, rank, sequence)`, which is unique since the sequence is.
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        (self.tick, self.rank, self.seq) == (other.tick, other.rank, other.seq)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.tick, self.rank, self.seq).cmp(&(other.tick, other.rank, other.seq))
    }
}
