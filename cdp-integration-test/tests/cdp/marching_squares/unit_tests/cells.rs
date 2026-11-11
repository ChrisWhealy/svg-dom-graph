//! Choosing a cell or a level, editing the threshold, and the fields that can be loaded.

use super::*;

#[test]
fn the_row_and_column_selectors_offer_only_the_cells_of_the_field() -> Result<(), String> {
    let tab = open()?;
    let enabled = |tab: &Tab, id: &str| count(tab, &format!("#{id} option:not([disabled])"));
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (1 + 18, 1 + 24))?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (1 + 6, 1 + 8))?;
    choose(&tab, "ms-field", "lab-tie")?;
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (2, 2))
}

#[test]
fn the_inspected_level_sets_the_numbers_and_the_visible_levels_are_independent_of_it() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    choose(&tab, "ms-inspect", "2")?;
    check(field(&tab, "status")?.contains("high level +0.520"), &field(&tab, "status")?)?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.52".to_owned())?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?, polylines(&tab, 2)?), (5, 3, 4))?;
    choose(&tab, "ms-show", "middle")?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?, polylines(&tab, 2)?), (0, 3, 0))?;
    check(
        field(&tab, "status")?.contains("high level +0.520"),
        "the numbers stay at the inspected level",
    )?;
    check(
        field(&tab, "levels")?.contains("low -0.530: 4 open, 1 closed"),
        "the totals cover all three",
    )
}

// The saddles.

#[test]
fn the_saddle_lab_reads_each_named_cell_with_the_same_solver() -> Result<(), String> {
    let tab = open()?;
    for (key, text_in_status) in [
        ("lab-5-positive", "mask 5; Q positive"),
        ("lab-5-negative", "mask 5; Q negative"),
        ("lab-10-positive", "mask 10; Q positive"),
        ("lab-10-negative", "mask 10; Q negative"),
        ("lab-tie", "mask 5; Q exactly zero"),
    ] {
        choose(&tab, "ms-field", key)?;
        check_eq(step(&tab)?, 1)?;
        choose(&tab, "ms-inspect", "1")?;
        let status = field(&tab, "status")?;
        check(status.contains(text_in_status), &format!("{key}: {status}"))?;
    }
    check(field(&tab, "status")?.contains("1 saddle ties"), &field(&tab, "status")?)?;
    // The tie is two strands, and the picture shows the saddle.
    check_eq(count(&tab, "#ms-inspector line")?, 4)
}

#[test]
fn the_ripple_cells_with_a_decider_are_outlined_once_visited_and_the_decision_is_in_the_status() -> Result<(), String> {
    let tab = open()?;
    let outlines = |tab: &Tab| count(tab, &format!("{STAGE} .ms-scene polyline[stroke=\"#7c3aed\"]"));
    check_eq(outlines(&tab)?, 3)?;
    seek(&tab, 13 * 24 + 22)?;
    check_eq(outlines(&tab)?, 1)?;
    seek(&tab, 13 * 24 + 23)?;
    check_eq(outlines(&tab)?, 2)?;
    inspect_cell(&tab, 13, 22)?;
    choose(&tab, "ms-inspect", "2")?;
    check(
        field(&tab, "status")?
            .starts_with("Cell (13, 22), high level +0.520: mask 5; Q positive; rho 0.416; T–R and B–L"),
        &field(&tab, "status")?,
    )
}

// Thresholds and fields.

#[test]
fn a_threshold_edit_changes_the_topology_and_editing_it_back_restores_the_digest() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let original = (digest(&tab)?, drawn_contours(&tab)?);
    check(
        field(&tab, "status")?.contains("Q negative") && field(&tab, "status")?.contains("T–L and R–B"),
        &field(&tab, "status")?,
    )?;
    choose(&tab, "ms-threshold", "0.102")?;
    check(
        field(&tab, "status")?.contains("Q positive") && field(&tab, "status")?.contains("T–R and B–L"),
        &field(&tab, "status")?,
    )?;
    check(field(&tab, "status")?.contains("middle level +0.102"), &field(&tab, "status")?)?;
    check(digest(&tab)? != original.0, "the topology changed")?;
    check(drawn_contours(&tab)? != original.1, "and so did the picture")?;
    zoom_to(&tab, 400)?;
    pan(&tab, "ArrowRight", 2)?;
    choose(&tab, "ms-threshold", "0.101")?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), original)?;
    check_eq(zoom_percent(&tab)?, 400)
}

#[test]
fn a_threshold_that_is_refused_leaves_everything_and_the_number_goes_back() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let committed = (digest(&tab)?, drawn_contours(&tab)?, step(&tab)?);
    // Not above the level below, not below the level above, not a number, and exactly at a sample.
    for (bad, why) in [
        ("-0.9", "strictly ascending"),
        ("0.6", "strictly ascending"),
        ("", "not finite"),
        ("0.1", "exactly at the middle level"),
    ] {
        choose(&tab, "ms-threshold", bad)?;
        check(!banner_hidden(&tab)?, &format!("{bad:?} is reported"))?;
        check(
            content(&tab, "#ms-error")?.contains(why) || content(&tab, "#ms-error")?.contains("finite"),
            &format!("{bad:?}: {}", content(&tab, "#ms-error")?),
        )?;
        check_eq(value_of(&tab, "ms-threshold")?, "0.101".to_owned()).map_err(|e| format!("{bad:?}: {e}"))?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?, step(&tab)?), committed.clone())?;
    }
    choose(&tab, "ms-threshold", "0.102")?;
    check(banner_hidden(&tab)?, "an edit that works clears the line")
}

#[test]
fn a_new_field_is_shown_at_its_last_step_and_the_view_is_kept_only_between_matching_axes() -> Result<(), String> {
    let tab = open()?;
    zoom_to(&tab, 400)?;
    let zoomed = zoom_percent(&tab)?;
    choose(&tab, "ms-field", "twin")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (CELLS, zoomed))?;
    check(field(&tab, "footer")?.starts_with("Twin hills 25×19"), &field(&tab, "footer")?)?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.24".to_owned())?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (48, 100))?;
    check(
        field(&tab, "footer")?.starts_with("Coarse saddle waves 9×7"),
        &field(&tab, "footer")?,
    )?;
    choose(&tab, "ms-field", "lab-tie")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (1, 100))?;
    check_eq(value_of(&tab, "ms-field")?, "lab-tie".to_owned())
}

#[test]
fn the_coarse_preset_shows_its_saddles_at_one_to_one() -> Result<(), String> {
    let tab = open()?;
    choose(&tab, "ms-field", "coarse")?;
    // 6, 6 and 2 ambiguous cells at the three levels, and a cell that is ambiguous at more than one level is outlined
    // once.
    let outlines = count(&tab, &format!("{STAGE} .ms-scene polyline[stroke=\"#7c3aed\"]"))?;
    check((6..=14).contains(&outlines), &format!("{outlines} outlines"))?;
    let sizes = json(
        &tab,
        r##"(() => JSON.stringify([...document.querySelectorAll('#ms-stage .ms-scene polyline[stroke="#7c3aed"]')].map(p => {
            const r = p.getBoundingClientRect(); return Math.max(r.width, r.height); })))()"##,
    )?;
    let smallest = sizes
        .as_array()
        .and_then(|a| a.iter().filter_map(Value::as_f64).reduce(f64::min))
        .unwrap_or(0.0);
    check(smallest > 60.0, &format!("a cell is {smallest} px across at 100%"))?;
    choose(&tab, "ms-inspect", "0")?;
    inspect_cell(&tab, 1, 1)?;
    check(field(&tab, "status")?.starts_with("Cell (1, 1)"), &field(&tab, "status")?)
}
