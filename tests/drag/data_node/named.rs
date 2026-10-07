//! Named data nodes (`Scene::add_named_data_node`): the further labelled outer box, its accessible name, and connectors
//! anchoring to the outer box.

use super::support::{rect_children, text_children, title_of};
use crate::common::{attr_f64, check, check_close, group_translate, make_svg, nth_group, the_connector};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_named_data_node` wraps a single-value node's own box in a further outer box of its own, labelled with
/// the given name. It is the same "outer box labelled with a name, inset value box beneath it" shape an operator node
/// already draws for its own result. The inner value box stays inset from every outer edge, exactly as an operator's
/// own value cell does.
#[wasm_bindgen_test]
fn a_named_single_value_data_node_wraps_it_in_a_further_labelled_outer_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "A", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-single", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    check(
        rects.len() == 2,
        &format!("expected 1 outer named box + 1 inner value box, found {}", rects.len()),
    )?;
    check(
        texts.len() == 2,
        &format!("expected 1 name label + 1 value text, found {}", texts.len()),
    )?;
    check(
        texts[0].text_content().as_deref() == Some("A"),
        &format!(
            "expected the first text to be the name label \"A\", got {:?}",
            texts[0].text_content()
        ),
    )?;
    check(
        texts[1].text_content().as_deref() == Some("AB"),
        &format!(
            "expected the second text to be the value \"AB\", got {:?}",
            texts[1].text_content()
        ),
    )?;

    let outer = &rects[0];
    let inner = &rects[1];
    let outer_width = attr_f64(outer, "width")?;
    let outer_height = attr_f64(outer, "height")?;
    let inner_x = attr_f64(inner, "x")?;
    let inner_y = attr_f64(inner, "y")?;
    let inner_width = attr_f64(inner, "width")?;
    let inner_height = attr_f64(inner, "height")?;

    check(
        inner_x > 0.0,
        &format!("expected the inner value box's left edge inset from the outer box's own, got x={inner_x}"),
    )?;
    check(
        inner_x + inner_width < outer_width,
        &format!(
            "expected the inner value box's right edge inset from the outer box's own, got {} against an outer \
             width of {outer_width}",
            inner_x + inner_width
        ),
    )?;
    check(
        inner_y > 0.0,
        &format!("expected the inner value box's top edge below the outer box's own label row, got y={inner_y}"),
    )?;
    check(
        inner_y + inner_height < outer_height,
        &format!(
            "expected the inner value box's bottom edge inset from the outer box's own, got {} against an outer \
             height of {outer_height}",
            inner_y + inner_height
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A named single-value node's own `aria-label` reads `"{name}: {type} = {value}"`. That is the node's own name, its
/// type, and its real formatted value, all as text. A plain (unnamed) node's own label gives only the type. Its
/// `<title>` — the browser's own mouse-hover tooltip — carries that exact same text.
#[wasm_bindgen_test]
fn a_named_single_value_data_nodes_own_aria_label_includes_the_name_and_the_real_value() -> Result<(), String> {
    let svg = make_svg("data-node-named-aria-label", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xABCD_EF01_2345_6789]), DataFormat::Hexadecimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "B", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-aria-label", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("B: u64 = AB CD EF 01 23 45 67 89"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group)?.as_deref() == Some("B: u64 = AB CD EF 01 23 45 67 89"),
        &format!("unexpected <title>: {:?}", title_of(&group)?),
    )
}

/// Same shape again, but for a multi-value grid. The outer named box wraps the grid's own existing box rather than
/// replacing it. That box is itself unchanged: still its own light-blue background plus one coloured `<rect>` per
/// value.
#[wasm_bindgen_test]
fn a_named_multi_value_data_node_wraps_the_grid_in_a_further_labelled_outer_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-multi", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "values", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-multi", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    // 1 outer named box + 1 inner grid box + 4 per-value cells.
    check(
        rects.len() == 6,
        &format!("expected 6 rects (outer named box + grid box + 4 cells), found {}", rects.len()),
    )?;
    // 1 name label + 4 cell values.
    check(
        texts.len() == 5,
        &format!("expected 5 texts (name label + 4 cell values), found {}", texts.len()),
    )?;
    check(
        texts[0].text_content().as_deref() == Some("values"),
        &format!(
            "expected the first text to be the name label \"values\", got {:?}",
            texts[0].text_content()
        ),
    )
}

/// An incoming connector anchors to a named data node's own *outer* named box, never to the inner value box `name`
/// wraps. An operator node's own result cell already gets the same guarantee, for the same reason. A connector landing
/// on the inner box would look like it terminates at the value, not at the named quantity as a whole.
#[wasm_bindgen_test]
fn a_named_data_nodes_own_connector_anchors_to_the_outer_named_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-anchor", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "src")
        .map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    let named = scene
        .add_named_data_node(Point::new(200.0, 100.0), "A", content)
        .map_err(|e| e.to_string())?;
    scene.add_edge(plain, named).map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-anchor", 1)?;
    let (nx, ny) = group_translate(&group)?;
    let outer = &rect_children(&group)?[0];
    let outer_width = attr_f64(outer, "width")?;
    let outer_height = attr_f64(outer, "height")?;

    let d = crate::common::path_d(&the_connector("data-node-named-anchor")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;

    // The anchor must land somewhere on the outer box's own perimeter — on one of its four edges — never strictly
    // inside it, where the inner value box sits.
    let on_perimeter = check_close(end_x, nx).is_ok()
        || check_close(end_x, nx + outer_width).is_ok()
        || check_close(end_y, ny).is_ok()
        || check_close(end_y, ny + outer_height).is_ok();
    check(
        on_perimeter,
        &format!(
            "expected the connector to land on the outer named box's own perimeter, got ({end_x}, {end_y}) \
             against an outer box at ({nx}, {ny}) sized {outer_width}x{outer_height}"
        ),
    )
}
