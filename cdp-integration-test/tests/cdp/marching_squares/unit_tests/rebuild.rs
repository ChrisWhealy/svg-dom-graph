//! What a rebuild of the scene keeps: the view, the step, the cell, and the focus.

use super::*;

// Rebuilding.

#[test]
fn a_scene_rebuilt_twenty_five_times_at_a_zoomed_and_panned_view_is_the_same() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    pan(&tab, "ArrowLeft", 2)?;
    pan(&tab, "ArrowUp", 1)?;
    let view = view_transform(&tab)?;
    let first = (digest(&tab)?, drawn_contours(&tab)?, field(&tab, "status")?);
    text(
        &tab,
        "(() => { window.__nodes = [...document.querySelectorAll('#ms-stage-svg g.ms-controls > *')]; return 'ok'; })()",
    )?;
    let mut ids = std::collections::BTreeSet::new();
    let mut generations = Vec::new();
    let mut attempts = Vec::new();
    for round in 0..25 {
        click_button(&tab, "rebuild")?;
        ids.insert(attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?);
        generations.push(footer_number(&tab, "generation")?);
        attempts.push(footer_number(&tab, "attempt")?);
        check_eq((digest(&tab)?, drawn_contours(&tab)?, field(&tab, "status")?), first.clone())
            .map_err(|e| format!("round {round}: {e}"))?;
        check_eq(view_transform(&tab)?, view.clone()).map_err(|e| format!("round {round}: the view: {e}"))?;
        check_eq(zoom_percent(&tab)?, 400)?;
        check_eq(
            (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
            (1, 0),
        )?;
    }
    check_eq(ids.len(), 25)?;
    check(generations.windows(2).all(|w| w[1] == w[0] + 1), &format!("{generations:?}"))?;
    check(attempts.windows(2).all(|w| w[1] == w[0] + 1), &format!("{attempts:?}"))?;
    check_geometry(&tab, (14, 12), "after 25 rebuilds")?;
    // The controls are the very same elements, and the page's own IDs are unique.
    check_eq(
        text(
            &tab,
            "String([...document.querySelectorAll('#ms-stage-svg g.ms-controls > *')].every((n, i) => n === window.__nodes[i]))",
        )?,
        "true".to_owned(),
    )?;
    let duplicates = text(
        &tab,
        "(() => { const seen = new Set(); let d = 0; for (const e of document.querySelectorAll('[id]')) { if (seen.has(e.id)) d++; seen.add(e.id); } return String(d); })()",
    )?;
    check_eq(duplicates, "0".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene"))?, 1)
}

#[test]
fn a_rebuild_keeps_the_step_the_cell_the_level_mode_and_says_nothing_new() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 300)?;
    inspect_cell(&tab, 14, 12)?;
    choose(&tab, "ms-show", "high")?;
    let said = live(&tab)?;
    for _ in 0..5 {
        click_button(&tab, "rebuild")?;
    }
    check_eq(
        (step(&tab)?, value_of(&tab, "ms-row")?, value_of(&tab, "ms-show")?),
        (300, "14".to_owned(), "high".to_owned()),
    )?;
    check_eq(live(&tab)?, said)?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?), (0, 0))
}

#[test]
fn one_click_on_the_plot_after_many_rebuilds_is_heard_once() -> Result<(), String> {
    let tab = open()?;
    for _ in 0..10 {
        click_button(&tab, "rebuild")?;
    }
    text(
        &tab,
        "(() => { window.__heard = 0; new MutationObserver(m => { window.__heard += m.length; }).observe(document.getElementById('ms-live'), { childList: true, characterData: true, subtree: true }); return 'ok'; })()",
    )?;
    inspect_cell(&tab, 14, 12)?;
    let before = text(&tab, "String(window.__heard)")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    let v = json(
        &tab,
        "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); return JSON.stringify([o.left + o.width / 2 - 3 * o.width, o.top + o.height / 2]); })()",
    )?;
    tab.click_point(headless_chrome::browser::tab::point::Point {
        x: v[0].as_f64().unwrap_or(0.0),
        y: v[1].as_f64().unwrap_or(0.0),
    })
    .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-column")? == "9"))?;
    std::thread::sleep(Duration::from_millis(200));
    let after = text(&tab, "String(window.__heard)")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    check(
        after > before && after - before <= 3,
        &format!("{} mutations for one click", after - before),
    )
}

#[test]
fn the_focus_in_the_scene_survives_a_rebuild_by_its_name() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} .ms-scene [aria-label=\"Zoom in\"]').focus(); return 'ok'; }})()"
        ),
    )?;
    click_button(&tab, "rebuild")?;
    check_eq(
        text(
            &tab,
            "document.activeElement && document.activeElement.getAttribute && String(document.activeElement.getAttribute('aria-label'))",
        )?,
        "Zoom in".to_owned(),
    )?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene [aria-label=\"Zoom in\"]"))?, 1)
}
