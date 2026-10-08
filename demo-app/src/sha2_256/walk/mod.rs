//! The walk through the hash: which step the demo is on, and what every node shows at it. Pure, so none of it needs a
//! browser.
//!
//! One flat position counts every step, so a selection toolbar can step through all of them. The steps are:
//!
//! | Positions | Step |
//! |---|---|
//! | `0` | The plain text message. |
//! | `1` | The message block: the message, the terminator byte and the length. |
//! | `2` | The block copied into the first 16 words of the message schedule. |
//! | `3..=50` | Schedule word `n`, one for each `n` from 16 to 63. |
//! | `51..=114` | Round `i`, one for each `i` from 0 to 63. |
//! | `115` | The digest: the working variables added to the initial hash. |
//!
//! Anything the walk has not reached yet shows zeros, the same "not yet written" convention the SHA3 demo uses.

mod phase;
mod ring;
mod shown;

use phase::{Phase, phase};
pub(super) use ring::Ring;
pub(super) use shown::{Shown, shown};

use super::algorithm::{BLOCK_WORDS, ROUNDS, Round, SCHEDULE_WORDS, Trace};

/// The one message this demo hashes.
pub(super) const MESSAGE: &[u8] = b"The quick brown fox jumps over the lazy dog";

/// The first position that builds a schedule word beyond the block's own 16.
const EXPAND_START: usize = 3;
/// The first position that runs a round.
const ROUND_START: usize = EXPAND_START + (SCHEDULE_WORDS - BLOCK_WORDS);
/// The position that adds the working variables to the initial hash.
const FINAL: usize = ROUND_START + ROUNDS;
/// How many positions the walk has.
pub(super) const STEPS: usize = FINAL + 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
