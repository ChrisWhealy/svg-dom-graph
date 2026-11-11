//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_two_connectors_render_the_expected_sharp_and_rounded_elbow_routes() -> Result<(), String> {
    let tab = new_tab()?;

    let sharp = connector_d(&tab, 1)?;
    let expected_sharp = "M 100 40 L 200 40 L 200 170 L 300 170";
    if sharp != expected_sharp {
        return Err(format!(
            "expected the sharp connector's d to be {expected_sharp:?}, got {sharp:?}"
        ));
    }

    let rounded = connector_d(&tab, 2)?;
    let expected_rounded = "M 100 40 L 192 40 A 8 8 0 0 1 200 48 L 200 162 A 8 8 0 0 0 208 170 L 300 170";
    if rounded != expected_rounded {
        return Err(format!(
            "expected the rounded connector's d to be {expected_rounded:?}, got {rounded:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector's rendered `<path>` must never fill. An elbow route bends back on itself, and the SVG default fill is
/// black. An unset `fill` would paint a solid black wedge across every corner.
#[test]
fn a_connector_path_does_not_fill_and_carries_the_arrow_marker() -> Result<(), String> {
    let tab = new_tab()?;

    let selector = "#diagram > g.svg-dom-graph-content > path:nth-of-type(1)";
    let path = tab
        .find_element(selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?;

    let fill = path
        .get_attribute_value("fill")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no fill attribute"))?;
    if fill != "none" {
        return Err(format!("expected {selector}'s fill to be \"none\", got {fill:?}"));
    }

    let marker_end = path
        .get_attribute_value("marker-end")
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("{selector} has no marker-end attribute"))?;
    if !marker_end.starts_with("url(#") {
        return Err(format!(
            "expected {selector}'s marker-end to reference a marker, got {marker_end:?}"
        ));
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging `solo` reroutes both connectors' `solo`-end. The rounded connector stays rounded afterwards — proving
/// `corner_radius` survives a redraw, not just the initial draw.
#[test]
fn dragging_solo_reroutes_both_connectors_and_keeps_their_own_corner_radius() -> Result<(), String> {
    let tab = new_tab()?;

    let solo_rect = tab
        .find_element("#diagram > g.svg-dom-graph-content > g:nth-of-type(1) rect")
        .map_err(|e| format!("could not find solo's <rect>: {e}"))?;
    let midpoint = solo_rect
        .get_midpoint()
        .map_err(|e| format!("could not get solo's midpoint: {e}"))?;

    let (dx, dy) = (40.0, 25.0);
    drag(
        &tab,
        &[
            (midpoint.x, midpoint.y),
            (midpoint.x + dx / 2.0, midpoint.y + dy / 2.0),
            (midpoint.x + dx, midpoint.y + dy),
        ],
    )?;

    std::thread::sleep(Duration::from_millis(100));

    let sharp = connector_d(&tab, 1)?;
    let expected_sharp = "M 140 65 L 220 65 L 220 170 L 300 170";
    if sharp != expected_sharp {
        return Err(format!(
            "expected the sharp connector's d after the drag to be {expected_sharp:?}, got {sharp:?}"
        ));
    }

    let rounded = connector_d(&tab, 2)?;
    let expected_rounded = "M 140 65 L 212 65 A 8 8 0 0 1 220 73 L 220 162 A 8 8 0 0 0 228 170 L 300 170";
    if rounded != expected_rounded {
        return Err(format!(
            "expected the rounded connector's d after the drag to be {expected_rounded:?}, got {rounded:?}"
        ));
    }

    Ok(())
}
