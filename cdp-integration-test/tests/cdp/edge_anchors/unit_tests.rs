//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn hubs_two_connectors_land_on_distinct_fixing_points() -> Result<(), String> {
    let tab = new_tab()?;

    let to_branch_a = connector_d(&tab, 3)?;
    let expected_to_branch_a = "M 230 260 L 230 300 L 100 300 L 100 340";
    if to_branch_a != expected_to_branch_a {
        return Err(format!(
            "expected the hub-to-branch_a connector's d to be {expected_to_branch_a:?}, got {to_branch_a:?}"
        ));
    }

    let to_branch_b = connector_d(&tab, 4)?;
    let expected_to_branch_b = "M 270 260 L 270 300 L 400 300 L 400 340";
    if to_branch_b != expected_to_branch_b {
        return Err(format!(
            "expected the hub-to-branch_b connector's d to be {expected_to_branch_b:?}, got {to_branch_b:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging `branch_a` far enough moves its `hub`-side anchor onto a different one of `hub`'s three fixing points. That
/// proves the snapped candidate is recomputed live on every redraw, not fixed once at `add_edge` time.
#[test]
fn dragging_branch_a_re_snaps_its_connector_onto_a_different_fixing_point() -> Result<(), String> {
    let tab = new_tab()?;

    let branch_a_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(5) rect")
        .map_err(|e| format!("could not find branch_a's <rect>: {e}"))?;
    let midpoint = branch_a_rect
        .get_midpoint()
        .map_err(|e| format!("could not get branch_a's midpoint: {e}"))?;

    let dx = 100.0;
    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + dx / 2.0, midpoint.y),
            (midpoint.x + dx, midpoint.y),
        ],
    )?;

    std::thread::sleep(Duration::from_millis(100));

    let to_branch_a = connector_d(&tab, 3)?;
    let expected_to_branch_a = "M 250 260 L 250 300 L 200 300 L 200 340";
    if to_branch_a != expected_to_branch_a {
        return Err(format!(
            "expected the hub-to-branch_a connector's d after the drag to be {expected_to_branch_a:?}, got {to_branch_a:?}"
        ));
    }

    let to_branch_b = connector_d(&tab, 4)?;
    let expected_to_branch_b = "M 270 260 L 270 300 L 400 300 L 400 340";
    if to_branch_b != expected_to_branch_b {
        return Err(format!(
            "expected the untouched hub-to-branch_b connector's d to still be {expected_to_branch_b:?}, got {to_branch_b:?}"
        ));
    }

    Ok(())
}
