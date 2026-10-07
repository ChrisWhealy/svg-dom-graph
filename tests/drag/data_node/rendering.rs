//! Rendering: a single value fills the whole box with its type colour, several values each get their own coloured inner
//! cell, and every cell shares one width however the values are formatted.

use super::support::{rect_children, text_children};
use crate::common::{attr_f64, check, check_close, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single value gets no inner cell box, and the outer box itself is filled with the value's own type colour. Its text
/// reads exactly the byte-group hex text the feature request's own example describes: no `0x` prefix, uppercase,
/// single-space byte separation.
#[wasm_bindgen_test]
fn add_data_node_with_a_single_value_colours_the_whole_box_and_has_no_inner_cell() -> Result<(), String> {
    let svg = make_svg("data-node-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-single", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 1, &format!("expected 1 text, found {}", texts.len()))?;
    check(
        texts[0].text_content().as_deref() == Some("F0 E1 D2 C3 B4 A5 96 87"),
        &format!("unexpected text: {:?}", texts[0].text_content()),
    )?;

    let rects = rect_children(&group)?;
    check(
        rects.len() == 1,
        &format!(
            "a single-value data node should have no inner cell box, found {} rects",
            rects.len()
        ),
    )?;
    check(
        rects[0].get_attribute("fill").as_deref() == Some("#f5dce4"),
        &format!(
            "expected the u64 type colour on the outer box, got {:?}",
            rects[0].get_attribute("fill")
        ),
    )?;
    check(attr_f64(&rects[0], "width")? > 0.0, "auto-computed rect width was not positive")?;
    check(
        attr_f64(&rects[0], "height")? > 0.0,
        "auto-computed rect height was not positive",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two or more values each get their own type-coloured inner cell box, inside the node's own unchanged, light blue
/// outer box. This is the second example from the feature request: two values stack into two rows of one column. It is
/// now rendered as two distinct coloured cells rather than two plain text lines.
#[wasm_bindgen_test]
fn add_data_node_with_two_values_gives_each_its_own_coloured_inner_cell() -> Result<(), String> {
    let svg = make_svg("data-node-two", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAA, 0xBB]), DataFormat::Hexadecimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-two", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 2, &format!("expected 2 texts, found {}", texts.len()))?;
    check(texts[0].text_content().as_deref() == Some("AA"), "unexpected text 0")?;
    check(texts[1].text_content().as_deref() == Some("BB"), "unexpected text 1")?;

    let rects = rect_children(&group)?;
    check(
        rects.len() == 3,
        &format!("expected 1 outer + 2 inner cell rects, found {}", rects.len()),
    )?;
    check(
        rects[0].get_attribute("fill").as_deref() == Some("#eef4ff"),
        &format!(
            "expected the unchanged light-blue outer box, got {:?}",
            rects[0].get_attribute("fill")
        ),
    )?;
    for (i, cell_rect) in rects[1..].iter().enumerate() {
        check(
            cell_rect.get_attribute("fill").as_deref() == Some("#fdebd3"),
            &format!(
                "expected the u8 type colour on inner cell {i}, got {:?}",
                cell_rect.get_attribute("fill")
            ),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `u64` under [`DataFormat::Binary`] is the extreme cell-aspect-ratio case: 64 digits, nybble-grouped and
/// byte-separated, render far wider than the cell is tall. `GridLayout`/`Automatic`'s own doc comment names this exact
/// case as the reason `GridLayout::MaxColumns` exists.
#[wasm_bindgen_test]
fn add_data_node_with_a_u64_binary_value_renders_an_extremely_wide_cell() -> Result<(), String> {
    let svg = make_svg("data-node-u64-binary", Size::new(1200.0, 260.0), Size::new(1200.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x0102030405060708]), DataFormat::Binary);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-u64-binary", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 1, &format!("expected 1 text, found {}", texts.len()))?;
    check(
        texts[0].text_content().as_deref()
            == Some("0000 0001 0000 0010 0000 0011 0000 0100 0000 0101 0000 0110 0000 0111 0000 1000"),
        &format!("unexpected text: {:?}", texts[0].text_content()),
    )?;

    let rects = rect_children(&group)?;
    let width = attr_f64(&rects[0], "width")?;
    let height = attr_f64(&rects[0], "height")?;
    check(
        width > height * 5.0,
        &format!("expected an extremely wide cell (width={width}, height={height})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `draw_content_box` measures only the widest cell's own rendered text, by character count, to size every cell alike —
/// see that function's own doc comment. Every `Hexadecimal`/`Binary` value of one integer type already renders the same
/// character count regardless of magnitude. So `Decimal` values of genuinely different digit counts are the one
/// scenario that actually exercises "pick the *right* cell to measure". They are not merely "measuring is skipped for
/// the rest."
#[wasm_bindgen_test]
fn add_data_node_with_decimal_values_of_different_digit_counts_shares_one_uniform_cell_width() -> Result<(), String> {
    let svg = make_svg("data-node-decimal-widths", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U32(vec![1, 4_294_967_295]), DataFormat::Decimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-decimal-widths", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 2, &format!("expected 2 texts, found {}", texts.len()))?;
    check(
        texts[0].text_content().as_deref() == Some("1"),
        &format!("unexpected first value text: {:?}", texts[0].text_content()),
    )?;
    check(
        texts[1].text_content().as_deref() == Some("4294967295"),
        &format!("unexpected second value text: {:?}", texts[1].text_content()),
    )?;

    // Both cells share one uniform width — the one-digit value's own cell is not narrower than the ten-digit value's.
    let rects = rect_children(&group)?;
    let narrow_width = attr_f64(&rects[1], "width")?;
    let wide_width = attr_f64(&rects[2], "width")?;
    check_close(narrow_width, wide_width)?;

    // That shared width is wide enough for the ten-digit value, not just the one-digit value — proving the longer
    // string, not the shorter one, was the one actually measured.
    let height = attr_f64(&rects[1], "height")?;
    check(
        wide_width > height * 2.0,
        &format!("expected a cell wide enough for a 10-digit value (width={wide_width}, height={height})"),
    )
}
