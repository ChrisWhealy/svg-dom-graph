//! `ref_name`/`current_ref_name`: the label a node is called by, and how a live `Selection` extends it.

use super::support::make_svg;
use crate::{
    scene::{ArithmeticOperator, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection, UnaryOperator},
    test_support::check,
};
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// A plain label node's own `ref_name` is its own visible text — the same string a later node's own description
/// would call it by.
#[wasm_bindgen_test]
fn a_plain_nodes_own_ref_name_is_its_own_label() -> Result<(), String> {
    let svg = make_svg("ref-name-plain");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "src")
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(node).unwrap().ref_name == "src",
        &format!("expected ref_name \"src\", got {:?}", inner.node_handle(node).unwrap().ref_name),
    )
}

/// A named data node's own `ref_name` is the name it was given, not its own type or formatted value.
#[wasm_bindgen_test]
fn a_named_data_nodes_own_ref_name_is_its_own_name() -> Result<(), String> {
    let svg = make_svg("ref-name-named-data");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_named_data_node(
            Point::origin(),
            "B",
            DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(node).unwrap().ref_name == "B",
        &format!("expected ref_name \"B\", got {:?}", inner.node_handle(node).unwrap().ref_name),
    )
}

/// An unnamed data node has no name to fall back on. So its own `ref_name` falls back to its own type name
/// instead. True for a single value and for a multi-value grid alike.
#[wasm_bindgen_test]
fn an_unnamed_data_nodes_own_ref_name_falls_back_to_its_own_type_name() -> Result<(), String> {
    let svg = make_svg("ref-name-unnamed-data");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let single = scene
        .add_data_node(
            Point::origin(),
            DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let grid = scene
        .add_data_node(
            Point::new(100.0, 0.0),
            DataNodeContent::new(NodeValues::U16(vec![1, 2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(single).unwrap().ref_name == "u32",
        &format!(
            "expected the single-value node's own ref_name \"u32\", got {:?}",
            inner.node_handle(single).unwrap().ref_name
        ),
    )?;
    check(
        inner.node_handle(grid).unwrap().ref_name == "u16",
        &format!(
            "expected the multi-value node's own ref_name \"u16\", got {:?}",
            inner.node_handle(grid).unwrap().ref_name
        ),
    )
}

/// A unary, binary, or arithmetic operator node's own `ref_name` is its own operator label — `"NOT"`, `"XOR"`,
/// `"ROTR 1"`. That is what a later stage would call it by, not the type of its own result.
#[wasm_bindgen_test]
fn an_operator_nodes_own_ref_name_is_its_own_operator_label() -> Result<(), String> {
    let svg = make_svg("ref-name-operator");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let operand = scene
        .add_data_node(
            Point::origin(),
            DataNodeContent::new(NodeValues::U32(vec![0x0F0F_0F0F]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let not_node = scene
        .add_unary_operator_node(
            Point::new(100.0, 0.0),
            UnaryOperator::Not,
            operand,
            DataNodeContent::new(NodeValues::U32(vec![!0x0F0F_0F0Fu32]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;

    let a = scene
        .add_data_node(
            Point::new(0.0, 100.0),
            DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(100.0, 100.0),
            DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let sub_node = scene
        .add_arithmetic_operator_node(
            Point::new(200.0, 100.0),
            ArithmeticOperator::Subtract,
            (a, b),
            DataNodeContent::new(NodeValues::U8(vec![6]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(not_node).unwrap().ref_name == "NOT",
        &format!(
            "expected the unary operator's own ref_name \"NOT\", got {:?}",
            inner.node_handle(not_node).unwrap().ref_name
        ),
    )?;
    check(
        inner.node_handle(sub_node).unwrap().ref_name == "SUB",
        &format!(
            "expected the arithmetic operator's own ref_name \"SUB\", got {:?}",
            inner.node_handle(sub_node).unwrap().ref_name
        ),
    )
}

/// A 2-row, 3-column named data node, for exercising every `Selection` variant against `current_ref_name`.
fn make_selectable_grid(scene: &Scene) -> crate::NodeId {
    scene
        .add_named_data_node(
            Point::origin(),
            "A",
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .unwrap()
}

/// With no selection, `current_ref_name` is just `ref_name` — the same name construction gave the node.
#[wasm_bindgen_test]
fn current_ref_name_with_no_selection_is_just_ref_name() -> Result<(), String> {
    let svg = make_svg("current-ref-name-none");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A", &format!("expected \"A\", got {got:?}"))
}

/// A selected flat cell appends its own index in parentheses — one index, the notation a 1-D array's own step uses.
#[wasm_bindgen_test]
fn current_ref_name_for_a_selected_cell_appends_its_flat_index() -> Result<(), String> {
    let svg = make_svg("current-ref-name-cell");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);
    scene.set_selection(node, Selection::Cell(4)).map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A(4)", &format!("expected \"A(4)\", got {got:?}"))
}

/// A selected row with no focus names the whole row, not a specific element within it.
#[wasm_bindgen_test]
fn current_ref_name_for_a_selected_row_names_the_whole_row() -> Result<(), String> {
    let svg = make_svg("current-ref-name-row");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);
    scene
        .set_selection(node, Selection::Row { row: 1, col: None })
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A(row 1)", &format!("expected \"A(row 1)\", got {got:?}"))
}

/// A row with its own focused column reads as `A(row, col)` matrix notation, not the flat-cell format.
#[wasm_bindgen_test]
fn current_ref_name_for_a_focused_cell_in_a_row_uses_matrix_notation() -> Result<(), String> {
    let svg = make_svg("current-ref-name-row-focus");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A(1, 2)", &format!("expected \"A(1, 2)\", got {got:?}"))
}

/// A selected column with no focus names the whole column, not a specific element within it.
#[wasm_bindgen_test]
fn current_ref_name_for_a_selected_column_names_the_whole_column() -> Result<(), String> {
    let svg = make_svg("current-ref-name-column");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);
    scene
        .set_selection(node, Selection::Column { col: 2, row: None })
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A(column 2)", &format!("expected \"A(column 2)\", got {got:?}"))
}

/// A column with its own focused row also reads as `A(row, col)` matrix notation, matching the row/focus case.
#[wasm_bindgen_test]
fn current_ref_name_for_a_focused_cell_in_a_column_uses_matrix_notation() -> Result<(), String> {
    let svg = make_svg("current-ref-name-column-focus");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = make_selectable_grid(&scene);
    scene
        .set_selection(node, Selection::Column { col: 2, row: Some(1) })
        .map_err(|e| e.to_string())?;

    let inner = scene.inner.borrow();
    let got = inner.node_handle(node).unwrap().current_ref_name();
    check(got == "A(1, 2)", &format!("expected \"A(1, 2)\", got {got:?}"))
}
