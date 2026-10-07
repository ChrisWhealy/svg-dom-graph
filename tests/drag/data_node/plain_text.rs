//! `DataFormat::PlainText`: the whole string in one box, rejection of non-printable or non-`u8` content, and grid
//! settings being ignored.

use super::support::{rect_children, text_children};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `DataFormat::PlainText` draws the whole string in one text box: one `<rect>`, one `<text>`, no grid. Spaces stay
/// spaces.
#[wasm_bindgen_test]
fn plain_text_draws_one_box_holding_the_whole_string() -> Result<(), String> {
    let svg = make_svg("data-node-plain-text", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(b"Hello, big world".to_vec()), DataFormat::PlainText);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-plain-text", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 1, &format!("expected 1 text, found {}", texts.len()))?;
    check(
        texts[0].text_content().as_deref() == Some("Hello, big world"),
        "the text holds the whole string",
    )?;
    let rects = rect_children(&group)?;
    check(rects.len() == 1, &format!("expected 1 rect, found {}", rects.len()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Non-printable bytes and non-`u8` values are rejected, and a rejected call draws nothing.
#[wasm_bindgen_test]
fn plain_text_rejects_non_printable_or_non_u8_content() -> Result<(), String> {
    let svg = make_svg("data-node-plain-bad", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    for values in [NodeValues::U8(vec![b'a', 0x07]), NodeValues::U16(vec![0x4142])] {
        let content = DataNodeContent::new(values, DataFormat::PlainText);
        check(
            matches!(
                scene.add_data_node(Point::new(10.0, 10.0), content),
                Err(Error::InvalidPlainText)
            ),
            "expected InvalidPlainText",
        )?;
    }
    check(nth_group("data-node-plain-bad", 0).is_err(), "nothing was drawn")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `PlainText` has no grid, so grid settings are ignored and not validated. `Columns(0)` would be rejected for any
/// other format.
#[wasm_bindgen_test]
fn plain_text_ignores_an_invalid_grid_layout() -> Result<(), String> {
    let svg = make_svg("data-node-plain-layout", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(b"abc".to_vec()), DataFormat::PlainText)
        .with_layout(GridLayout::Columns(0))
        .with_column_groups(2);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    check(rect_children(&nth_group("data-node-plain-layout", 0)?)?.len() == 1, "one box")
}
