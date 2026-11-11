//! The tests of [`super`], which holds the helpers they use.

use super::*;

#[test]
fn dropping_mover_onto_blocker_lands_at_the_expected_clear_position() -> Result<(), String> {
    let tab = new_tab()?;

    let mover_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(3) rect")
        .map_err(|e| format!("could not find mover's <rect>: {e}"))?;
    let mover_midpoint = mover_rect
        .get_midpoint()
        .map_err(|e| format!("could not get mover's midpoint: {e}"))?;

    let blocker_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(2) rect")
        .map_err(|e| format!("could not find blocker's <rect>: {e}"))?;
    let blocker_midpoint = blocker_rect
        .get_midpoint()
        .map_err(|e| format!("could not get blocker's midpoint: {e}"))?;

    // Drags mover's centre directly onto blocker's centre — a real, multi-step drag, not a single instantaneous jump,
    // so it exercises the same pointermove sequence a real user's mouse would produce.
    let steps = 4;
    let mut waypoints = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        waypoints.push((
            mover_midpoint.x + (blocker_midpoint.x - mover_midpoint.x) * t,
            mover_midpoint.y + (blocker_midpoint.y - mover_midpoint.y) * t,
        ));
    }
    drag(&tab, &waypoints)?;

    // Give the page a moment to process the dispatched events, including the corrective move_node call resolve_overlap
    // triggers on pointerup, before reading the result back.
    std::thread::sleep(Duration::from_millis(100));

    let mover_group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(3)")
        .map_err(|e| format!("could not re-find mover's <g> after the drag: {e}"))?;
    let (after_x, after_y) = group_translate(&mover_group)?;

    let (expected_x, expected_y) = (214.0, 150.0);
    let close = |got: f64, expected: f64| (got - expected).abs() <= 1.0;

    if !close(after_x, expected_x) || !close(after_y, expected_y) {
        return Err(format!(
            "expected mover's <rect> to land at approximately ({expected_x}, {expected_y}), got ({after_x}, {after_y})"
        ));
    }

    let blocker_group = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(2)")
        .map_err(|e| format!("could not re-find blocker's <g>: {e}"))?;
    let (blocker_x, blocker_y) = group_translate(&blocker_group)?;
    // mover: (80, 40) at (after_x, after_y); blocker: (80, 40) at (blocker_x, blocker_y).
    let overlaps = after_x < blocker_x + 80.0
        && after_x + 80.0 > blocker_x
        && after_y < blocker_y + 40.0
        && after_y + 40.0 > blocker_y;
    if overlaps {
        return Err(format!(
            "mover ({after_x}, {after_y}) still overlaps blocker ({blocker_x}, {blocker_y}) after the drop"
        ));
    }

    Ok(())
}
