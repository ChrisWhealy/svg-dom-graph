//! One slice, as it is recorded for comparison.

use super::PopRecord;

/// One slice, as it is recorded for comparison.
#[derive(Debug, Clone)]
pub struct SliceRecord {
    pub index: u64,
    pub tick: u64,
    pub pops: Vec<PopRecord>,
    pub waiting: usize,
    pub busy: usize,
    pub checkpoint: String,
}
