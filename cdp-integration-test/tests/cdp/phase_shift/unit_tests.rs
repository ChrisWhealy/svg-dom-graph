//! The tests of [`super`], which holds the helpers they use.

use super::*;

#[test]
fn the_two_scenes_have_the_sizes_that_were_asked_for_and_nothing_else_is_there() -> Result<(), String> {
    let tab = open()?;
    for (svg, size) in [(CIRCLE, "400x400"), (SINE, "430x400")] {
        let got = text(
            &tab,
            &format!(
                "(() => {{ const s = document.querySelector('{svg}'); \
                 return s.getAttribute('width') + 'x' + s.getAttribute('height'); }})()"
            ),
        )?;
        check_eq(got, size.to_owned())?;
        check_eq(count(&tab, &format!("{svg} g[role=\"img\"]"))?, 1)?;
    }
    check_eq(count(&tab, "#panel-phase-shift svg")?, 2)?;
    check_eq(count(&tab, "#panel-phase-shift [data-action]")?, 0)?;
    Ok(())
}

#[test]
fn the_point_starts_at_45_degrees_and_stays_there() -> Result<(), String> {
    let tab = open()?;
    check_eq(angle_text(&tab)?, "\u{3c6}=45\u{b0}".to_owned())?;
    check_eq(handle_attribute(&tab, "aria-valuenow")?, "45".to_owned())?;
    check(
        sine_labels(&tab)?.contains("sin(x + 45\u{b0})"),
        "the second wave is named with 45",
    )?;
    std::thread::sleep(Duration::from_millis(600));
    check_eq(angle_text(&tab)?, "\u{3c6}=45\u{b0}".to_owned())
}

#[test]
fn the_sine_plot_has_a_blue_wave_and_a_wave_of_another_colour() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SINE} polyline{FIRST_WAVE}"))?, 1)?;
    check_eq(count(&tab, &format!("{SINE} polyline{SECOND_WAVE}"))?, 1)?;
    let labels = text(
        &tab,
        &format!("[...document.querySelectorAll('{SINE} text')].map(t => t.textContent).join('|')"),
    )?;
    check(
        labels.contains("sin(x)") && labels.contains("sin(x + 45\u{b0})"),
        &format!("the waves are named: {labels:?}"),
    )
}

#[test]
fn dragging_the_point_moves_the_second_wave_and_never_the_first() -> Result<(), String> {
    let tab = open()?;
    let (first, second) = (wave(&tab, FIRST_WAVE)?, wave(&tab, SECOND_WAVE)?);
    drag_to(&tab, 120)?;
    check_eq(shown(&tab)?, 120.0)?;
    check_eq(wave(&tab, FIRST_WAVE)?, first)?;
    check(wave(&tab, SECOND_WAVE)? != second, "the second wave has moved")?;
    check(
        sine_labels(&tab)?.contains("sin(x + 120\u{b0})"),
        "the second wave is named with its new shift",
    )
}

#[test]
fn the_point_can_be_dragged_backwards_as_well_as_forwards() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 200)?;
    check_eq(shown(&tab)?, 200.0)?;
    drag_to(&tab, 20)?;
    check_eq(shown(&tab)?, 20.0)
}

#[test]
fn a_shift_of_nothing_puts_the_second_wave_on_the_first_and_marks_no_distance() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 90)?;
    check_eq(count(&tab, &format!("{SINE} [stroke-dasharray=\"1 5\"]"))?, 1)?;
    focus_handle(&tab)?;
    press(&tab, "Home")?;
    check_eq(shown(&tab)?, 0.0)?;
    check_eq(count(&tab, &format!("{SINE} [stroke-dasharray=\"1 5\"]"))?, 0)
}

#[test]
fn the_keys_move_the_point_and_the_wave_follows() -> Result<(), String> {
    let tab = open()?;
    focus_handle(&tab)?;
    let before = wave(&tab, SECOND_WAVE)?;
    press(&tab, "ArrowRight")?;
    check_eq(shown(&tab)?, 46.0)?;
    check(wave(&tab, SECOND_WAVE)? != before, "the second wave has moved")?;
    press(&tab, "PageUp")?;
    check_eq(shown(&tab)?, 61.0)?;
    press(&tab, "End")?;
    check_eq(shown(&tab)?, 359.0)?;
    press(&tab, "ArrowRight")?;
    check_eq(shown(&tab)?, 0.0)?;
    check_eq(handle_attribute(&tab, "aria-valuenow")?, "0".to_owned())
}

#[test]
fn the_circle_has_an_arc_and_the_angle_beside_it() -> Result<(), String> {
    let tab = open()?;
    check_eq(angle_text(&tab)?, "\u{3c6}=45\u{b0}".to_owned())?;
    check_eq(arc_points(&tab)?, 46)
}

#[test]
fn the_arc_and_its_text_follow_the_point_the_long_way_round_beyond_180_degrees() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 100)?;
    check_eq(angle_text(&tab)?, "\u{3c6}=100\u{b0}".to_owned())?;
    check_eq(arc_points(&tab)?, 101)?;
    drag_to(&tab, 270)?;
    check_eq(angle_text(&tab)?, "\u{3c6}=270\u{b0}".to_owned())?;
    check_eq(arc_points(&tab)?, 271)?;
    drag_to(&tab, 20)?;
    check_eq(angle_text(&tab)?, "\u{3c6}=20\u{b0}".to_owned())?;
    check_eq(arc_points(&tab)?, 21)
}

#[test]
fn the_grid_of_scenes_needs_no_scroll_bar_in_a_window_that_is_wide_enough() -> Result<(), String> {
    let tab = open()?;
    let sizes = text(
        &tab,
        "(() => { const g = document.querySelector('.phase-shift-grid'); \
         return [g.scrollWidth, g.clientWidth, g.scrollHeight, g.clientHeight].join(','); })()",
    )?;
    let n: Vec<i64> = sizes
        .split(',')
        .map(|n| n.parse().map_err(|e| format!("{e}")))
        .collect::<Result<_, _>>()?;
    check_eq(n[0], n[1])?;
    check_eq(n[2], n[3])
}
