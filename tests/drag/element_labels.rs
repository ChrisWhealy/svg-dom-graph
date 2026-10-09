//! `DataNodeContent::with_labels`/`with_labelling_style`: a label beside each row of a grid, outside the cells and in
//! the grid box's own left padding. It is off by default, follows `LabellingStyle`, is the same size as the data text, is
//! hidden from assistive technology, and is named in the first cell's own `aria-label` instead.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

/// Adds `content`, to a fresh scene `id`, and returns its node size and the texts drawn.
fn draw(id: &str, content: DataNodeContent) -> Result<(Size, Vec<web_sys::Element>), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    let size = scene.node_rect(node).map_err(|e| e.to_string())?.size;
    let group = nth_group(id, 0)?;
    let all = group.query_selector_all("text").map_err(|e| format!("{e:?}"))?;
    let texts = (0..all.length())
        .filter_map(|i| all.get(i))
        .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
        .collect();
    Ok((size, texts))
}

use wasm_bindgen::JsCast;

fn eight() -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8((1..=8).collect()), DataFormat::Decimal).with_layout(GridLayout::Columns(1))
}

fn visible(texts: &[web_sys::Element]) -> Vec<String> {
    texts
        .iter()
        .filter(|t| t.get_attribute("aria-hidden").is_some())
        .map(|t| t.text_content().unwrap_or_default())
        .collect()
}

#[wasm_bindgen_test]
fn labels_are_off_by_default() -> Result<(), String> {
    let (_, texts) = draw("labels-off", eight())?;
    check(
        texts.len() == 8 && visible(&texts).is_empty(),
        "an unlabelled grid draws only its 8 value texts",
    )
}

#[wasm_bindgen_test]
fn numeric_labels_read_zero_to_n_minus_one() -> Result<(), String> {
    let (_, texts) = draw("labels-numeric", eight().with_labels())?;
    check(
        visible(&texts) == ["0", "1", "2", "3", "4", "5", "6", "7"],
        &format!("expected 0..7, got {:?}", visible(&texts)),
    )
}

#[wasm_bindgen_test]
fn alphabetic_labels_read_a_to_h() -> Result<(), String> {
    let content = eight().with_labelling_style(LabellingStyle::Alphabetic);
    let (_, texts) = draw("labels-alpha", content)?;
    check(
        visible(&texts) == ["a", "b", "c", "d", "e", "f", "g", "h"],
        &format!("expected a..h, got {:?}", visible(&texts)),
    )
}

#[wasm_bindgen_test]
fn alphabetic_labels_grow_past_z() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8((0..28).collect()), DataFormat::Decimal)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Alphabetic);
    let (_, texts) = draw("labels-alpha-long", content)?;
    let labels = visible(&texts);
    check(
        labels[25] == "z" && labels[26] == "aa" && labels[27] == "ab",
        &format!("got {labels:?}"),
    )
}

#[wasm_bindgen_test]
fn labels_cost_the_node_at_most_a_little_width_and_no_height() -> Result<(), String> {
    let (plain, _) = draw("labels-width-off", eight())?;
    let (labelled, _) = draw("labels-width-on", eight().with_labels())?;
    check(labelled.width >= plain.width, "a label should not narrow the node")?;
    check(
        labelled.width - plain.width < 10.0,
        &format!(
            "a one-character label should fit nearly in the padding: {} to {}",
            plain.width, labelled.width
        ),
    )?;
    check(
        (labelled.height - plain.height).abs() < 0.01,
        "labels should not change the height",
    )
}

#[wasm_bindgen_test]
fn a_label_is_the_same_size_as_the_data_text_and_left_of_its_cell() -> Result<(), String> {
    let (_, texts) = draw("labels-geometry", eight().with_labelling_style(LabellingStyle::Alphabetic))?;
    let label = texts
        .iter()
        .find(|t| t.get_attribute("aria-hidden").is_some())
        .ok_or("no label drawn")?;
    let value = texts
        .iter()
        .find(|t| t.get_attribute("aria-hidden").is_none())
        .ok_or("no value drawn")?;
    check(
        label.get_attribute("font-size") == value.get_attribute("font-size"),
        "the label should use the data text's own font size",
    )?;
    let x = |e: &web_sys::Element| e.get_attribute("x").and_then(|v| v.parse::<f64>().ok()).unwrap_or(f64::NAN);
    check(x(label) < x(value), "the label should sit left of the value")
}

#[wasm_bindgen_test]
fn a_wider_grid_labels_each_row_by_its_first_element() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8((1..=8).collect()), DataFormat::Decimal)
        .with_layout(GridLayout::Columns(4))
        .with_labels();
    let (_, texts) = draw("labels-rows", content)?;
    check(
        visible(&texts) == ["0", "4"],
        &format!("expected one label per row, got {:?}", visible(&texts)),
    )
}

#[wasm_bindgen_test]
fn a_first_cell_names_its_element_in_its_accessible_name() -> Result<(), String> {
    let content = eight().with_labelling_style(LabellingStyle::Alphabetic);
    let (_, texts) = draw("labels-aria", content)?;
    let named: Vec<String> = texts.iter().filter_map(|t| t.get_attribute("aria-label")).collect();
    check(
        named.first().map(String::as_str) == Some("element a, row 0, column 0: 1"),
        &format!("got {:?}", named.first()),
    )
}

#[wasm_bindgen_test]
fn a_single_value_is_never_labelled() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal).with_labels();
    let (_, texts) = draw("labels-single", content)?;
    check(texts.len() == 1, "a single value draws only its own text")
}
