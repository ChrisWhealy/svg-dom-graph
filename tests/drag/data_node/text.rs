//! `NodeValues::Text` and `DataFormat::Text`: each string drawn as it is and never as markup, cells wide enough for the
//! widest string whatever font a character comes from, the cell-width hint, replacements, and what is refused.

use super::support::{check_fits, rect_children, text_children, texts_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, UnaryOperator},
};
use wasm_bindgen_test::wasm_bindgen_test;

const INFINITY: &str = "\u{221E}";
const DASH: &str = "\u{2014}";

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

fn text(values: &[&str]) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Text(strings(values)), DataFormat::Text).with_layout(GridLayout::Columns(1))
}

fn scene_with(id: &str, content: DataNodeContent) -> Result<(Scene, NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 500.0), Size::new(900.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

/// The width of each cell of the first node in `id`, a single value's one box or each inner cell of a grid.
fn cell_widths(id: &str) -> Result<Vec<f64>, String> {
    let group = nth_group(id, 0)?;
    let rects = rect_children(&group)?;
    Ok(rects[1..].iter().map(|r| r.get_bounding_client_rect().width()).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every string is drawn as given, Unicode and empty ones included, in one cell each, and fits its cell.
#[wasm_bindgen_test]
fn every_string_is_drawn_as_it_is_and_fits_its_cell() -> Result<(), String> {
    let values = ["0", INFINITY, DASH, "A", "", "\u{4E16}\u{754C}", "1250"];
    let _scene = scene_with("text-cells", text(&values))?;
    check(texts_of("text-cells")? == values, &format!("{:?}", texts_of("text-cells")?))?;
    check_fits("text-cells")?;
    let widths = cell_widths("text-cells")?;
    check(
        widths.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01),
        &format!("one width for every cell: {widths:?}"),
    )
}

/// A string that looks like markup is text. Nothing in it becomes an element.
#[wasm_bindgen_test]
fn a_string_that_looks_like_markup_is_drawn_literally() -> Result<(), String> {
    let _scene = scene_with("text-markup", text(&["<b>bold</b>", "<script>1</script>", "&amp;"]))?;
    let group = nth_group("text-markup", 0)?;
    check(
        texts_of("text-markup")? == ["<b>bold</b>", "<script>1</script>", "&amp;"],
        "the text is literal",
    )?;
    check(
        group.query_selector("b, script").map_err(|e| format!("{e:?}"))?.is_none(),
        "no element was made from the text",
    )
}

/// Spaces inside a string are kept, not collapsed to one.
#[wasm_bindgen_test]
fn spaces_inside_a_string_are_kept() -> Result<(), String> {
    let _scene = scene_with("text-spaces", text(&["a   b", " lead"]))?;
    check(
        texts_of("text-spaces")? == ["a   b", " lead"],
        &format!("{:?}", texts_of("text-spaces")?),
    )?;
    let group = nth_group("text-spaces", 0)?;
    for cell in text_children(&group)? {
        check(
            cell.get_attribute("style")
                .is_some_and(|style| style.contains("white-space: pre")),
            "a text cell keeps its spaces",
        )?;
    }
    check_fits("text-spaces")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The hint widens the cells to its own measured width, and is not drawn or counted or spoken.
#[wasm_bindgen_test]
fn a_width_hint_widens_the_cells_without_adding_a_value() -> Result<(), String> {
    let _plain = scene_with("text-nohint", text(&["A", "B"]))?;
    let _hinted = scene_with("text-hint", text(&["A", "B"]).with_cell_width_hint("9999999999"))?;
    let (plain, hinted) = (cell_widths("text-nohint")?, cell_widths("text-hint")?);
    check(
        hinted[0] > plain[0] + 20.0,
        &format!("hinted {hinted:?} against plain {plain:?}"),
    )?;
    check(texts_of("text-hint")? == ["A", "B"], "the hint is not drawn")?;
    let group = nth_group("text-hint", 0)?;
    check(
        !group.get_attribute("aria-label").unwrap_or_default().contains("9999"),
        "the hint is not part of the accessible name",
    )?;
    check_fits("text-hint")
}

/// A string wider than the hint still gets the room it needs.
#[wasm_bindgen_test]
fn a_string_wider_than_the_hint_still_fits() -> Result<(), String> {
    let _scene = scene_with("text-wide", text(&["A", "a much longer string"]).with_cell_width_hint("99"))?;
    check_fits("text-wide")
}

/// With a hint as wide as every later value, replacing the values changes no cell and no box.
#[wasm_bindgen_test]
fn with_a_wide_enough_hint_a_replacement_moves_nothing() -> Result<(), String> {
    let (scene, node) = scene_with("text-replace", text(&["0", INFINITY, DASH]).with_cell_width_hint("1250"))?;
    let before = scene.node_rect(node).map_err(|e| e.to_string())?;
    let widths = cell_widths("text-replace")?;
    scene
        .set_data_values(node, NodeValues::Text(strings(&["0", "1250", "A"])))
        .map_err(|e| e.to_string())?;
    check(
        texts_of("text-replace")? == ["0", "1250", "A"],
        &format!("{:?}", texts_of("text-replace")?),
    )?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == before,
        "the node's box did not change",
    )?;
    check(cell_widths("text-replace")? == widths, "no cell changed width")?;
    check_fits("text-replace")
}

/// An empty string is read out as "empty", and a replacement keeps the name up to date.
#[wasm_bindgen_test]
fn an_empty_cell_is_named_empty_and_a_replacement_updates_the_name() -> Result<(), String> {
    let (scene, node) = scene_with("text-empty", text(&["", "B"]))?;
    let names = |id: &str| -> Result<Vec<Option<String>>, String> {
        Ok(text_children(&nth_group(id, 0)?)?
            .iter()
            .map(|t| t.get_attribute("aria-label"))
            .collect())
    };
    check(
        names("text-empty")? == [Some("row 0, column 0: empty".to_owned()), Some("row 1, column 0: B".to_owned())],
        &format!("{:?}", names("text-empty")?),
    )?;
    scene
        .set_data_values(node, NodeValues::Text(strings(&["A", ""])))
        .map_err(|e| e.to_string())?;
    check(
        names("text-empty")? == [Some("row 0, column 0: A".to_owned()), Some("row 1, column 0: empty".to_owned())],
        &format!("{:?}", names("text-empty")?),
    )
}

/// A single text node names its type and value, and an empty one says "empty".
#[wasm_bindgen_test]
fn a_single_text_node_names_its_value() -> Result<(), String> {
    let (scene, node) = scene_with("text-single", text(&["Hello"]))?;
    let group = nth_group("text-single", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("text = Hello"),
        &format!("{:?}", group.get_attribute("aria-label")),
    )?;
    scene
        .set_data_values(node, NodeValues::Text(strings(&[""])))
        .map_err(|e| e.to_string())?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("text = empty"),
        &format!("{:?}", group.get_attribute("aria-label")),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Content that does not match, or with a control character, is refused before anything is drawn.
#[wasm_bindgen_test]
fn text_that_does_not_match_its_format_or_has_a_control_character_is_refused() -> Result<(), String> {
    let svg = make_svg("text-invalid", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let cases = [
        DataNodeContent::new(NodeValues::Text(strings(&["a"])), DataFormat::Decimal),
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Text),
        text(&["a\nb"]),
        text(&["a\tb"]),
        text(&["a"]).with_cell_width_hint("9\n9"),
    ];
    for content in cases {
        let outcome = scene.add_data_node(Point::new(10.0, 10.0), content);
        check(
            matches!(outcome, Err(Error::InvalidTextContent)),
            &format!("expected InvalidTextContent, got {outcome:?}"),
        )?;
    }
    check(nth_group("text-invalid", 0).is_err(), "a rejected call should draw nothing")
}

/// A replacement with a control character, or of another kind, is refused and the node is left as it was.
#[wasm_bindgen_test]
fn a_bad_replacement_is_refused_and_leaves_the_node_alone() -> Result<(), String> {
    let (scene, node) = scene_with("text-bad-replace", text(&["a", "b"]))?;
    for values in [
        NodeValues::Text(strings(&["a", "x\ny"])),
        NodeValues::Text(strings(&["a"])),
        NodeValues::U8(vec![1, 2]),
    ] {
        let outcome = scene.set_data_values(node, values);
        check(
            matches!(outcome, Err(Error::IncompatibleNodeValues(_))),
            &format!("{outcome:?}"),
        )?;
    }
    check(
        texts_of("text-bad-replace")? == ["a", "b"],
        "the node still shows what it showed",
    )
}

/// No operator has a meaning for text.
#[wasm_bindgen_test]
fn no_operator_accepts_text() -> Result<(), String> {
    let (scene, node) = scene_with("text-operator", text(&["a"]))?;
    let outcome = scene.add_unary_operator_node(Point::new(300.0, 10.0), UnaryOperator::Not, node, text(&["b"]));
    check(
        matches!(&outcome, Err(Error::OperatorOnText { operator }) if operator == "NOT"),
        &format!("expected OperatorOnText for NOT, got {outcome:?}"),
    )
}
