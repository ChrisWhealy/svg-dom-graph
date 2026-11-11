//! The Quicksort panel in the built demo gallery: the bars on a common baseline as tall as their values, the colours
//! that say what the algorithm is doing with each, the buttons that choose another array, the stepping toolbar that
//! walks through the sort, what each step announces, and what a redraw keeps.
//!
//! The algorithm itself is covered by native unit tests: every arrangement of six elements ends sorted, and every step
//! holds the same elements. What they cannot reach is the browser: that a step draws the bars where they belong, in the
//! right colours, that the buttons drawn in the scene work, and that the zoom survives the scene being drawn again for
//! each step.
//!
//! The bars, their colours and the buttons are the same for every sorting demo, so the functions that read them are
//! shared: see [`crate::sorting`].
//!
//! Like [`crate::hamming`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

crate::sorting::sort_demo_helpers!("#quicksort-diagram");

const VIEWPORT: (u32, u32) = (1800, 1800);
/// The array the panel opens on, and its elements in order.
const INITIAL: [u8; 10] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-quicksort",
        &format!("{DIAGRAM} g[data-action=\"random\"]"),
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

mod unit_tests;
