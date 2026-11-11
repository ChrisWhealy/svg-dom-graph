//! The tests of [`super`], which holds the helpers they use.

use super::*;

#[test]
fn solo_node_can_be_dragged_a_small_distance() -> Result<(), String> {
    let tab = new_tab()?;

    let group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(1)")
        .map_err(|e| format!("could not find solo's <g>: {e}"))?;
    let (before_x, before_y) = group_translate(&group)?;

    let rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(1) rect")
        .map_err(|e| format!("could not find solo's <rect>: {e}"))?;
    let midpoint = rect.get_midpoint().map_err(|e| format!("could not get solo's midpoint: {e}"))?;

    // A real drag: press, several intermediate moves, release — not a single instantaneous jump.
    let (dx, dy) = (40.0, 25.0);
    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + dx / 2.0, midpoint.y + dy / 2.0),
            (midpoint.x + dx, midpoint.y + dy),
        ],
    )?;

    // Give the page a moment to process the dispatched events before reading the result back.
    std::thread::sleep(Duration::from_millis(100));

    let group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(1)")
        .map_err(|e| format!("could not re-find solo's <g> after the drag: {e}"))?;
    let (after_x, after_y) = group_translate(&group)?;

    let expected_x = before_x + dx;
    let expected_y = before_y + dy;
    let close = |got: f64, expected: f64| (got - expected).abs() <= 1.0;

    if !close(after_x, expected_x) || !close(after_y, expected_y) {
        return Err(format!(
            "expected solo's <g> to move to approximately ({expected_x}, {expected_y}), got ({after_x}, {after_y}) \
             — started at ({before_x}, {before_y}), dragged by ({dx}, {dy})"
        ));
    }

    Ok(())
}
