//! Jumps, the range, the announcements, play and pause, and leaving the page.

use super::*;

// The controls.

#[test]
fn the_three_jumps_land_just_after_their_event_and_the_construction_left_rotation_is_the_first() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    let a = position(&tab)?;
    check(field(&tab, "event")?.contains("Right-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-1")?;
    let b = position(&tab)?;
    check(field(&tab, "event")?.contains("Left-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-2")?;
    let c = position(&tab)?;
    check(field(&tab, "event")?.contains("successor"), &field(&tab, "event")?)?;
    check(a < b && b < c, "in order")?;
    check(live(&tab)?.starts_with(&format!("Step {c} of")), &live(&tab)?)
}

#[test]
fn the_range_goes_to_any_step_and_says_where_it_rests_once() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('rbt-seek'); for (const v of ['3', '9', '12']) { r.value = v; \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq(position(&tab)?, 12)?;
    check_eq(live(&tab)?, String::new())?;
    text(
        &tab,
        "(() => { document.getElementById('rbt-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Step 12 of"), &live(&tab)?)
}

#[test]
fn a_transitional_step_says_repair_in_progress_and_a_finished_one_says_it_is_valid() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_button(&tab, "next")?;
    check(
        live(&tab)?.contains("Repair in progress, not yet a valid red-black tree"),
        &live(&tab)?,
    )?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    click_button(&tab, "previous")?;
    click_button(&tab, "next")?;
    check(!live(&tab)?.contains("Repair in progress"), &live(&tab)?)?;
    check(
        live(&tab)?.contains("Delete 15 complete: a valid red-black tree of 5 keys, root 5"),
        &live(&tab)?,
    )
}

// Play.

#[test]
fn play_runs_one_tick_at_a_time_and_announces_only_the_big_moves() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    check_eq(
        (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
        (1, "Pause".to_owned()),
    )?;
    check_eq(attribute(&tab, "[data-action=\"play\"]", "aria-label")?, "Pause".to_owned())?;
    check_eq(advance(&tab, 1)?, 1)?;
    check_eq(position(&tab)?, 1)?;
    check_eq(live(&tab)?, String::new())?; // an insertion event is a small move
    let last = last_step(&tab)?;
    check_eq(advance(&tab, 1000)?, last - 1)?;
    check_eq((position(&tab)?, waiting(&tab)?), (last, 0))?;
    check_eq(content(&tab, "[data-action=\"play\"]")?, "Play".to_owned())?;
    check(live(&tab)?.contains("Delete 15 complete"), &live(&tab)?)
}

#[test]
fn pausing_says_where_the_run_is_and_a_stale_tick_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    advance(&tab, 1)?;
    click_button(&tab, "play")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 1))?;
    check(live(&tab)?.starts_with("Step 1 of"), &live(&tab)?)?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 1)
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    for action in ["next", "previous", "restart", "jump-0"] {
        let tab = open_manual()?;
        seek(&tab, 4)?;
        click_button(&tab, "play")?;
        check_eq(waiting(&tab)?, 1)?;
        click_button(&tab, action)?;
        check_eq(
            (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
            (0, "Play".to_owned()),
        )
        .map_err(|e| format!("{action}: {e}"))?;
    }
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    seek(&tab, 7)?;
    check_eq(waiting(&tab)?, 0)
}

#[test]
fn the_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__rbtAdvance")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(
        text(&tab, "typeof window.__rbtAdvance + typeof window.__rbtPending")?,
        "undefinedundefined".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 0)
}

// Pan and zoom.

#[test]
fn pan_and_zoom_are_off_so_the_picture_cannot_leave_the_tree() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let before = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    text(
        &tab,
        &format!(
            "(() => {{ const s = document.querySelector('{STAGE} .rbt-scene'); \
             s.dispatchEvent(new WheelEvent('wheel', {{ deltaY: -400, ctrlKey: true, bubbles: true, cancelable: true }})); \
             return 'ok'; }})()"
        ),
    )?;
    let after = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    check_eq(after, before)?;
    check_eq(overlay_error(&tab)? < 0.6, true)?;
    check_eq(count(&tab, &format!("{STAGE} [role=application]"))?, 0)
}
