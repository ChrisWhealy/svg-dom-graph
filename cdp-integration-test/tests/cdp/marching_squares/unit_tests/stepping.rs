//! Walking through the steps: restarting, the ends, direct seeks, and what a visited cell shows.

use super::*;

// Stepping.

#[test]
fn restart_shows_the_samples_and_no_contour_and_next_visits_one_cell_at_a_time() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    check_eq(step(&tab)?, 0)?;
    for slot in 0..3 {
        check_eq(polylines(&tab, slot)?, 0)?;
    }
    check(
        count(&tab, &format!("{STAGE} .ms-scene circle"))? >= 475,
        "the samples are drawn",
    )?;
    check(field(&tab, "status")?.starts_with("No cell selected."), &field(&tab, "status")?)?;
    check(live(&tab)?.starts_with("No cell selected."), &live(&tab)?)?;
    click_button(&tab, "next")?;
    check_eq(step(&tab)?, 1)?;
    check(field(&tab, "status")?.starts_with("Cell (0, 0)"), &field(&tab, "status")?)?;
    check(field(&tab, "legend")?.contains("partial"), &field(&tab, "legend")?)
}

#[test]
fn the_ends_are_bounded_and_say_so_and_a_dimmed_button_still_takes_a_press() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())?;
    check_eq(attribute(&tab, "[data-action=\"next\"]", "aria-disabled")?, "true".to_owned())?;
    click_button(&tab, "restart")?;
    click_button(&tab, "previous")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    check_eq(
        attribute(&tab, "[data-action=\"restart\"]", "aria-disabled")?,
        "true".to_owned(),
    )
}

#[test]
fn forward_backward_a_direct_seek_and_a_rebuild_draw_the_same_step() -> Result<(), String> {
    let tab = open()?;
    let mut forward = Vec::new();
    click_button(&tab, "restart")?;
    for k in 0..=CELLS {
        if k > 0 {
            click_button(&tab, "next")?;
        }
        if k % 24 == 0 || k == CELLS {
            forward.push((k, digest(&tab)?, drawn_contours(&tab)?));
        }
    }
    for (k, digest_k, contours_k) in forward.iter().rev() {
        seek(&tab, *k)?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (digest_k.clone(), contours_k.clone()))
            .map_err(|e| format!("backward to {k}: {e}"))?;
    }
    for k in [192, 24, 408, 0, 96] {
        seek(&tab, k)?;
        let want = forward.iter().find(|(at, ..)| *at == k).ok_or("a step that was recorded")?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (want.1.clone(), want.2.clone()))
            .map_err(|e| format!("direct to {k}: {e}"))?;
        click_button(&tab, "rebuild")?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (want.1.clone(), want.2.clone()))
            .map_err(|e| format!("rebuilt at {k}: {e}"))?;
    }
    Ok(())
}

// The inspector.

#[test]
fn an_unvisited_cell_reveals_nothing_and_a_visited_one_reads_its_case() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 50)?;
    inspect_cell(&tab, 14, 12)?;
    check(field(&tab, "status")?.contains("Not yet evaluated"), &field(&tab, "status")?)?;
    for hidden in ["mask", "Q ", "rho"] {
        check(
            !field(&tab, "status")?.contains(hidden),
            &format!("{hidden}: {}", field(&tab, "status")?),
        )?;
    }
    check(
        content(&tab, "#ms-inspector")?.contains("Not yet evaluated"),
        "the picture says so too",
    )?;
    seek(&tab, 14 * 24 + 13)?;
    let status = field(&tab, "status")?;
    check(
        status.starts_with("Cell (14, 12), middle level +0.101: mask 10; Q negative; rho 0.425; T–L and R–B"),
        &status,
    )?;
    check(
        content(&tab, "#ms-inspector")?.contains("mask 10"),
        &content(&tab, "#ms-inspector")?,
    )?;
    check_eq(count(&tab, "#ms-inspector line")?, 2)
}
