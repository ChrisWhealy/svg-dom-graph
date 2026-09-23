//! `Scene::add_edge`/`add_edge_with`: the relationship text each new edge appends to both of its own endpoints'
//! `aria-label`/`<title>`, so a `<path>` is never the only place conveying which node feeds which.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{ArithmeticOperator, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn title_of(element: &web_sys::Element) -> Result<Option<String>, String> {
    Ok(element
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .map(|title| title.text_content().unwrap_or_default()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A plain label node starts with no `aria-label` at all — its own visible text already serves as its accessible
/// name. `add_edge` gives both endpoints of a new edge one, seeded with their own name, so neither loses it.
#[wasm_bindgen_test]
fn add_edge_gives_both_endpoints_a_relationship_clause() -> Result<(), String> {
    let svg = make_svg("relationships-plain", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let group_a = nth_group("relationships-plain", 0)?;
    let group_b = nth_group("relationships-plain", 1)?;
    check(
        group_a.get_attribute("aria-label").as_deref() == Some("A. Output to B."),
        &format!("unexpected source aria-label: {:?}", group_a.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group_a)?.as_deref() == Some("A. Output to B."),
        &format!("unexpected source <title>: {:?}", title_of(&group_a)?),
    )?;
    check(
        group_b.get_attribute("aria-label").as_deref() == Some("B. Input from A."),
        &format!("unexpected destination aria-label: {:?}", group_b.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group_b)?.as_deref() == Some("B. Input from A."),
        &format!("unexpected destination <title>: {:?}", title_of(&group_b)?),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A source feeding more than one destination gets one "Output to" clause per edge, in the order each edge was
/// added. These never merge into one shared clause.
#[wasm_bindgen_test]
fn a_source_feeding_two_destinations_gets_two_separate_output_clauses() -> Result<(), String> {
    let svg = make_svg("relationships-fan-out", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let x = scene
        .add_node(Point::new(140.0, 0.0), Size::new(80.0, 40.0), "X")
        .map_err(|e| e.to_string())?;
    let y = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "Y")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, x).map_err(|e| e.to_string())?;
    scene.add_edge(a, y).map_err(|e| e.to_string())?;

    let group_a = nth_group("relationships-fan-out", 0)?;
    check(
        group_a.get_attribute("aria-label").as_deref() == Some("A. Output to X. Output to Y."),
        &format!("unexpected aria-label: {:?}", group_a.get_attribute("aria-label")),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A binary operator node's own two auto-wired input edges append one "Input from" clause per operand, in the
/// same order the operator constructor received them. These never merge into one shared "Inputs: A, B" clause.
#[wasm_bindgen_test]
fn a_binary_operators_own_two_auto_wired_inputs_each_append_their_own_clause() -> Result<(), String> {
    let svg = make_svg("relationships-operator", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_named_data_node(
            Point::new(0.0, 0.0),
            "A",
            DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_named_data_node(
            Point::new(0.0, 90.0),
            "B",
            DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![6]), DataFormat::Decimal);
    scene
        .add_arithmetic_operator_node(Point::new(200.0, 40.0), ArithmeticOperator::Subtract, (a, b), result)
        .map_err(|e| e.to_string())?;

    let group_op = nth_group("relationships-operator", 2)?;
    check(
        group_op.get_attribute("aria-label").as_deref() == Some("SUB result = 6. Input from A. Input from B."),
        &format!("unexpected operator aria-label: {:?}", group_op.get_attribute("aria-label")),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A relationship clause survives a later `Scene::set_selection` on the same node. `set_selection` truncates back
/// to `base_label_len`, which a relationship clause already advanced past — so it truncates away only a stale
/// selection description, never a relationship clause added since.
#[wasm_bindgen_test]
fn set_selection_after_an_edge_keeps_the_relationship_clause() -> Result<(), String> {
    let svg = make_svg("relationships-selection", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_named_data_node(
            Point::new(0.0, 0.0),
            "A",
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(|e| e.to_string())?;
    let out = scene
        .add_node(Point::new(140.0, 0.0), Size::new(80.0, 40.0), "OUT")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, out).map_err(|e| e.to_string())?;
    scene.set_selection(a, Selection::Cell(1)).map_err(|e| e.to_string())?;

    let group_a = nth_group("relationships-selection", 0)?;
    check(
        group_a.get_attribute("aria-label").as_deref()
            == Some("A: u8 data grid, 1 row by 3 columns, 3 values. Output to OUT., cell 1 selected"),
        &format!("unexpected aria-label: {:?}", group_a.get_attribute("aria-label")),
    )
}
