//! The Bubble sort panel in the built demo gallery: the bars, the green and blue pair that is compared, the swap that
//! slides them past each other, the lilac bar at the end of each pass, the buttons that choose another array, and the
//! stepping toolbar. The drawing, the buttons and the toolbar are the shared ones: see [`crate::sorting`]. What is
//! particular to Bubble sort is which bars are marked at each step, and how many steps a sorted array and one in
//! reverse take.
//!
//! The algorithm itself is covered by native unit tests: every arrangement of five elements ends sorted, every step
//! holds the same elements, and the swaps are exactly the pairs out of order. What they cannot reach is the browser.
//!
//! Like [`crate::quicksort`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

crate::sorting::sort_demo_helpers!("#bubble-sort-diagram");

const VIEWPORT: (u32, u32) = (1800, 1800);
/// The array the panel opens on, and its elements in order.
const INITIAL: [u8; 10] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-bubble-sort",
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
