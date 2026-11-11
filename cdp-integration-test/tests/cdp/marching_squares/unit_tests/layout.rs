//! What the page draws when it opens: the controls, their labels and their size, and the plot.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The page.

#[test]
fn the_page_opens_at_the_last_step_of_the_ripple_field_with_three_levels() -> Result<(), String> {
    let tab = open()?;
    check_eq((step(&tab)?, field(&tab, "step")?), (CELLS, format!("Step {CELLS} of {CELLS}")))?;
    check_eq(value_of(&tab, "ms-field")?, "ripple".to_owned())?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.101".to_owned())?;
    check(banner_hidden(&tab)?, "no error")?;
    check(
        field(&tab, "levels")?.contains("low -0.530: 4 open, 1 closed"),
        &field(&tab, "levels")?,
    )?;
    check(
        field(&tab, "levels")?.contains("high +0.520: 4 open, 0 closed"),
        &field(&tab, "levels")?,
    )?;
    check(
        field(&tab, "legend")?.contains("The contour set is complete."),
        &field(&tab, "legend")?,
    )?;
    check_eq(count(&tab, "#ms-staging svg > *")?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene"))?, 1)
}

#[test]
fn every_button_the_range_and_the_selectors_are_in_the_stage_svg_and_outside_the_swapped_part() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, "#panel-marching-squares button")?, 0)?;
    check_eq(count(&tab, "#ms-stage-svg g.ms-controls > [role=button][data-action]")?, 5)?;
    check_eq(count(&tab, "#ms-stage-svg foreignObject select")?, 5)?;
    check_eq(
        count(&tab, "#ms-stage-svg foreignObject input[type=\"number\"]#ms-threshold")?,
        1,
    )?;
    check_eq(count(&tab, "#ms-stage-svg foreignObject input[type=\"range\"]#ms-seek")?, 1)?;
    check_eq(count(&tab, ".ms-scene-host [data-action], .ms-scene-host foreignObject")?, 0)?;
    // The controls are above the plot.
    let above = text(
        &tab,
        "(() => { const b = document.querySelector('[data-action=\"next\"]').getBoundingClientRect(); \
         const t = document.querySelector('#ms-stage .ms-scene').getBoundingClientRect(); return String(b.bottom <= t.top); })()",
    )?;
    check_eq(above, "true".to_owned())?;
    // Every selector has a name that a person can hear.
    check_eq(count(&tab, "#ms-stage-svg foreignObject select:not([aria-label])")?, 0)
}

#[test]
fn the_plot_names_its_levels_by_pattern_and_describes_the_state() -> Result<(), String> {
    let tab = open()?;
    let description = text(
        &tab,
        "document.querySelector('#ms-stage .ms-scene [role=img], #ms-stage .ms-scene [aria-label^=\"Marching\"]').getAttribute('aria-label')",
    )?;
    check(
        description.contains("Ripple plane") && description.contains("low -0.530 solid"),
        &description,
    )?;
    check(
        description.contains("middle +0.101 dashed") && description.contains("high +0.520 dotted"),
        &description,
    )?;
    check(description.contains("432 of 432 cells visited"), &description)?;
    // The legend has three entries, each with its text, and the lines are told apart by dash as well as colour.
    let legend = text(&tab, "document.querySelector('#ms-stage .ms-scene').textContent")?;
    check(
        legend.contains("low, solid") && legend.contains("middle, dashed") && legend.contains("high, dotted"),
        &legend,
    )?;
    let dashes = text(
        &tab,
        "JSON.stringify(['#1d4ed8','#b45309','#047857'].map(c => document.querySelector('#ms-stage .ms-scene polyline[stroke=\"' + c + '\"]').getAttribute('stroke-dasharray')))",
    )?;
    check_eq(dashes, "[null,\"9 5\",\"2 5\"]".to_owned())
}

#[test]
fn the_text_of_every_option_fits_inside_its_selector() -> Result<(), String> {
    let tab = open()?;
    let found = json(
        &tab,
        r#"(() => { const c = document.createElement('canvas').getContext('2d'); const out = [];
            for (const s of document.querySelectorAll('#ms-stage-svg foreignObject select, #ms-stage-svg foreignObject input')) {
                const style = getComputedStyle(s); c.font = style.fontSize + ' ' + style.fontFamily;
                const room = s.getBoundingClientRect().width - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 12;
                const texts = s.tagName === 'SELECT' ? [...s.options].map(o => o.textContent) : [s.value];
                for (const t of texts) out.push([s.id, t, c.measureText(t).width, room]);
            } return JSON.stringify(out); })()"#,
    )?;
    let rows = found.as_array().ok_or("no rows")?;
    check(rows.len() > 40, "every option was measured")?;
    for row in rows {
        let (id, shown, width, room) = (
            &row[0],
            &row[1],
            row[2].as_f64().unwrap_or(f64::MAX),
            row[3].as_f64().unwrap_or(0.0),
        );
        check(
            width <= room,
            &format!("{id}: {shown} is {width:.1} wide and the room is {room:.1}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_name_of_the_range_is_clear_of_the_buttons_above_it_and_of_the_plot_below() -> Result<(), String> {
    let tab = open()?;
    let gaps = json(
        &tab,
        r#"(() => { const label = document.querySelector('#ms-stage-svg .scene-slider-label').getBoundingClientRect();
            const buttons = [...document.querySelectorAll('#ms-stage-svg [data-action]')].map(b => b.getBoundingClientRect().bottom);
            const plot = document.querySelector('#ms-stage .ms-scene').getBoundingClientRect().top;
            const range = document.getElementById('ms-seek').getBoundingClientRect();
            return JSON.stringify([label.top - Math.max(...buttons), plot - range.bottom, label.left]); })()"#,
    )?;
    let (above, below) = (gaps[0].as_f64().unwrap_or(-1.0), gaps[1].as_f64().unwrap_or(-1.0));
    check(above >= 6.0, &format!("the name is {above:.1} px below the buttons"))?;
    check(below >= 2.0, &format!("the range is {below:.1} px above the plot"))
}

#[test]
fn the_controls_stay_one_size_at_every_zoom_and_the_footer_follows_the_zoom() -> Result<(), String> {
    let tab = open()?;
    let size = |tab: &Tab| {
        text(
            tab,
            "(() => { const r = document.querySelector('[data-action=\"next\"] rect').getBoundingClientRect(); \
             const s = document.getElementById('ms-seek').getBoundingClientRect(); \
             return [r.width, r.height, s.width].map(v => v.toFixed(2)).join(' '); })()",
        )
    };
    let before = size(&tab)?;
    for target in [25, 400] {
        zoom_to(&tab, target)?;
        check_eq(size(&tab)?, before.clone()).map_err(|e| format!("at {target}%: {e}"))?;
    }
    wait_until(&tab, 3, |t| Ok(field(t, "zoom")? == "400%"))?;
    zoom_to(&tab, 100)?;
    wait_until(&tab, 3, |t| Ok(field(t, "zoom")? == "100%"))
}
