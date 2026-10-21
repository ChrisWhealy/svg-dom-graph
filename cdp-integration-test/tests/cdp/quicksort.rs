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

use crate::common::{DemoTab, new_demo_tab};
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on ten bars, in the order of the array and not sorted, with three buttons and the walk at its start.
/// They stand on one baseline, are thin, and are as tall as their values, in proportion. None is marked.
#[test]
fn the_panel_opens_on_ten_bars_as_tall_as_their_values() -> Result<(), String> {
    let tab = open()?;
    let bars = bars(&tab)?;
    check_eq(bars.iter().map(|b| b.value).collect::<Vec<_>>(), INITIAL.to_vec())?;
    for action in ["random", "sorted", "reversed"] {
        check_eq(
            text(
                &tab,
                &format!("String(document.querySelectorAll('{DIAGRAM} g[data-action=\"{action}\"]').length)"),
            )?
            .as_str(),
            "1",
        )?;
    }
    let baseline = bars[0].bottom;
    let per_unit = bars[0].height / f64::from(bars[0].value);
    for bar in &bars {
        check(
            (bar.bottom - baseline).abs() < 0.5,
            &format!("{bar:?} is not on the baseline {baseline}"),
        )?;
        check(
            (bar.height / f64::from(bar.value) - per_unit).abs() < 0.05,
            &format!("{bar:?} is not as tall as its value, in proportion"),
        )?;
        check(bar.fill == bars[0].fill, "no bar is marked at the start")?;
    }
    check(bars.windows(2).all(|w| w[0].left < w[1].left), "the bars should be in a row")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(said(&tab)?.starts_with("Step 1 of "), "the first step should be announced")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The second step chooses the first range and its pivot, which is the last element. The pivot's bar is red and no
/// other.
#[test]
fn the_pivot_is_the_one_red_bar() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 1)?;
    check_eq(with_fill(&tab, RED)?, vec![9])?;
    check_eq(values(&tab)?, INITIAL.to_vec())?;
    check(with_fill(&tab, GREEN)?.is_empty(), "no bar is being examined yet")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The third step looks at the first bar, which is green. It is smaller than the pivot and already at the front, so
/// nothing moves.
#[test]
fn the_bar_being_examined_is_green() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    check_eq(with_fill(&tab, GREEN)?, vec![0])?;
    check_eq(with_fill(&tab, RED)?, vec![9])?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sixth step looks at 29, which is smaller than the pivot 56, and must go to position 2, where 83 is. It does not
/// move yet: 29 is green and 83 is blue, and the bars are where they were. The two smaller bars already gathered at the
/// front are teal, and the pivot is still red.
#[test]
fn the_bar_to_swap_with_is_blue_before_anything_moves() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 5)?;
    check_eq(values(&tab)?, INITIAL.to_vec())?;
    check_eq(with_fill(&tab, GREEN)?, vec![3])?;
    check_eq(with_fill(&tab, BLUE)?, vec![2])?;
    check_eq(with_fill(&tab, TEAL)?, vec![0, 1])?;
    check_eq(with_fill(&tab, RED)?, vec![9])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The next step is the swap: 29 and 83 have changed places, the green bar went with 29 and the pivot is still red.
/// Nothing is blue, since nothing is waiting. The two bars slid to their new places: each carries a SMIL animation.
#[test]
fn a_swap_is_a_step_of_its_own_and_the_two_bars_slide() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 5)?;
    check_eq(slides(&tab)?, 0)?;
    goto(&tab, 6)?;
    check_eq(values(&tab)?, vec![47, 12, 29, 83, 61, 18, 95, 34, 72, 56])?;
    check_eq(with_fill(&tab, GREEN)?, vec![2])?;
    check(with_fill(&tab, BLUE)?.is_empty(), "nothing is waiting for a swap now")?;
    check_eq(with_fill(&tab, RED)?, vec![9])?;
    check_eq(slides(&tab)?, 2)?;
    // Each slides from the other's place: the animations go in opposite directions, from a place to a different one.
    let json = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} animateTransform')] \
             .map(a => [a.getAttribute('from'), a.getAttribute('to')]))"
        ),
    )?;
    let pairs: Vec<(String, String)> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    check_eq(pairs.len(), 2)?;
    check(
        pairs[0].0 != pairs[0].1 && pairs[1].0 != pairs[1].1,
        "a bar should slide from one place to another",
    )?;
    check(pairs[0].0 != pairs[1].0, "the two bars start in different places")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back over a swap draws the bars where they are, without a slide, since no one saw them where they were. So
/// does jumping to a step with another array.
#[test]
fn stepping_back_over_a_swap_does_not_slide() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 6)?;
    check_eq(slides(&tab)?, 2)?;
    goto(&tab, 5)?;
    check_eq(slides(&tab)?, 0)?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At the last step the bars are in order, from the shortest to the tallest, and every one is lilac.
#[test]
fn the_last_step_shows_the_bars_in_order_and_all_in_place() -> Result<(), String> {
    let tab = open()?;
    let last = step_count(&tab)? - 1;
    goto(&tab, last)?;
    let mut expected = INITIAL;
    expected.sort_unstable();
    check_eq(values(&tab)?, expected.to_vec())?;
    check_eq(with_fill(&tab, LILAC)?, (0..10).collect::<Vec<_>>())?;
    check(said(&tab)?.contains("sorted"), "the last step should say that it is sorted")
}

/// Once the first pivot is placed and the range before it is chosen, the bars outside that range are pale, and the
/// pivot, in its final place, is lilac.
#[test]
fn bars_outside_the_range_are_pale_and_a_placed_pivot_is_lilac() -> Result<(), String> {
    let tab = open()?;
    // The elements smaller than 56 are five, so the pivot goes to position 5 and the range before it is 0 to 4.
    goto_note(&tab, "Sort positions 0 to 4")?;
    check_eq(with_fill(&tab, PALE)?, vec![6, 7, 8, 9])?;
    check_eq(with_fill(&tab, LILAC)?, vec![5])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step is announced in the live region outside the scene, with how far along the walk is.
#[test]
fn each_step_is_announced_with_how_far_along_it_is() -> Result<(), String> {
    let tab = open()?;
    let count = step_count(&tab)?;
    for position in 1..6 {
        goto(&tab, position)?;
        let said = said(&tab)?;
        check(
            said.starts_with(&format!("Step {} of {count}. ", position + 1)),
            &format!("position {position} announced {said:?}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Already sorted" gives bars in order from the shortest to the tallest, and says so in the live region before the
/// first step. Its walk is the longest, since a pivot that is always the tallest bar left removes only itself.
#[test]
fn the_sorted_button_gives_bars_in_order_with_the_longest_walk() -> Result<(), String> {
    let tab = open()?;
    let random_steps = step_count(&tab)?;
    press(&tab, "sorted")?;
    let values = values(&tab)?;
    check_eq(values.len(), 10)?;
    check(values.windows(2).all(|w| w[0] < w[1]), &format!("not in order: {values:?}"))?;
    check(said(&tab)?.starts_with("A new array: "), "the new array should be announced")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        step_count(&tab)? > random_steps,
        "the sorted array should take more steps than the random one",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Reverse sorted" gives bars from the tallest to the shortest.
#[test]
fn the_reverse_button_gives_bars_from_tallest_to_shortest() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "reversed")?;
    let values = values(&tab)?;
    check_eq(values.len(), 10)?;
    check(
        values.windows(2).all(|w| w[0] > w[1]),
        &format!("not in descending order: {values:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Random array" gives ten different bars, which are probably not the ones the panel opened on.
#[test]
fn the_random_button_gives_ten_different_bars() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "random")?;
    let mut values = values(&tab)?;
    check_eq(values.len(), 10)?;
    values.sort_unstable();
    values.dedup();
    check_eq(values.len(), 10)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Restart" goes back to the first step, with the bars as they were given and none marked.
#[test]
fn restart_goes_back_to_the_unsorted_bars() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 8)?;
    check(values(&tab)? != INITIAL.to_vec(), "eight steps should have moved something")?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check_eq(values(&tab)?, INITIAL.to_vec())?;
    check(with_fill(&tab, RED)?.is_empty(), "no pivot is chosen at the start")
}

/// The zoom stays through steps forward and back, a restart, and another array, since each draws the scene again.
#[test]
fn the_zoom_survives_every_kind_of_redraw() -> Result<(), String> {
    let tab = open()?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in should change the view")?;
    goto(&tab, 5)?;
    check_eq(view(&tab)?, zoomed.clone())?;
    goto(&tab, 2)?;
    click(&tab, "Restart selection")?;
    check_eq(view(&tab)?, zoomed.clone())?;
    press(&tab, "reversed")?;
    check_eq(view(&tab)?, zoomed)
}
