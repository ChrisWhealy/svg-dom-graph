//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The window opens with the nine boxes of the path and the controls, and the walk at its start.
#[test]
fn the_window_opens_with_the_path_the_controls_and_the_walk_at_the_start() -> Result<(), String> {
    let tab = open()?;
    for name in BOXES {
        check(
            !text(
                &tab,
                &format!("String(!document.querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]'))"),
            )?
            .starts_with("true"),
            &format!("{name} should be drawn"),
        )?;
    }
    for action in ["toggle-0", "toggle-1", "toggle-2", "toggle-3", "introduce", "remove"] {
        check_eq(
            text(
                &tab,
                &format!("String(document.querySelectorAll('{DIAGRAM} g[data-action=\"{action}\"]').length)"),
            )?
            .as_str(),
            "1",
        )?;
    }
    check_eq(step_position(&tab)?, Some(0))?;
    check(says(&tab, "No error has been introduced.")?, "no error to begin with")?;
    // Only the data is there to read.
    check(!faint(&tab, "Data bits")?, "the data is the input")?;
    for name in &BOXES[1..] {
        check(faint(&tab, name)?, &format!("{name} should not be reached yet"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path runs along a row, and back along the next: a block, not a line. The boxes of a row are level, the first row
/// runs left to right and the second right to left, and no two boxes overlap.
#[test]
fn the_path_snakes_into_a_block() -> Result<(), String> {
    let tab = open()?;
    let boxes = rects(&tab)?;
    let middle_y = |b: &[f64; 4]| (b[1] + b[3] / 2.0).round();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for (index, b) in boxes.iter().enumerate() {
        match rows.last_mut() {
            Some(row) if (middle_y(&boxes[row[0]]) - middle_y(b)).abs() < 4.0 => row.push(index),
            _ => rows.push(vec![index]),
        }
    }
    check(
        (2..=4).contains(&rows.len()),
        &format!("expected a block of 2 to 4 rows, got {}", rows.len()),
    )?;
    for (n, row) in rows.iter().enumerate() {
        let xs: Vec<f64> = row.iter().map(|&i| boxes[i][0]).collect();
        let forward = n % 2 == 0;
        check(
            xs.windows(2).all(|w| if forward { w[0] < w[1] } else { w[0] > w[1] }),
            &format!(
                "row {n} should run {}: {xs:?}",
                if forward { "left to right" } else { "right to left" }
            ),
        )?;
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[3] <= b[1] || b[1] + b[3] <= a[1];
            check(apart, "two boxes overlap")?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data bit's button flips it, says so in its name and its pressed state, and keeps the keyboard focus across the
/// redraw.
#[test]
fn a_data_bit_button_toggles_its_bit_and_keeps_the_focus() -> Result<(), String> {
    let tab = open()?;
    check_eq(label_of(&tab, "toggle-0")?.as_str(), "d1 = 1")?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"toggle-0\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "true",
    )?;
    press(&tab, "toggle-0")?;
    check_eq(label_of(&tab, "toggle-0")?.as_str(), "d1 = 0")?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"toggle-0\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "false",
    )?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "toggle-0",
    )?;
    check_eq(
        text(&tab, "document.getElementById('hamming-announcement').textContent")?.as_str(),
        "The data bits are now 0 0 1 1.",
    )?;
    // The other buttons are untouched.
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 0")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step reaches one more box of the path.
#[test]
fn each_step_reaches_the_next_box() -> Result<(), String> {
    let tab = open()?;
    for position in 1..=LAST {
        goto(&tab, position)?;
        for (index, name) in BOXES.iter().enumerate() {
            check_eq(faint(&tab, name)?, index > position)?;
        }
    }
    // Stepping back makes the later boxes faint again.
    goto(&tab, 2)?;
    check(
        faint(&tab, "Received codeword")?,
        "stepping back should make the receiver faint again",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flipping a bit moves the walk to the received codeword, and the receiver finds exactly that bit: the error position
/// at the end of the path is the position the status line says was flipped. The data is recovered whichever bit it was.
#[test]
fn the_receiver_finds_whichever_bit_was_flipped() -> Result<(), String> {
    let tab = open()?;
    // The choice is random, so try several times.
    for _ in 0..4 {
        press(&tab, "introduce")?;
        check_eq(step_position(&tab)?.map(|p| p.max(3)), step_position(&tab)?)?;
        let status = texts(&tab)?
            .join(" ")
            .split("The bit at position ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next().map(str::to_owned))
            .ok_or("the status line does not say which bit")?;
        goto(&tab, LAST)?;
        // The box draws its title first and its one value last.
        let found = text(
            &tab,
            &format!(
                "[...document.querySelector('{DIAGRAM} g[aria-label^=\"Error position\"]').querySelectorAll('text')].at(-1).textContent"
            ),
        )?;
        check_eq(found.as_str(), status.as_str())?;
        check(says(&tab, "They are the data that was sent.")?, "the data should be recovered")?;
        goto(&tab, 0)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flipping a bit moves a walk that is before the received codeword on to it, and never moves one that is past it.
/// Removing the error leaves the walk where it is.
#[test]
fn flipping_a_bit_moves_the_walk_to_the_received_codeword_but_never_back() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "introduce")?;
    check_eq(step_position(&tab)?, Some(3))?;
    check(!faint(&tab, "Received codeword")?, "the received codeword should be reached")?;
    goto(&tab, 6)?;
    press(&tab, "introduce")?;
    check_eq(step_position(&tab)?, Some(6))?;
    press(&tab, "remove")?;
    check_eq(step_position(&tab)?, Some(6))?;
    check(says(&tab, "No error has been introduced.")?, "the error should be gone")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" starts the walk again and removes the flipped bit, but keeps the data bits as they were set.
#[test]
fn restart_goes_back_to_the_start_and_removes_the_error_but_keeps_the_data() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-1")?;
    press(&tab, "introduce")?;
    goto(&tab, 5)?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    check(faint(&tab, "Received codeword")?, "the walk should be back at the start")?;
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 1")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines one test for each position listed, calling `$run` with it. Each is a test of its own, with its own tab, so
/// the CI deals them out to different shards, instead of one long test that walks every position in a single shard.
macro_rules! at_each_position {
    ($run:ident: $($name:ident => $position:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() -> Result<(), String> {
                $run($position)
            }
        )+
    };
}

at_each_position! { announces:
    the_walk_announces_step_2 => 1,
    the_walk_announces_step_3 => 2,
    the_walk_announces_step_4 => 3,
    the_walk_announces_step_5 => 4,
    the_walk_announces_step_6 => 5,
    the_walk_announces_step_7 => 6,
    the_walk_announces_step_8 => 7,
    the_walk_announces_step_9 => 8,
}

at_each_position! { notes_follow_the_walk:
    the_notes_at_position_0_follow_the_walk => 0,
    the_notes_at_position_1_follow_the_walk => 1,
    the_notes_at_position_2_follow_the_walk => 2,
    the_notes_at_position_3_follow_the_walk => 3,
    the_notes_at_position_4_follow_the_walk => 4,
    the_notes_at_position_5_follow_the_walk => 5,
    the_notes_at_position_6_follow_the_walk => 6,
    the_notes_at_position_7_follow_the_walk => 7,
    the_notes_at_position_8_follow_the_walk => 8,
}

at_each_position! { restart_from:
    restart_from_position_0_returns_to_the_start => 0,
    restart_from_position_1_returns_to_the_start => 1,
    restart_from_position_2_returns_to_the_start => 2,
    restart_from_position_3_returns_to_the_start => 3,
    restart_from_position_4_returns_to_the_start => 4,
    restart_from_position_5_returns_to_the_start => 5,
    restart_from_position_6_returns_to_the_start => 6,
    restart_from_position_7_returns_to_the_start => 7,
    restart_from_position_8_returns_to_the_start => 8,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Restart makes one announcement that says both that the error is gone and that the walk is back at the first step.
/// Two announcements made together would leave a screen reader saying only the second.
#[test]
fn restart_announces_the_removed_error_and_the_first_step_together() -> Result<(), String> {
    let tab = open_with("?error=3")?;
    goto(&tab, 5)?;
    click(&tab, "Restart selection")?;
    let said = text(&tab, "document.getElementById('hamming-announcement').textContent")?;
    check(
        said.contains("No error has been introduced.") && said.contains("Step 1 of 9: Data bits"),
        &format!("restart announced {said:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Previous selection" at position 0 has nowhere earlier to go, and the toolbar reports it exactly as it reports
/// "Restart". So the walk stays at the start, a flipped bit is removed as a restart removes it, the data stays, and
/// nothing fails.
#[test]
fn previous_at_position_zero_is_a_restart() -> Result<(), String> {
    let tab = open_with("?error=5")?;
    check(
        says(&tab, "The bit at position 5 is flipped in transmission.")?,
        "the page should open with a flipped bit",
    )?;
    click_and_settle(&tab, "Previous selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "stepping back from the start restarts, which removes the flipped bit",
    )?;
    check(!faint(&tab, "Data bits")?, "the data should still be there")?;
    check(faint(&tab, "Parity bits")?, "nothing else should be reached")?;
    check_eq(
        text(&tab, "String(document.getElementById('hamming-window-error').hidden)")?.as_str(),
        "true",
    )
}

at_each_position! { finds_error_at:
    the_receiver_finds_an_error_at_position_1 => 1,
    the_receiver_finds_an_error_at_position_2 => 2,
    the_receiver_finds_an_error_at_position_3 => 3,
    the_receiver_finds_an_error_at_position_4 => 4,
    the_receiver_finds_an_error_at_position_5 => 5,
    the_receiver_finds_an_error_at_position_6 => 6,
    the_receiver_finds_an_error_at_position_7 => 7,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back from a box that shows the error to one before it takes the mark away, since a mark is drawn at full
/// strength even in a faint box, and would give away what the walk has not reached.
#[test]
fn stepping_back_takes_the_error_marks_away_from_boxes_not_reached() -> Result<(), String> {
    let tab = open_with("?error=5")?;
    goto(&tab, LAST)?;
    check(
        box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "marked once reached",
    )?;
    check(
        box_label(&tab, "Corrected codeword")?.contains("also highlighted"),
        "marked once reached",
    )?;
    goto(&tab, 6)?;
    check(
        !box_label(&tab, "Corrected codeword")?.contains("also highlighted"),
        "the corrected codeword is not reached, so shows no mark",
    )?;
    check(
        box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "the received codeword is reached, so keeps its mark",
    )?;
    goto(&tab, 2)?;
    check(
        !box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "the received codeword is not reached, so shows no mark",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom stays through redraws of every kind: a data bit changed, an error introduced and removed, a step forward
/// and back, and a restart.
#[test]
fn the_zoom_survives_every_kind_of_redraw() -> Result<(), String> {
    let tab = open()?;
    click_and_settle(&tab, "Zoom in")?;
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in should change the view")?;
    press(&tab, "toggle-0")?;
    press(&tab, "introduce")?;
    press(&tab, "remove")?;
    goto(&tab, 5)?;
    goto(&tab, 1)?;
    click(&tab, "Restart selection")?;
    check_eq(view(&tab)?, zoomed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The error banner stays hidden while redraws work.
#[test]
fn the_error_banner_stays_hidden_while_redraws_work() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-2")?;
    press(&tab, "introduce")?;
    goto(&tab, 4)?;
    check_eq(
        text(&tab, "(() => { const b = document.getElementById('hamming-window-error'); return b.hidden + '|' + b.textContent; })()")?
            .as_str(),
        "true|",
    )
}
