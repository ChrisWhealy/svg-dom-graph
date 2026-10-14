//! The walk through pass one: which step the demo is on, and what the scene shows at it. Pure, so none of it needs a
//! browser.
//!
//! One flat position counts every step, so a selection toolbar can step through all of them. With `M` merges and `N`
//! distinct bytes, the steps are:
//!
//! | Positions | Step |
//! |---|---|
//! | `0` | The bytes are counted, and each is a tree of one node. |
//! | `1..=M` | Merge `n`: the two lightest trees become one. |
//! | `M + 1` | Each byte's code length is read from the depth of its leaf. |
//! | `M + 2..=M + N + 1` | Canonical code `n`, one for each byte in canonical order. |
//!
//! The scene is drawn again at every position, from [`shown`] alone. So stepping backwards is the same work as stepping
//! forwards, and nothing needs undoing.

mod layout;
mod shown;

pub(super) use layout::{Spacing, layout};
pub(super) use shown::{Row, Shown, shown};

use super::algorithm::Trace;

/// How many positions the walk has: the counting step, every merge, the lengths, and one step for each canonical code.
pub(super) fn steps(trace: &Trace) -> usize {
    trace.tree.merges.len() + trace.codebook.entries.len() + 2
}

/// A leaf's label: its byte and its count, as in `e x8`. The multiplication sign would read better, but a plain text
/// value in a data node is printable ASCII only.
pub(super) fn leaf_label(symbol: u8, weight: u32) -> String {
    format!("{} x{weight}", symbol_label(symbol))
}

/// A byte as the scene writes it inside Markdown, which reads `*`, `_`, a backtick and a backslash as formatting. Each
/// is escaped so that it is drawn as itself.
pub(super) fn symbol_markdown(symbol: u8) -> String {
    let label = symbol_label(symbol);
    if matches!(label.as_str(), "*" | "_" | "`" | "\\") {
        format!("\\{label}")
    } else {
        label
    }
}

/// A byte as the scene writes it: the character if it is printable ASCII, `space` for a space, and hexadecimal
/// otherwise. A byte of a multi-byte character is never printable on its own, so it is always shown in hexadecimal.
pub(super) fn symbol_label(symbol: u8) -> String {
    match symbol {
        b' ' => "space".to_owned(),
        0x21..=0x7e => char::from(symbol).to_string(),
        _ => format!("0x{symbol:02X}"),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
