#![cfg(test)]

//! Shared helpers for this crate's own internal `#[cfg(test)]` unit tests — `error::unit_tests` and
//! `scene::drag::unit_tests`.
//!
//! Not reachable from the external `tests/drag/` integration test binary: that binary depends on this crate as
//! any external consumer would, so it only sees the public API, and `#[cfg(test)]` code is not part of a normal
//! (non-`--test`) build in the first place. `tests/drag/common.rs` keeps its own copy of [`check`] for that
//! reason, not because the duplication was overlooked.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns `Err(msg)` when `condition` is `false` — every test in this crate, internal or external, follows the
/// same `Result<(), String>` convention: a failure prints its message directly, with no panic and no stack trace.
pub(crate) fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.into()) }
}
