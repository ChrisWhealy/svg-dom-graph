//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both Argand diagrams are plot nodes: an image with a name, and the second has one dot for each root of unity.
#[test]
fn both_diagrams_are_plot_nodes_with_a_name_and_the_marks_of_what_they_show() -> Result<(), String> {
    let tab = open()?;
    check(name(&tab)?.starts_with("Argand diagram of 3 + 4i"), &name(&tab)?)?;
    let roots = text(
        &tab,
        &format!("document.querySelector('{ROOTS} g[role=\"img\"]').getAttribute('aria-label')"),
    )?;
    check(roots.contains("8 roots of unity"), &roots)?;
    // A dot for each root, and the unit circle, in the roots diagram.
    check(
        number(
            &tab,
            &format!("document.querySelectorAll('{ROOTS} g[role=\"img\"] circle').length"),
        )? == 8.0,
        "eight dots",
    )?;
    check(
        number(
            &tab,
            &format!("document.querySelectorAll('{DIAGRAM} g[role=\"img\"] circle').length"),
        )? == 1.0,
        "one dot",
    )?;
    // The real and the imaginary axes carry an arrowhead each, and the imaginary ticks are written with an i.
    check(
        number(
            &tab,
            &format!("document.querySelectorAll('{DIAGRAM} g[role=\"img\"] > path').length"),
        )? == 2.0,
        "two arrowheads",
    )?;
    let ticks = text(
        &tab,
        &format!("[...document.querySelectorAll('{DIAGRAM} g[role=\"img\"] text')].map(t => t.textContent).join(' ')"),
    )?;
    check(ticks.contains("5i") && ticks.contains("Re") && ticks.contains("Im"), &ticks)
}

/// A real click on the diagram puts the number there, rounded to a tenth, and moves both sliders and the diagram with
/// it.
#[test]
fn a_real_click_on_the_diagram_puts_the_number_where_it_was_clicked() -> Result<(), String> {
    let tab = open()?;
    click(&tab, page_point(&tab, 2.0, -3.0)?)?;
    wait_for("the sliders to move", || {
        Ok(number(&tab, "document.querySelector('.argand-slider-real').value")? == 2.0)
    })?;
    check(
        number(&tab, "document.querySelector('.argand-slider-imaginary').value")? == -3.0,
        "the imaginary slider",
    )?;
    wait_for("the diagram to be named for the new number", || {
        Ok(name(&tab)?.starts_with("Argand diagram of 2 - 3i"))
    })?;
    check(
        number(
            &tab,
            &format!("document.querySelectorAll('{DIAGRAM} g[role=\"img\"] circle').length"),
        )? == 1.0,
        "still one dot: the drawing was replaced, not added to",
    )?;
    // Where it was not exactly on a tenth, the nearest tenth is taken.
    click(&tab, page_point(&tab, -1.52, 0.98)?)?;
    wait_for("the second click", || {
        Ok(number(&tab, "document.querySelector('.argand-slider-real').value")? == -1.5)
    })?;
    check(
        number(&tab, "document.querySelector('.argand-slider-imaginary').value")? == 1.0,
        "rounded to a tenth",
    )
}

/// A click in the margin of the node, outside the plotted area, changes nothing.
#[test]
fn a_click_outside_the_plotted_area_changes_nothing() -> Result<(), String> {
    let tab = open()?;
    let (x, y) = page_point(&tab, -5.5, 5.5)?;
    click(&tab, (x - 6.0, y - 6.0))?;
    std::thread::sleep(Duration::from_millis(200));
    check(
        number(&tab, "document.querySelector('.argand-slider-real').value")? == 3.0,
        "the real slider is where it was",
    )?;
    check(name(&tab)?.starts_with("Argand diagram of 3 + 4i"), "so is the diagram")
}
