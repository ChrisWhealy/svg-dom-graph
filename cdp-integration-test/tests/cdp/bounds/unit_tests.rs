//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_node_dragged_past_the_view_box_clamps_and_stays_real_clickable() -> Result<(), String> {
    let tab = new_tab()?;

    let rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(7) rect")
        .map_err(|e| format!("could not find bounded's <rect>: {e}"))?;
    let start = rect
        .get_midpoint()
        .map_err(|e| format!("could not get bounded's midpoint: {e}"))?;

    // A real drag, well past the view box's own right edge (500) and above its own top edge (0). `bounded` starts at
    // (400, 20), size (80, 40). So a (300, -60) move overshoots the right edge by a wide margin and the top edge
    // comfortably. Kept well inside `launch_browser`'s own generously sized window. An even larger delta risks landing
    // at some unrelated point past the window's actual edge instead of the view box's.
    drag(
        &tab,
        &[
            (start.x, start.y),
            (start.x + 150.0, start.y - 30.0),
            (start.x + 300.0, start.y - 60.0),
        ],
    )?;
    std::thread::sleep(Duration::from_millis(100));

    // Clamped to the view box's own top-right corner: x = 500 - 80 = 420, y = 0.
    let (clamped_x, clamped_y) = rect_origin(&tab)?;
    let close = |got: f64, expected: f64| (got - expected).abs() <= 1.0;
    if !close(clamped_x, 420.0) || !close(clamped_y, 0.0) {
        return Err(format!(
            "expected bounded's <g> to clamp to approximately (420, 0), got ({clamped_x}, {clamped_y})"
        ));
    }

    // The crux of this test: re-find the element and ask for its own midpoint again, fresh, from wherever it is now. If
    // the first drag had left it outside the view box's own clipped rendering area, a real browser could never resolve
    // this to a paintable point at all. This call is standing in for a user's own mouse cursor finding something to
    // click.
    let clamped_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(7) rect")
        .map_err(|e| format!("could not re-find bounded's <rect> after the first drag: {e}"))?;
    let clamped_midpoint = clamped_rect
        .get_midpoint()
        .map_err(|e| format!("could not get bounded's midpoint after it was clamped to the edge: {e}"))?;

    // Drags it back toward the centre of the view box, landing clear of every other fixture node (see this module's own
    // doc comment for why that matters here). It is comfortably clear of both edges too. So this second drag is not
    // itself reclamped, and its own success is unambiguous.
    let (dx, dy) = (-250.0, 150.0);
    drag(
        &tab,
        &[
            (clamped_midpoint.x, clamped_midpoint.y),
            (clamped_midpoint.x + dx / 2.0, clamped_midpoint.y + dy / 2.0),
            (clamped_midpoint.x + dx, clamped_midpoint.y + dy),
        ],
    )?;
    std::thread::sleep(Duration::from_millis(100));

    let (final_x, final_y) = rect_origin(&tab)?;
    let expected_x = clamped_x + dx;
    let expected_y = clamped_y + dy;
    if !close(final_x, expected_x) || !close(final_y, expected_y) {
        return Err(format!(
            "expected bounded's second drag to land at approximately ({expected_x}, {expected_y}), got \
             ({final_x}, {final_y}) — the node clamped to the edge by the first drag did not respond to a real, \
             separately hit-tested second drag"
        ));
    }

    Ok(())
}
