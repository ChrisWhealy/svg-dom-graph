//! Drawing once for a burst of moves, and taking the hooks off when the page is left.

use super::*;

// Frames.

#[test]
fn a_burst_of_range_moves_draws_once_at_the_next_frame_and_says_nothing_until_it_rests() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "restart")?;
    let said = live(&tab)?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ms-seek'); for (const v of ['30', '90', '200']) { r.value = v; \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq((pending_frames(&tab)?, step(&tab)?), (1, 0))?;
    check_eq(advance_frames(&tab, 5)?, 1)?;
    check_eq((step(&tab)?, pending_frames(&tab)?), (200, 0))?;
    check_eq(live(&tab)?, said)?;
    text(
        &tab,
        "(() => { document.getElementById('ms-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Cell ("), &live(&tab)?)
}

#[test]
fn letting_go_draws_now_and_the_frame_that_was_waiting_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "restart")?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ms-seek'); r.value = '120'; \
         r.dispatchEvent(new Event('input', { bubbles: true })); r.value = '60'; \
         r.dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check_eq((step(&tab)?, pending_frames(&tab)?), (60, 0))?;
    check_eq(advance_frames(&tab, 3)?, 0)?;
    check_eq(step(&tab)?, 60)
}

#[test]
fn the_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__msAdvanceFrame")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(
        text(&tab, "typeof window.__msAdvanceFrame + typeof window.__msPendingFrames")?,
        "undefinedundefined".to_owned(),
    )?;
    click_button(&tab, "restart")?;
    check_eq(step(&tab)?, CELLS)
}
