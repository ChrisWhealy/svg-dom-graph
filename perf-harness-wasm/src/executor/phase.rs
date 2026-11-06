//! What the harness is doing, set on the allocator so that a block carries it.
//!
//! It is how the bytes that stay allocated after a teardown are traced back to the phase that made them. Setting one costs a
//! store, and it is never set inside a timed interval.

pub const COLLECT: usize = 1;
/// Working out what an operation needs.
pub const PREPARE: usize = 2;
/// The library calls themselves.
pub const LIBRARY: usize = 3;
/// Keeping what the calls returned and the time they took.
pub const BOOKKEEPING: usize = 4;
/// Counting resources.
pub const SAMPLE: usize = 5;
/// Reading the document and comparing it with the model.
pub const CHECK: usize = 6;

pub fn name(tag: usize) -> String {
    let grown = tag & crate::alloc::GROWN_FLAG != 0;
    let base = match tag & !crate::alloc::GROWN_FLAG {
        COLLECT => "model, stream hash and delay",
        PREPARE => "preparing an operation",
        LIBRARY => "library calls",
        BOOKKEEPING => "keeping what the calls returned",
        SAMPLE => "counting resources",
        CHECK => "reading the document and comparing",
        _ => "other",
    };
    if grown {
        format!("a block from before the baseline that grew, during: {base}")
    } else {
        base.to_string()
    }
}
