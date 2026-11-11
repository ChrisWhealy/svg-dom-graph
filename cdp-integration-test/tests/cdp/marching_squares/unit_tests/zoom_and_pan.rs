//! The geometry that is drawn, and what zoom, pan and a click on the plot do to it.

use super::*;

// The geometry.

#[test]
fn the_drawn_contours_are_right_in_the_grids_own_coordinates_and_the_open_ends_survive_clipping() -> Result<(), String>
{
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    check_geometry(&tab, (14, 12), "at 1x")
}

#[test]
fn the_geometry_is_unchanged_by_zoom_and_pan_and_the_model_digest_does_not_move() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    for target in [25, 400, 100] {
        zoom_to(&tab, target)?;
        check_eq(zoom_percent(&tab)?, target)?;
        check_geometry(&tab, (14, 12), &format!("at {target}%"))?;
        check(
            screen_deviation(&tab)? <= SCREEN_TOLERANCE,
            &format!("at {target}%: {}", screen_deviation(&tab)?),
        )?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), before.clone()).map_err(|e| format!("at {target}%: {e}"))?;
    }
    zoom_to(&tab, 400)?;
    let at_four = view_transform(&tab)?;
    pan(&tab, "ArrowRight", 3)?;
    pan(&tab, "ArrowDown", 2)?;
    check(view_transform(&tab)? != at_four, "the view moved")?;
    check_geometry(&tab, (14, 12), "after a pan at 400%")?;
    check(screen_deviation(&tab)? <= SCREEN_TOLERANCE, "after a pan")?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), before)
}

#[test]
fn a_wheel_zoom_about_an_off_centre_pointer_and_a_pointer_pan_survive_a_rebuild() -> Result<(), String> {
    let tab = open()?;
    // A cell near the top left, so that the pointer is well away from the centre of the scene.
    inspect_cell(&tab, 4, 1)?;
    let toolbar = toolbar_place(&tab)?;
    let start = outline_corner(&tab)?;
    // Two turns of the real wheel with Ctrl held, over the corner of the cell. The point under the pointer must not
    // move.
    for round in 1..=2 {
        let before = zoom_percent(&tab)?;
        ctrl_wheel(&tab, start, -100.0)?;
        wait_until(&tab, 3, |t| Ok(zoom_percent(t)? > before))?;
        let now = outline_corner(&tab)?;
        check(
            near(now, start, 0.75),
            &format!("turn {round}: the point under the pointer moved from {start:?} to {now:?}"),
        )?;
    }
    check(zoom_percent(&tab)? > 100, "zoomed in")?;
    check_eq(
        toolbar_place(&tab)?.iter().map(|v| v.round()).collect::<Vec<_>>(),
        toolbar.iter().map(|v| v.round()).collect::<Vec<_>>(),
    )?;
    // A drag on the plot with the real mouse pans the view by the distance of the drag.
    let before = (outline_corner(&tab)?, view_transform(&tab)?);
    let from = (before.0.0 + 60.0, before.0.1 + 60.0);
    drag(
        &tab,
        &[
            from,
            (from.0 + 20.0, from.1 + 10.0),
            (from.0 + 60.0, from.1 + 30.0),
            (from.0 + 90.0, from.1 + 45.0),
        ],
    )?;
    let panned = outline_corner(&tab)?;
    check(view_transform(&tab)? != before.1, "the drag moved the view")?;
    check(
        near(panned, (before.0.0 + 90.0, before.0.1 + 45.0), 2.0),
        &format!("moved from {:?} to {panned:?}", before.0),
    )?;
    check_eq(
        toolbar_place(&tab)?.iter().map(|v| v.round()).collect::<Vec<_>>(),
        toolbar.iter().map(|v| v.round()).collect::<Vec<_>>(),
    )?;
    // The contours are the same in the grid's coordinates, and in the page.
    let (percent, view) = (zoom_percent(&tab)?, view_transform(&tab)?);
    let drawn = (digest(&tab)?, drawn_contours(&tab)?);
    check_geometry(&tab, (4, 1), "after a wheel zoom and a pan")?;
    check(screen_deviation(&tab)? <= SCREEN_TOLERANCE, "on the screen")?;
    // Rebuilding keeps all of it.
    click_button(&tab, "rebuild")?;
    check_eq((zoom_percent(&tab)?, view_transform(&tab)?), (percent, view))?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), drawn)?;
    check(
        near(outline_corner(&tab)?, panned, 0.75),
        &format!("the cell moved to {:?}", outline_corner(&tab)?),
    )?;
    check_eq(
        toolbar_place(&tab)?.iter().map(|v| v.round()).collect::<Vec<_>>(),
        toolbar.iter().map(|v| v.round()).collect::<Vec<_>>(),
    )?;
    check_geometry(&tab, (4, 1), "after the rebuild")?;
    check(screen_deviation(&tab)? <= SCREEN_TOLERANCE, "on the screen after the rebuild")
}

#[test]
fn a_drag_on_the_plot_pans_and_does_not_choose_a_cell_and_a_click_still_does() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 9, 12)?;
    zoom_to(&tab, 400)?;
    let before = (outline_corner(&tab)?, view_transform(&tab)?);
    let centre = json(
        &tab,
        "(() => { const r = document.getElementById('ms-stage-svg').getBoundingClientRect(); return JSON.stringify([r.left + 380, r.top + 178 + 260]); })()",
    )?;
    let from = (centre[0].as_f64().unwrap_or(0.0), centre[1].as_f64().unwrap_or(0.0));
    drag(&tab, &[from, (from.0 + 25.0, from.1 + 25.0), (from.0 + 70.0, from.1 - 30.0)])?;
    check(view_transform(&tab)? != before.1, "the drag panned")?;
    check(
        near(outline_corner(&tab)?, (before.0.0 + 70.0, before.0.1 - 30.0), 2.0),
        &format!("{:?}", outline_corner(&tab)?),
    )?;
    // A drag is not a click, so the cell that was chosen is still the one.
    check_eq(
        (value_of(&tab, "ms-row")?, value_of(&tab, "ms-column")?),
        ("9".to_owned(), "12".to_owned()),
    )?;
    // A press that does not move is a click, and chooses the cell that it is on.
    let spot = outline_corner(&tab)?;
    tab.click_point(headless_chrome::browser::tab::point::Point {
        x: spot.0 + 20.0,
        y: spot.1 + 20.0,
    })
    .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-column")? == "12"))?;
    check_eq(view_transform(&tab)? != before.1, true)?;
    // A drag that begins on the toolbar does not pan.
    let view = view_transform(&tab)?;
    let bar = json(
        &tab,
        "(() => { const r = document.querySelector('#ms-stage .ms-scene [aria-label=\"Scene controls\"]').getBoundingClientRect(); return JSON.stringify([r.left + r.width / 2, r.top + 4]); })()",
    )?;
    let from = (bar[0].as_f64().unwrap_or(0.0), bar[1].as_f64().unwrap_or(0.0));
    drag(&tab, &[from, (from.0 - 30.0, from.1 - 5.0), (from.0 - 90.0, from.1 - 10.0)])?;
    check_eq(view_transform(&tab)?, view)
}

#[test]
fn a_loop_has_a_dot_at_its_seam_as_wide_as_the_line() -> Result<(), String> {
    let tab = open()?;
    choose(&tab, "ms-field", "twin")?;
    inspect_cell(&tab, 9, 12)?;
    let dots = text(
        &tab,
        r#"(() => { const scene = document.querySelector('#ms-stage .ms-scene');
            const loops = [...scene.querySelectorAll('polyline')].filter(p => ['#1d4ed8','#b45309','#047857'].includes(p.getAttribute('stroke')))
                .filter(p => { const v = p.getAttribute('points').trim().split(/\s+/); return v.length > 2 && v[0] === v[v.length - 1]; });
            return JSON.stringify(loops.map(p => { const first = p.getAttribute('points').trim().split(/\s+/)[0].split(',').map(Number);
                const d = [...scene.querySelectorAll('circle')].find(c => c.getAttribute('fill') === p.getAttribute('stroke') &&
                    Math.abs(+c.getAttribute('cx') - first[0]) < 1e-6 && Math.abs(+c.getAttribute('cy') - first[1]) < 1e-6);
                return d ? +d.getAttribute('r') : null; })); })()"#,
    )?;
    let radii: Vec<Option<f64>> = serde_json::from_str(&dots).map_err(|e| e.to_string())?;
    check(!radii.is_empty(), "the twin hills have loops")?;
    check(radii.iter().all(|r| *r == Some(1.25)), &format!("{radii:?}"))
}

#[test]
fn a_partial_frame_has_no_loop_closed_early_and_no_join_across_unvisited_cells() -> Result<(), String> {
    let tab = open()?;
    // The low level has one closed loop. Before the last of its cells has been visited, it is not closed.
    let mut first_closed = None;
    for k in (0..=CELLS).step_by(8) {
        seek(&tab, k)?;
        let closed = text(
            &tab,
            r##"String([...document.querySelectorAll('#ms-stage .ms-scene polyline[stroke="#1d4ed8"]')].filter(p => {
                const v = p.getAttribute('points').trim().split(/\s+/); return v.length > 2 && v[0] === v[v.length - 1]; }).length)"##,
        )?;
        if closed != "0" && first_closed.is_none() {
            first_closed = Some(k);
        }
        if first_closed.is_some() {
            check_eq(closed.as_str(), "1")?;
        }
    }
    let first = first_closed.ok_or("the loop closes")?;
    check(first > 8 && first < CELLS, &format!("closed at {first}"))?;
    // At the end of the field the result is whole, and the status says so.
    check(field(&tab, "status")?.contains("Complete."), &field(&tab, "status")?)
}

#[test]
fn clicking_the_plot_chooses_the_cell_through_the_zoom() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    // Where the outline of the cell is on the screen, now and at 400%.
    let centre_of_outline = |tab: &Tab| -> Result<(f64, f64), String> {
        let v = json(
            tab,
            "(() => { const r = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); \
             return JSON.stringify([r.left + r.width / 2, r.top + r.height / 2]); })()",
        )?;
        Ok((v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0)))
    };
    // Choose another cell by a click at the centre of the cell (4, 1), whose screen place is the outline's place moved
    // by the cell size, found from the sample markers.
    let (x, y) = centre_of_outline(&tab)?;
    let step_px = json(
        &tab,
        "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); \
         return JSON.stringify([o.width, o.height]); })()",
    )?;
    let (cw, ch) = (step_px[0].as_f64().unwrap_or(1.0), step_px[1].as_f64().unwrap_or(1.0));
    let (tx, ty) = (x - 11.0 * cw, y - 10.0 * ch);
    tab.click_point(headless_chrome::browser::tab::point::Point { x: tx, y: ty })
        .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-row")? == "4"))?;
    check_eq(
        (value_of(&tab, "ms-row")?, value_of(&tab, "ms-column")?),
        ("4".to_owned(), "1".to_owned()),
    )?;
    check(field(&tab, "status")?.starts_with("Cell (4, 1)"), &field(&tab, "status")?)?;
    // At 400% only the middle of the plot is on the screen, so use a cell there, and click its neighbour on the left.
    zoom_to(&tab, 400)?;
    inspect_cell(&tab, 9, 12)?;
    let (x, y) = centre_of_outline(&tab)?;
    let (cw, _) = {
        let v = json(
            &tab,
            "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); return JSON.stringify([o.width, o.height]); })()",
        )?;
        (v[0].as_f64().unwrap_or(1.0), v[1].as_f64().unwrap_or(1.0))
    };
    check(cw > 80.0, &format!("a cell is {cw} px wide at 400%"))?;
    tab.click_point(headless_chrome::browser::tab::point::Point { x: x - cw, y })
        .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-column")? == "11"))?;
    check_eq(
        (value_of(&tab, "ms-row")?, value_of(&tab, "ms-column")?),
        ("9".to_owned(), "11".to_owned()),
    )?;
    Ok(())
}
