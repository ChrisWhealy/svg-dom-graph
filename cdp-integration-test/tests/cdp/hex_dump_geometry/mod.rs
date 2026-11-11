//! Where the marks over a hex dump's bytes sit, with and without the offsets, and how little a change to a set touches.
//!
//! These run against the fixture, which has the same 65 bytes as two dumps, `dump_off` and `dump_on`, and a large dump,
//! `dump_big`, with every byte unreached.

use crate::common::{ascii_column, dump_marks, hex_column, new_tab};

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

mod unit_tests;
