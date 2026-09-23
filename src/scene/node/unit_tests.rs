use super::{construction_guard::OperatorConstructionGuard, render_guard::RenderGuard};
use crate::{
    scene::{ArithmeticOperator, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection, UnaryOperator},
    test_support::check,
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A fresh `SvgRoot` in its own container, with a unique `id` so parallel tests in this binary do not collide.
fn make_svg(id: &str) -> SvgRoot {
    let container_id = format!("{id}-container");
    let el = document().create_element("div").unwrap();
    el.set_id(&container_id);
    document().query_selector("body").unwrap().unwrap().append_child(&el).unwrap();

    let svg = SvgRoot::create_in(&container_id, Size::new(200.0, 200.0)).unwrap();
    svg.root.set_id(id);
    svg
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed `RenderGuard` removes both `group` and every element tracked via `track` from the document.
/// `draw_box`/`draw_content_box` can be left in exactly that partial state. This happens when a later fallible step
/// fails, and the `?` operator drops the guard on the way out.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_removes_the_group_and_every_loose_element() -> Result<(), String> {
    let svg = make_svg("render-guard-rollback");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::new(group.clone());
    guard.track(loose.clone());
    drop(guard);

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed RenderGuard dropped",
    )?;
    check(
        loose.as_element().parent_node().is_none(),
        "the loosely tracked element was still attached after an armed RenderGuard dropped",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed guard leaves `group` and every tracked element attached. So a
/// successful `draw_box`/`draw_content_box` call is not accidentally rolled back by its own cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_guard_leaves_the_group_and_every_loose_element_attached() -> Result<(), String> {
    let svg = make_svg("render-guard-disarm");
    let group = svg.group().unwrap();
    let loose = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::new(group.clone());
    guard.track(loose.clone());
    guard.disarm();

    check(
        group.as_element().parent_node().is_some(),
        "group was removed even though the guard was disarmed",
    )?;
    check(
        loose.as_element().parent_node().is_some(),
        "the loosely tracked element was removed even though the guard was disarmed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed guard that tracked nothing beyond `group` itself is still safe. `group.remove()` is a harmless
/// no-op on an already-empty group.
#[wasm_bindgen_test]
fn dropping_an_armed_guard_with_no_loose_elements_only_removes_the_group() -> Result<(), String> {
    let svg = make_svg("render-guard-empty");
    let group = svg.group().unwrap();

    drop(RenderGuard::new(group.clone()));

    check(
        group.as_element().parent_node().is_none(),
        "group was still attached after an armed, otherwise-empty RenderGuard dropped",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `release` stops tracking the most recently tracked node, so an armed guard dropping afterward leaves it exactly
/// where it was — not one of the elements `draw_content_box`'s own per-cell loop has already appended into `group`
/// (whose own removal would remove it anyway), but proved here directly: even a node never appended anywhere must
/// survive, once released, purely because tracking it stopped.
#[wasm_bindgen_test]
fn release_stops_tracking_the_most_recently_tracked_node() -> Result<(), String> {
    let svg = make_svg("render-guard-release");
    let group = svg.group().unwrap();
    let released = svg.rect(Point::origin(), Size::new(10.0, 10.0)).unwrap();

    let mut guard = RenderGuard::new(group.clone());
    guard.track(released.clone());
    guard.release();
    drop(guard);

    check(
        group.as_element().parent_node().is_none(),
        "group should still be removed on drop — release only affects the tracked node, not group itself",
    )?;
    check(
        released.as_element().parent_node().is_some(),
        "release should have stopped tracking the node, but dropping the still-armed guard removed it anyway",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping an armed `OperatorConstructionGuard` removes every edge tracked via `track_edge` — its rendered path,
/// its `edge_handles` entry, and its place in the graph — then the node itself, the same partial state a failure
/// drawing an operator's own auto-wired input edge would otherwise leave behind.
#[wasm_bindgen_test]
fn dropping_an_armed_construction_guard_removes_the_node_and_every_tracked_edge() -> Result<(), String> {
    let svg = make_svg("construction-guard-rollback");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let input = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "input")
        .map_err(|e| e.to_string())?;
    let operator = scene
        .add_node(Point::new(100.0, 0.0), Size::new(40.0, 20.0), "operator")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(input, operator).map_err(|e| e.to_string())?;

    let (node_group, edge_path) = {
        let inner = scene.inner.borrow();
        (
            inner.node_handle(operator).unwrap().group.clone(),
            inner.edge_handle(edge).unwrap().path.clone(),
        )
    };

    let mut guard = OperatorConstructionGuard::new(scene.clone(), operator);
    guard.track_edge(edge);
    drop(guard);

    check(
        node_group.as_element().parent_node().is_none(),
        "the operator node's own group was still attached after an armed OperatorConstructionGuard dropped",
    )?;
    check(
        edge_path.as_element().parent_node().is_none(),
        "the tracked edge's own path was still attached after an armed OperatorConstructionGuard dropped",
    )?;

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(operator).is_none(),
        "node_handles still held the removed operator node",
    )?;
    check(inner.edge_handle(edge).is_none(), "edge_handles still held the removed edge")?;
    check(
        inner.graph.node(operator).is_none(),
        "the graph still held the removed operator node",
    )?;
    check(inner.graph.edge(edge).is_none(), "the graph still held the removed edge")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The counterpart to the test above: a disarmed `OperatorConstructionGuard` leaves the node and every tracked
/// edge exactly as they were. So a fully successful operator creation is not accidentally rolled back by its own
/// cleanup on the way out.
#[wasm_bindgen_test]
fn disarming_a_construction_guard_leaves_the_node_and_every_tracked_edge_in_place() -> Result<(), String> {
    let svg = make_svg("construction-guard-disarm");
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let input = scene
        .add_node(Point::origin(), Size::new(40.0, 20.0), "input")
        .map_err(|e| e.to_string())?;
    let operator = scene
        .add_node(Point::new(100.0, 0.0), Size::new(40.0, 20.0), "operator")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(input, operator).map_err(|e| e.to_string())?;

    let mut guard = OperatorConstructionGuard::new(scene.clone(), operator);
    guard.track_edge(edge);
    guard.disarm();

    let inner = scene.inner.borrow();
    check(
        inner.node_handle(operator).is_some(),
        "node_handles lost the operator node despite a disarmed guard",
    )?;
    check(
        inner.edge_handle(edge).is_some(),
        "edge_handles lost the edge despite a disarmed guard",
    )?;
    check(
        inner.graph.node(operator).is_some(),
        "the graph lost the operator node despite a disarmed guard",
    )?;
    check(
        inner.graph.edge(edge).is_some(),
        "the graph lost the edge despite a disarmed guard",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
