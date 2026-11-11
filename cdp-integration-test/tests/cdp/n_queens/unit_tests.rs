//! The tests of [`super`], which holds the helpers they use.

use super::*;

// The board.

#[test]
fn the_board_is_a_grid_of_squares_with_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [role=grid]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} [role=row]"))?, 8)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"0\"]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"-1\"]"))?, 63)?;
    check_eq(count(&tab, "#nq-r3-c5")?, 1)?;
    check_eq(cell_label(&tab, 3, 5)?, "empty".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} .nq-fills rect"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} .nq-glyphs text"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} .nq-rays path"))?, 3)?;
    check_eq(queens(&tab)?, 0)
}

#[test]
fn the_layers_are_in_order_with_the_focus_ring_on_top() -> Result<(), String> {
    let tab = open()?;
    let order = text(
        &tab,
        &format!(
            "Array.from(document.querySelector('{SVG} .nq-board').children).map(c => c.getAttribute('class')).join(' ')"
        ),
    )?;
    let layers: Vec<&str> = order
        .split(' ')
        .filter(|c| c.starts_with("nq-") && !c.starts_with("nq-off"))
        .collect();
    check_eq(
        layers,
        vec![
            "nq-fills", "nq-rays", "nq-glyphs", "nq-outline", "nq-frame", "nq-grid", "nq-ring",
        ],
    )?;
    check_eq(attribute(&tab, &format!("{SVG} .nq-rays"), "aria-hidden")?, "true".to_owned())?;
    // Rays and outline never take the pointer or the focus.
    let hit = text(
        &tab,
        "(() => { const r = document.getElementById('nq-r2-c2').getBoundingClientRect(); \
         const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return e.id; })()",
    )?;
    check_eq(hit, "nq-r2-c2".to_owned())
}

#[test]
fn the_page_opens_at_the_empty_board_paused() -> Result<(), String> {
    let tab = open()?;
    check_eq(field(&tab, "position")?, "0/33589".to_owned())?;
    check_eq(field(&tab, "kind")?, "Initial".to_owned())?;
    check_eq(field(&tab, "depth")?, "0 / 8".to_owned())?;
    check_eq(field(&tab, "solutions")?, "0 / 92".to_owned())?;
    check_eq(field(&tab, "playback")?, "Paused at 20 events per second".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, "33589".to_owned())?;
    check(
        content(&tab, "#n-queens-board-info")?.contains("KiB"),
        "the page says what the trace takes",
    )
}

// What each step shows.

#[test]
fn a_try_outlines_the_square_and_draws_three_rays_a_place_draws_a_queen() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Try".to_owned())?;
    check_eq(field(&tab, "cell")?, "row 1, column 1".to_owned())?;
    check(
        outline_class(&tab)?.contains("nq-candidate"),
        "the square is outlined as a candidate",
    )?;
    check_eq(cell_label(&tab, 0, 0)?, "candidate".to_owned())?;
    let drawn = rays(&tab)?;
    check(
        drawn.iter().all(|(class, d)| class == "nq-ray" && !d.is_empty()),
        "three neutral rays",
    )?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Place".to_owned())?;
    check_eq(queens(&tab)?, 1)?;
    check_eq(cell_label(&tab, 0, 0)?, "queen".to_owned())?;
    check(outline_class(&tab)?.contains("nq-off"), "no outline")?;
    check(rays(&tab)?.iter().all(|(class, _)| class.contains("nq-off")), "no rays")
}

#[test]
fn a_reject_marks_the_square_and_makes_the_reason_the_heavy_ray() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 3)?;
    let try_rays = rays(&tab)?;
    seek(&tab, 4)?;
    check_eq(field(&tab, "kind")?, "Reject".to_owned())?;
    check_eq(field(&tab, "reason")?, "column".to_owned())?;
    check(outline_class(&tab)?.contains("nq-rejected"), "a rejected outline")?;
    check_eq(cell_label(&tab, 1, 0)?, "rejected: column".to_owned())?;
    let reject_rays = rays(&tab)?;
    check_eq(reject_rays[0].0.clone(), "nq-ray nq-chosen".to_owned())?;
    check_eq(reject_rays[1].0.clone(), "nq-ray nq-muted".to_owned())?;
    check_eq(reject_rays[2].0.clone(), "nq-ray nq-muted".to_owned())?;
    let same_lines = try_rays.iter().zip(&reject_rays).all(|(a, b)| a.1 == b.1);
    check(same_lines, "the rays are the same lines as on the Try before it")
}

#[test]
fn the_rays_of_a_try_are_the_same_whether_or_not_a_queen_is_in_the_way() -> Result<(), String> {
    let tab = open()?;
    // Step 3 tries (row 2, column 1) under a queen, and is refused. Step 2 of a later branch tries it free.
    seek(&tab, 3)?;
    let first = rays(&tab)?;
    check(
        first.iter().all(|(class, _)| class == "nq-ray"),
        "neutral on a Try that is about to be refused",
    )?;
    seek(&tab, 2)?;
    seek(&tab, 3)?;
    check_eq(rays(&tab)?, first)
}

#[test]
fn a_solution_is_framed_and_shows_its_vector_and_ordinal() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "kind")?, "Solution".to_owned())?;
    check_eq(field(&tab, "vector")?, "[1, 5, 8, 6, 3, 7, 2, 4]".to_owned())?;
    check_eq(field(&tab, "solutions")?, "1 / 92 (solution 1)".to_owned())?;
    check_eq(queens(&tab)?, 8)?;
    check(
        !attribute(&tab, &format!("{SVG} .nq-frame"), "class")?.contains("nq-off"),
        "the frame is drawn",
    )?;
    check(outline_class(&tab)?.contains("nq-off"), "no outline")?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Remove".to_owned())?;
    check(
        attribute(&tab, &format!("{SVG} .nq-frame"), "class")?.contains("nq-off"),
        "the frame is gone",
    )?;
    check(
        outline_class(&tab)?.contains("nq-removed"),
        "the square the queen left is outlined",
    )?;
    check_eq(queens(&tab)?, 7)
}

#[test]
fn the_last_step_is_exhausted_with_an_empty_board() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(field(&tab, "kind")?, "Exhausted".to_owned())?;
    check_eq(queens(&tab)?, 0)?;
    check_eq(field(&tab, "solutions")?, "92 / 92".to_owned())
}

#[test]
fn going_back_draws_what_going_forward_did() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    let there = (rays(&tab)?, queens(&tab)?, outline_class(&tab)?, cell_label(&tab, 2, 3)?);
    seek(&tab, 4_000)?;
    seek(&tab, 40)?;
    check_eq(
        (rays(&tab)?, queens(&tab)?, outline_class(&tab)?, cell_label(&tab, 2, 3)?),
        there,
    )
}

#[test]
fn the_elements_of_the_board_are_the_same_at_every_step() -> Result<(), String> {
    let tab = open()?;
    let all = format!("{SVG} *");
    let before = count(&tab, &all)?;
    for to in [1, 2, 4, 20_000, 33_589, 0] {
        seek(&tab, to)?;
        check_eq(count(&tab, &all)?, before)?;
    }
    Ok(())
}

// The range.

#[test]
fn the_range_goes_straight_to_a_step() -> Result<(), String> {
    let tab = open()?;
    for to in [1, 20_000, 33_589, 0] {
        seek(&tab, to)?;
        check_eq(
            attribute(&tab, "#n-queens-seek", "aria-valuetext")?.starts_with(&format!("Step {to} of")),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn a_burst_of_moves_is_drawn_once_at_the_latest_place_and_resting_says_it_once() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { window.__draws = 0; const o = new MutationObserver(() => { window.__draws++; }); \
                o.observe(document.getElementById('n-queens-st-position'), { childList: true }); return 'ok'; })()",
    )?;
    // All three moves are in one task, as a fast drag between two frames is, so no frame comes in between.
    text(
        &tab,
        "(() => { const r = document.getElementById('n-queens-seek'); \
         for (const v of ['500', '800', '1200']) { r.value = v; r.dispatchEvent(new Event('input', { bubbles: true })); } \
         return 'ok'; })()",
    )?;
    wait_until(&tab, 5, |tab| Ok(position(tab)? == 1_200))?;
    check_eq(text(&tab, "String(window.__draws)")?, "1".to_owned())?;
    check_eq(live(&tab)?, String::new())?;
    move_range(&tab, 1_200, true)?;
    check(live(&tab)?.starts_with("Step 1200 of"), "it says where it is")
}

// The buttons.

#[test]
fn previous_next_and_restart_move_one_step_and_back_to_the_start() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next")?;
    click(&tab, "n-queens-next")?;
    click(&tab, "n-queens-previous")?;
    check_eq(position(&tab)?, 1)?;
    seek(&tab, 300)?;
    click(&tab, "n-queens-restart")?;
    check_eq((position(&tab)?, queens(&tab)?), (0, 0))
}

#[test]
fn a_button_at_an_end_is_dimmed_keeps_the_focus_and_says_so() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, "#n-queens-previous", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-restart", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-next", "aria-disabled")?, "false".to_owned())?;
    text(&tab, "document.getElementById('n-queens-previous').focus(), 'ok'")?;
    click(&tab, "n-queens-previous")?;
    check_eq(position(&tab)?, 0)?;
    check_eq(focused(&tab)?, "n-queens-previous".to_owned())?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(attribute(&tab, "#n-queens-next", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-play", "aria-disabled")?, "true".to_owned())?;
    click(&tab, "n-queens-next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())
}

#[test]
fn the_solution_buttons_jump_strictly_between_all_92_solutions() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-previous-solution")?;
    check_eq(live(&tab)?, "There is no earlier solution.".to_owned())?;
    for ordinal in 1..=92 {
        click(&tab, "n-queens-next-solution")?;
        check_eq(field(&tab, "solutions")?, format!("{ordinal} / 92 (solution {ordinal})"))?;
    }
    click(&tab, "n-queens-next-solution")?;
    check_eq(live(&tab)?, "There is no later solution.".to_owned())?;
    check_eq(field(&tab, "solutions")?, "92 / 92 (solution 92)".to_owned())?;
    click(&tab, "n-queens-previous-solution")?;
    check_eq(field(&tab, "solutions")?, "91 / 92 (solution 91)".to_owned())
}

// Playing.

#[test]
fn play_runs_one_tick_at_a_time_and_pause_takes_the_tick_back() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "n-queens-play")?;
    check_eq(content(&tab, "#n-queens-play")?, "Pause".to_owned())?;
    check_eq(waiting(&tab)?, 1)?;
    check_eq(advance(&tab, 5)?, 5)?;
    check_eq((position(&tab)?, waiting(&tab)?), (5, 1))?;
    check_eq(live(&tab)?, String::new())?;
    click(&tab, "n-queens-play")?;
    check_eq((content(&tab, "#n-queens-play")?, waiting(&tab)?), ("Play".to_owned(), 0))?;
    check(live(&tab)?.starts_with("Paused at step 5 of"), "pausing says where")?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 5)
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    let tab = open_manual()?;
    for id in [
        "n-queens-next",
        "n-queens-previous",
        "n-queens-restart",
        "n-queens-next-solution",
    ] {
        click(&tab, "n-queens-play")?;
        check_eq(waiting(&tab)?, 1)?;
        click(&tab, id)?;
        check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    }
    click(&tab, "n-queens-play")?;
    move_range(&tab, 700, true)?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 700))?;
    click(&tab, "n-queens-play")?;
    select(&tab, "n-queens-size", "5")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 0))
}

#[test]
fn playing_a_board_of_four_reaches_the_end_and_stops_there() -> Result<(), String> {
    let tab = open_manual()?;
    select(&tab, "n-queens-size", "4")?;
    check_eq(last_step(&tab)?, 139)?;
    click(&tab, "n-queens-play")?;
    check_eq(advance(&tab, 1_000)?, 139)?;
    check_eq((position(&tab)?, field(&tab, "kind")?), (139, "Exhausted".to_owned()))?;
    check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    click(&tab, "n-queens-play")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 139))?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())
}

#[test]
fn only_a_solution_and_the_end_are_announced_while_playing() -> Result<(), String> {
    let tab = open_manual()?;
    select(&tab, "n-queens-size", "4")?;
    let first = {
        click(&tab, "n-queens-next-solution")?;
        let at = position(&tab)?;
        click(&tab, "n-queens-restart")?;
        at
    };
    let announced = live(&tab)?;
    click(&tab, "n-queens-play")?;
    advance(&tab, first - 1)?;
    check_eq(live(&tab)?, announced)?;
    advance(&tab, 1)?;
    check(live(&tab)?.contains("solution 1 of 2"), "the solution is announced")?;
    advance(&tab, 1_000)?;
    check(live(&tab)?.contains("finished"), "the end is announced")
}

// The keyboard and the pointer.

#[test]
fn the_arrow_keys_home_and_end_move_between_squares_and_do_not_move_the_search() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r0-c0').focus(), 'ok'")?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())?;
    check(ring_shown(&tab)?, "the ring is shown while the grid has the focus")?;
    for (key, expected) in [
        ("ArrowRight", "nq-r0-c1"),
        ("ArrowDown", "nq-r1-c1"),
        ("ArrowLeft", "nq-r1-c0"),
        ("ArrowLeft", "nq-r1-c0"),
        ("ArrowUp", "nq-r0-c0"),
        ("ArrowUp", "nq-r0-c0"),
        ("End", "nq-r0-c7"),
        ("Home", "nq-r0-c0"),
    ] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(active_cell(&tab)?, expected.to_owned())?;
        check_eq(focused(&tab)?, expected.to_owned())?;
    }
    tab.press_key("PageDown").map_err(|e| e.to_string())?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())?;
    check_eq(position(&tab)?, 0)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"0\"]"))?, 1)
}

#[test]
fn control_with_home_and_end_go_to_the_corners() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r3-c3').focus(), 'ok'")?;
    {
        let (key, expected) = ("End", "nq-r3-c7");
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(active_cell(&tab)?, expected.to_owned())?;
    }
    text(
        &tab,
        "(() => { const g = document.querySelector('#n-queens-stage-svg [role=gridcell][tabindex=\"0\"]'); \
         g.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', ctrlKey: true, bubbles: true })); return 'ok'; })()",
    )?;
    check_eq(active_cell(&tab)?, "nq-r7-c7".to_owned())?;
    text(
        &tab,
        "(() => { const g = document.querySelector('#n-queens-stage-svg [role=gridcell][tabindex=\"0\"]'); \
         g.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', ctrlKey: true, bubbles: true })); return 'ok'; })()",
    )?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())
}

#[test]
fn a_click_on_a_square_moves_the_focus_and_shows_the_ring_but_not_the_search() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 2)?;
    let at = centre_of(&tab, "nq-r4-c5")?;
    drag(&tab, &[at, at])?;
    check_eq(active_cell(&tab)?, "nq-r4-c5".to_owned())?;
    check_eq(focused(&tab)?, "nq-r4-c5".to_owned())?;
    check(ring_shown(&tab)?, "the ring is shown after a mouse click")?;
    check_eq((position(&tab)?, queens(&tab)?), (2, 1))
}

#[test]
fn the_ring_is_hidden_when_the_focus_leaves_the_grid() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r2-c2').focus(), 'ok'")?;
    check(ring_shown(&tab)?, "shown")?;
    text(&tab, "document.getElementById('n-queens-next').focus(), 'ok'")?;
    check(!ring_shown(&tab)?, "hidden")?;
    text(&tab, "document.getElementById('nq-r2-c2').focus(), 'ok'")?;
    check(ring_shown(&tab)?, "shown again")
}

#[test]
fn the_ring_stays_on_the_active_square_across_steps() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r1-c1').focus(), 'ok'")?;
    seek(&tab, 3)?;
    check_eq(focused(&tab)?, "nq-r1-c1".to_owned())?;
    check(ring_shown(&tab)?, "still shown")
}

// A new size.

#[test]
fn a_new_size_replaces_the_grid_and_clamps_the_active_square() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r7-c7').focus(), 'ok'")?;
    select(&tab, "n-queens-size", "4")?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, 16)?;
    check_eq(count(&tab, &format!("{SVG} [role=row]"))?, 4)?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 139))?;
    check_eq(field(&tab, "depth")?, "0 / 4".to_owned())?;
    check_eq(field(&tab, "solutions")?, "0 / 2".to_owned())?;
    check_eq(active_cell(&tab)?, "nq-r3-c3".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, "139".to_owned())?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "vector")?, "[2, 4, 1, 3]".to_owned())?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "vector")?, "[3, 1, 4, 2]".to_owned())
}

#[test]
fn every_size_of_board_shows_its_own_number_of_solutions() -> Result<(), String> {
    let tab = open()?;
    for (n, solutions) in [(4, 2), (5, 10), (6, 4), (7, 40), (9, 352)] {
        select(&tab, "n-queens-size", &n.to_string())?;
        check_eq(field(&tab, "solutions")?, format!("0 / {solutions}"))?;
        check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, n * n)?;
    }
    Ok(())
}

// A write that fails.

#[test]
fn a_write_that_fails_is_reported_and_the_next_button_draws_the_board_again() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 2)?;
    fail_writes_of(&tab, "d", true)?;
    click(&tab, "n-queens-next")?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the failure is reported")?;
    check(content(&tab, "#n-queens-error")?.contains("may be out of date"), "and says why")?;
    check_eq(position(&tab)?, 2)?;
    click(&tab, "n-queens-next")?;
    check_eq(position(&tab)?, 3)?;
    check(banner_hidden(&tab)?, "a draw that works hides it")?;
    check(
        rays(&tab)?.iter().all(|(class, d)| class == "nq-ray" && !d.is_empty()),
        "the rays are drawn",
    )
}

#[test]
fn a_button_with_nowhere_to_go_still_draws_a_board_that_is_out_of_date() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    fail_writes_of(&tab, "class", true)?;
    click(&tab, "n-queens-previous")?;
    fail_writes_of(&tab, "class", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, last)?;
    click(&tab, "n-queens-next")?;
    check_eq(position(&tab)?, last)?;
    check(banner_hidden(&tab)?, "repaired by a button that had nowhere to go")?;
    check_eq(field(&tab, "kind")?, "Exhausted".to_owned())?;
    check_eq(queens(&tab)?, 0)
}

#[test]
fn a_failure_while_playing_stops_the_playback() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "n-queens-play")?;
    advance(&tab, 2)?;
    fail_writes_of(&tab, "d", true)?;
    advance(&tab, 1)?;
    fail_writes_of(&tab, "d", false)?;
    check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    check_eq(position(&tab)?, 2)?;
    check(!banner_hidden(&tab)?, "reported")
}

// The cost.

#[test]
fn a_step_and_a_jump_on_the_largest_board_are_quick() -> Result<(), String> {
    let tab = open()?;
    select(&tab, "n-queens-size", "9")?;
    check_eq(last_step(&tab)?, 153_502)?;
    let started = Instant::now();
    seek(&tab, 100_000)?;
    for _ in 0..20 {
        click(&tab, "n-queens-next")?;
    }
    let took = started.elapsed();
    eprintln!("n-queens: a jump and 20 steps on a board of 9 took {took:?}");
    check(took < Duration::from_secs(5), "a jump and twenty steps take under five seconds")
}

// The controls in the scene.

#[test]
fn the_selectors_the_buttons_and_the_range_are_all_inside_the_scene() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [data-action]"))?, 6)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject select"))?, 2)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject input[type=\"range\"]"))?, 1)?;
    // None of them is left in the page's own HTML, outside the scene.
    check_eq(
        text(
            &tab,
            "String(Array.from(document.querySelectorAll('#panel-n-queens button, #panel-n-queens select, \
             #panel-n-queens input')).filter(c => !c.closest('#n-queens-stage-svg')).length)",
        )?,
        "0".to_owned(),
    )?;
    for id in ["restart", "previous", "next", "play", "previous-solution", "next-solution"] {
        check_eq(count(&tab, &format!("{SVG} g#n-queens-{id}[role=\"button\"]"))?, 1)?;
    }
    check_eq(attribute(&tab, "#n-queens-size", "aria-label")?, "Board size".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-speed", "aria-label")?, "Speed".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "aria-label")?, "Go to step".to_owned())?;
    check_eq(text(&tab, "document.getElementById('n-queens-size').value")?, "8".to_owned())?;
    check_eq(text(&tab, "document.getElementById('n-queens-speed').value")?, "20".to_owned())
}

#[test]
fn the_scene_is_wide_enough_for_the_controls_and_none_of_them_is_over_the_board() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, SVG, "width")?, "640".to_owned())?;
    check_eq(attribute(&tab, SVG, "height")?, "636".to_owned())?;
    let overlap = text(
        &tab,
        &format!(
            "(() => {{ const scene = document.querySelector('{SVG}').getBoundingClientRect(); \
             const board = document.querySelector('{SVG} .nq-fills').getBoundingClientRect(); \
             const controls = Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')); \
             const bad = controls.filter(c => {{ const r = c.getBoundingClientRect(); \
               return r.left < scene.left || r.right > scene.right || r.bottom > board.top || r.width === 0; }}); \
             return controls.length + ':' + bad.length; }})()"
        ),
    )?;
    check_eq(overlap, "9:0".to_owned())
}

#[test]
fn a_dimmed_button_is_still_in_the_tab_order() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, "#n-queens-previous", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-previous", "tabindex")?, "0".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-next", "tabindex")?, "0".to_owned())
}

#[test]
fn a_button_works_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('n-queens-next').focus(), 'ok'")?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 1)?;
    tab.press_key(" ").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 2)?;
    check_eq(focused(&tab)?, "n-queens-next".to_owned())
}

#[test]
fn the_speed_selector_says_the_speed_in_the_status() -> Result<(), String> {
    let tab = open()?;
    select(&tab, "n-queens-speed", "50")?;
    check_eq(field(&tab, "playback")?, "Paused at 50 events per second".to_owned())?;
    select(&tab, "n-queens-speed", "1")?;
    check_eq(field(&tab, "playback")?, "Paused at 1 events per second".to_owned())
}

#[test]
fn a_new_size_leaves_the_controls_where_they_were() -> Result<(), String> {
    let tab = open()?;
    let before = text(
        &tab,
        &format!(
            "Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')).map(c => {{ const r = c.getBoundingClientRect(); return [r.left, r.top, r.width].join(','); }}).join(';')"
        ),
    )?;
    select(&tab, "n-queens-size", "5")?;
    let after = text(
        &tab,
        &format!(
            "Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')).map(c => {{ const r = c.getBoundingClientRect(); return [r.left, r.top, r.width].join(','); }}).join(';')"
        ),
    )?;
    check_eq(after, before)?;
    check_eq(text(&tab, "document.getElementById('n-queens-size').value")?, "5".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, last_step(&tab)?.to_string())
}

#[test]
fn a_new_board_that_cannot_be_made_leaves_the_old_board_trace_and_range_as_they_were() -> Result<(), String> {
    // The attributes that the controls are written with, which a new board writes: the end of the range, the dimming of
    // a button, and what the range says.
    for name in ["max", "aria-disabled", "aria-valuetext"] {
        let tab = open()?;
        seek(&tab, 100)?;
        fail_writes_of(&tab, name, true)?;
        select(&tab, "n-queens-size", "5")?;
        // While the writes still fail, the page is the old one, whole.
        check_eq(squares(&tab)?, 64).map_err(|e| format!("{name}: the board: {e}"))?;
        check_eq(position(&tab)?, 100).map_err(|e| format!("{name}: the step: {e}"))?;
        check_eq(last_step(&tab)?, 33_589).map_err(|e| format!("{name}: the trace: {e}"))?;
        check_eq(attribute(&tab, "#n-queens-seek", "max")?, "33589".to_owned())
            .map_err(|e| format!("{name}: the range: {e}"))?;
        check_eq(text(&tab, "document.getElementById('n-queens-size').value")?, "8".to_owned())
            .map_err(|e| format!("{name}: the selector: {e}"))?;
        check(!banner_hidden(&tab)?, &format!("{name}: the failure is reported"))?;
        fail_writes_of(&tab, name, false)?;
        // The next action, even one with nowhere to go, draws the old board again, and then everything agrees.
        click(&tab, "n-queens-next")?;
        check_eq(position(&tab)?, 101).map_err(|e| format!("{name}: after: {e}"))?;
        check_eq(attribute(&tab, "#n-queens-seek", "max")?, "33589".to_owned())?;
        check_eq(text(&tab, "document.getElementById('n-queens-seek').value")?, "101".to_owned())?;
        check_eq(field(&tab, "depth")?.ends_with("/ 8"), true)?;
        check_eq(squares(&tab)?, 64)?;
        check(banner_hidden(&tab)?, &format!("{name}: repaired"))?;
        // And the size can be changed once the writes work.
        select(&tab, "n-queens-size", "5")?;
        check_eq((squares(&tab)?, last_step(&tab)?, position(&tab)?), (25, 504, 0))
            .map_err(|e| format!("{name}: then: {e}"))?;
        check_eq(attribute(&tab, "#n-queens-seek", "max")?, "504".to_owned())?;
    }
    Ok(())
}

#[test]
fn a_new_board_that_cannot_be_made_keeps_the_focus_and_the_active_square() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r6-c6').focus(), 'ok'")?;
    fail_writes_of(&tab, "max", true)?;
    select(&tab, "n-queens-size", "4")?;
    fail_writes_of(&tab, "max", false)?;
    check_eq(active_cell(&tab)?, "nq-r6-c6".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"0\"]"))?, 1)?;
    check_eq(squares(&tab)?, 64)
}

#[test]
fn play_does_not_show_as_playing_when_no_timer_can_be_set() -> Result<(), String> {
    let tab = open()?;
    fail_timers(&tab, Some(0))?;
    click(&tab, "n-queens-play")?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check_eq(field(&tab, "playback")?, "Paused at 20 events per second".to_owned())?;
    check(!banner_hidden(&tab)?, "the failure is reported")?;
    check(
        content(&tab, "#n-queens-error")?.contains("Play could not carry on"),
        "and says what failed",
    )?;
    check(live(&tab)?.contains("Play could not carry on"), "and is announced")?;
    // Nothing is going to move the search, however long is waited.
    std::thread::sleep(Duration::from_millis(400));
    check_eq(position(&tab)?, 0)?;
    // The board is right: a button still works, and the failure is not one of drawing the board.
    click(&tab, "n-queens-next")?;
    check_eq(position(&tab)?, 1)?;
    // Once timers work, Play works, and the message goes.
    fail_timers(&tab, None)?;
    click(&tab, "n-queens-play")?;
    check_eq(play_label(&tab)?, "Pause".to_owned())?;
    check(banner_hidden(&tab)?, "the message goes when playback starts")?;
    wait_until(&tab, 10, |tab| Ok(position(tab)? > 3))?;
    click(&tab, "n-queens-play")?;
    check_eq(play_label(&tab)?, "Play".to_owned())
}

#[test]
fn a_tick_that_cannot_be_scheduled_stops_the_playback_and_says_so() -> Result<(), String> {
    let tab = open()?;
    // The first tick can be set, and the one after it cannot.
    fail_timers(&tab, Some(1))?;
    click(&tab, "n-queens-play")?;
    // The first tick comes after 50 milliseconds, which a round trip to the browser can outlast, so only the end is
    // waited for.
    wait_until(&tab, 10, |tab| Ok(!banner_hidden(tab)?))?;
    check_eq(play_label(&tab)?, "Play".to_owned())?;
    check_eq(field(&tab, "playback")?, "Paused at 20 events per second".to_owned())?;
    check(
        content(&tab, "#n-queens-error")?.contains("Play could not carry on"),
        "and says what failed",
    )?;
    // The step that was drawn stays, and nothing runs on after it.
    let stopped = position(&tab)?;
    check_eq(stopped, 1)?;
    std::thread::sleep(Duration::from_millis(400));
    check_eq(position(&tab)?, stopped)?;
    // The board is whole, and carries on from where it was once timers work.
    fail_timers(&tab, None)?;
    click(&tab, "n-queens-play")?;
    wait_until(&tab, 10, |tab| Ok(position(tab)? > stopped + 2))?;
    click(&tab, "n-queens-play")?;
    check(banner_hidden(&tab)?, "the message went when playback started again")
}

#[test]
fn the_manual_clocks_hooks_are_on_the_window_while_the_page_is_and_come_off_when_it_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(hook_types(&tab)?, "function,function".to_owned())?;
    click(&tab, "n-queens-play")?;
    check_eq((waiting(&tab)?, advance(&tab, 2)?), (1, 2))?;
    leave(&tab)?;
    check_eq(hook_types(&tab)?, "undefined,undefined".to_owned())?;
    // Nothing that a person does has any effect once the page is left, and no tick is left waiting to run.
    let at = position(&tab)?;
    click(&tab, "n-queens-next")?;
    click(&tab, "n-queens-play")?;
    check_eq(position(&tab)?, at)
}

#[test]
fn a_hook_that_another_clock_put_in_place_is_left_alone() -> Result<(), String> {
    let tab = open_manual()?;
    text(
        &tab,
        "(() => { window.__nQueensAdvance = function () { return 'other'; }; return 'ok'; })()",
    )?;
    leave(&tab)?;
    // The one that was replaced is not ours to remove. The one that was not, is.
    check_eq(hook_types(&tab)?, "function,undefined".to_owned())?;
    check_eq(text(&tab, "String(window.__nQueensAdvance())")?, "other".to_owned())
}

#[test]
fn a_page_with_the_real_clock_puts_no_hooks_on_the_window() -> Result<(), String> {
    let tab = open()?;
    check_eq(hook_types(&tab)?, "undefined,undefined".to_owned())?;
    click(&tab, "n-queens-play")?;
    leave(&tab)?;
    // The timer that was waiting is cleared, so the search does not move on after the page is left.
    let at = position(&tab)?;
    std::thread::sleep(Duration::from_millis(400));
    check_eq(position(&tab)?, at)
}

// A move of the range that is waiting for its frame.

#[test]
fn a_move_of_the_range_that_is_waiting_for_its_frame_does_not_move_a_board_of_another_size() -> Result<(), String> {
    let tab = open()?;
    // Both happen in one task, as when a person moves the range and the selector between two frames: the range is moved
    // to 20000 on the board of eight, and the size is changed to four before the frame that would draw it.
    text(
        &tab,
        "(() => { const r = document.getElementById('n-queens-seek'); r.value = '20000'; \
         r.dispatchEvent(new Event('input', { bubbles: true })); \
         const s = document.getElementById('n-queens-size'); s.value = '4'; \
         s.dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    // The frame that was waiting comes now.
    std::thread::sleep(Duration::from_millis(300));
    check_eq((squares(&tab)?, last_step(&tab)?), (16, 139))?;
    check_eq(position(&tab)?, 0).map_err(|e| format!("the new board is not at its first step: {e}"))?;
    check_eq(field(&tab, "kind")?, "Initial".to_owned())?;
    check_eq(text(&tab, "document.getElementById('n-queens-seek').value")?, "0".to_owned())?;
    // A move of the range made after the new board is on the page is applied as usual.
    seek(&tab, 100)?;
    check_eq(position(&tab)?, 100)
}

#[test]
fn a_move_of_the_range_that_is_waiting_is_still_applied_when_the_size_does_not_change() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('n-queens-seek'); r.value = '2000'; \
         r.dispatchEvent(new Event('input', { bubbles: true })); return 'ok'; })()",
    )?;
    wait_until(&tab, 5, |tab| Ok(position(tab)? == 2_000))
}
