//! Rejection and acceptance of bad input: empty content, a zero grid layout and a blank node name are refused before
//! anything is drawn, and a padded name is kept verbatim.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_data_node`/`add_data_node_with` rejects an empty `DataNodeContent` before drawing anything or touching
/// the graph's model — mirrors `add_node_with`'s own `EdgeAnchors(0)` rejection test.
#[wasm_bindgen_test]
fn add_data_node_rejects_empty_content_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-empty", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let empty = DataNodeContent::new(NodeValues::U8(vec![]), DataFormat::Decimal);
    let result = scene.add_data_node(Point::new(10.0, 10.0), empty);
    check(
        matches!(result, Err(Error::EmptyNodeContent)),
        &format!("expected Err(Error::EmptyNodeContent), got {result:?}"),
    )?;
    check(
        nth_group("data-node-empty", 0).is_err(),
        "a rejected add_data_node call left a <g> rendered in the scene",
    )?;

    let valid = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    scene.add_data_node(Point::new(10.0, 10.0), valid).map_err(|e| e.to_string())?;
    nth_group("data-node-empty", 0)?;
    check(
        nth_group("data-node-empty", 1).is_err(),
        "expected exactly one <g> after the rejected call and one valid add_data_node call",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_data_node`/`add_data_node_with` rejects a `GridLayout` wrapping `0`, before drawing anything or touching
/// the graph's model. This mirrors `add_data_node_rejects_empty_content_before_touching_the_scene` above.
#[wasm_bindgen_test]
fn add_data_node_rejects_a_zero_grid_layout_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-bad-layout", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    for layout in [GridLayout::Columns(0), GridLayout::Rows(0), GridLayout::MaxColumns(0)] {
        let content = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal).with_layout(layout);
        let result = scene.add_data_node(Point::new(10.0, 10.0), content);
        check(
            matches!(result, Err(Error::InvalidGridLayout(_))),
            &format!("expected Err(Error::InvalidGridLayout(_)) for {layout:?}, got {result:?}"),
        )?;
    }
    check(
        nth_group("data-node-bad-layout", 0).is_err(),
        "a rejected add_data_node call left a <g> rendered in the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_named_data_node` rejects an empty `name`, and one holding only whitespace, before drawing anything or touching
/// the graph — an accessible name of `": u8 = 12"` names nothing.
#[wasm_bindgen_test]
fn add_named_data_node_rejects_an_empty_or_whitespace_only_name_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-named-blank", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    for blank in ["", "   ", "\t\n"] {
        let content = DataNodeContent::new(NodeValues::U8(vec![12]), DataFormat::Decimal);
        let result = scene.add_named_data_node(Point::new(10.0, 10.0), blank, content);
        check(
            matches!(result, Err(Error::EmptyNodeName)),
            &format!("name {blank:?}: expected Err(Error::EmptyNodeName), got {result:?}"),
        )?;
    }
    check(
        nth_group("data-node-named-blank", 0).is_err(),
        "a rejected add_named_data_node call left a <g> rendered in the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `name` that is not blank, only padded with surrounding whitespace, is accepted — and used exactly as given, not
/// silently trimmed.
#[wasm_bindgen_test]
fn add_named_data_node_accepts_a_name_padded_with_whitespace_verbatim() -> Result<(), String> {
    let svg = make_svg("data-node-named-padded", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let content = DataNodeContent::new(NodeValues::U8(vec![12]), DataFormat::Decimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), " B ", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-padded", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some(" B : u8 = 12"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )
}

/// `add_named_data_node` validates `content` exactly as `add_data_node` does — the shared implementation behind both —
/// before drawing anything or touching the graph.
#[wasm_bindgen_test]
fn add_named_data_node_rejects_empty_content_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-named-empty", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let empty = DataNodeContent::new(NodeValues::U8(vec![]), DataFormat::Decimal);
    let result = scene.add_named_data_node(Point::new(10.0, 10.0), "A", empty);
    check(
        matches!(result, Err(Error::EmptyNodeContent)),
        &format!("expected Err(Error::EmptyNodeContent), got {result:?}"),
    )?;
    check(
        nth_group("data-node-named-empty", 0).is_err(),
        "a rejected add_named_data_node call left a <g> rendered in the scene",
    )
}
