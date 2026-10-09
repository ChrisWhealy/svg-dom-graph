//! `Scene::set_data_values` replaces the values a data node shows, in place, whether it holds a grid or a single value.
//! The cells keep their size, shape, selection and colours. Each cell's own text and accessible name follow the new
//! values, and so does a single value's own accessible name and tooltip.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection, UnaryOperator},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn cell_texts(group: &web_sys::Element) -> Result<Vec<String>, String> {
    let nodes = group.query_selector_all("text").map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::new();
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a longer length than it returned")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| "text is not an Element".to_owned())?;
        out.push(el.text_content().unwrap_or_default());
    }
    Ok(out)
}

fn grid(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, web_sys::Element), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(3)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node, nth_group(id, 0)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each cell's own text and accessible name follow the new values, and the selection survives.
#[wasm_bindgen_test]
fn new_values_rewrite_each_cells_text_and_accessible_name_and_keep_the_selection() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-basic")?;
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;
    let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;

    scene
        .set_data_values(node, NodeValues::U8(vec![9, 8, 7, 6, 5, 4]))
        .map_err(|e| e.to_string())?;

    check(
        cell_texts(&group)? == ["9", "8", "7", "6", "5", "4"],
        &format!("{:?}", cell_texts(&group)?),
    )?;
    let named = group
        .query_selector_all("text[aria-label]")
        .map_err(|e| format!("{e:?}"))?
        .get(5)
        .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
        .and_then(|e| e.get_attribute("aria-label"))
        .unwrap_or_default();
    check(named == "row 1, column 2: 4", &named)?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
        "the node's own rect changed",
    )?;
    check(
        group
            .get_attribute("aria-label")
            .unwrap_or_default()
            .contains("row 1 selected, column 2 focused"),
        "the selection description was lost",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A different width or count is rejected, and a rejected call changes nothing.
#[wasm_bindgen_test]
fn incompatible_values_are_rejected_and_change_nothing() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-reject")?;
    for bad in [
        NodeValues::U16(vec![1, 2, 3, 4, 5, 6]),
        NodeValues::U8(vec![1, 2, 3]),
        NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7]),
    ] {
        check(
            matches!(scene.set_data_values(node, bad), Err(Error::IncompatibleNodeValues(_))),
            "an incompatible set of values was accepted",
        )?;
    }
    check(
        cell_texts(&group)? == ["1", "2", "3", "4", "5", "6"],
        "a rejected call changed the cells",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operator node's own result and a plain label node are rejected, and a rejected call changes nothing.
#[wasm_bindgen_test]
fn operator_results_and_label_nodes_are_rejected() -> Result<(), String> {
    let svg = make_svg("set-values-rejected", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let operand = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let operator = scene
        .add_unary_operator_node(
            Point::new(200.0, 10.0),
            UnaryOperator::Not,
            operand,
            DataNodeContent::new(NodeValues::U32(vec![!1]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let label = scene
        .add_node(Point::new(10.0, 150.0), Size::new(60.0, 30.0), "plain")
        .map_err(|e| e.to_string())?;
    let before = nth_group("set-values-rejected", 1)?.get_attribute("aria-label");

    for (name, node) in [("an operator node", operator), ("a plain label node", label)] {
        check(
            matches!(
                scene.set_data_values(node, NodeValues::U32(vec![2])),
                Err(Error::IncompatibleNodeValues(_))
            ),
            &format!("{name} was accepted"),
        )?;
    }
    check(
        nth_group("set-values-rejected", 1)?.get_attribute("aria-label") == before,
        "a rejected call changed the operator node",
    )?;
    // The rejected call must not have half-applied: it checked the node before touching its content, so selecting a cell
    // of the operator, which reads that content, still succeeds.
    scene.set_selection(operator, Selection::Cell(0)).map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-value node is replaced in place: its text, accessible name and tooltip all quote the new value, and its
/// size, selection description and relationship clauses survive.
#[wasm_bindgen_test]
fn a_single_value_node_is_replaced_in_place_with_its_name_tooltip_and_clauses_kept() -> Result<(), String> {
    let svg = make_svg("set-values-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![0]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let sink = scene
        .add_node(Point::new(200.0, 10.0), Size::new(80.0, 40.0), "sink")
        .map_err(|e| e.to_string())?;
    scene.add_edge(node, sink).map_err(|e| e.to_string())?;
    scene.set_selection(node, Selection::Cell(0)).map_err(|e| e.to_string())?;
    let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;
    let group = nth_group("set-values-single", 0)?;
    check(
        group
            .get_attribute("aria-label")
            .unwrap_or_default()
            .starts_with("u32 = 00 00 00 00. Output to sink"),
        &format!("unexpected starting label: {:?}", group.get_attribute("aria-label")),
    )?;

    scene
        .set_data_values(node, NodeValues::U32(vec![0xDEAD_BEEF]))
        .map_err(|e| e.to_string())?;

    check(cell_texts(&group)? == ["DE AD BE EF"], &format!("{:?}", cell_texts(&group)?))?;
    let label = group.get_attribute("aria-label").unwrap_or_default();
    check(
        label.starts_with("u32 = DE AD BE EF. Output to sink.") && label.contains("cell 0 selected"),
        &format!("the label lost its clauses or its selection: {label:?}"),
    )?;
    let title = group
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .and_then(|t| t.text_content())
        .unwrap_or_default();
    check(
        title == label,
        &format!("the tooltip {title:?} does not match the label {label:?}"),
    )?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
        "the node's own rect changed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A named single-value node keeps its name in the accessible name, and a later value change replaces only the value. A
/// second change after a relationship was added still finds it.
#[wasm_bindgen_test]
fn a_named_single_value_node_keeps_its_name_across_repeated_replacements() -> Result<(), String> {
    let svg = make_svg("set-values-named", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_named_data_node(
            Point::new(10.0, 10.0),
            "temp1",
            DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("set-values-named", 0)?;

    for value in [0x0000_0002, 0xFFFF_FFFF, 0] {
        scene
            .set_data_values(node, NodeValues::U32(vec![value]))
            .map_err(|e| e.to_string())?;
        let bytes: Vec<String> = value.to_be_bytes().iter().map(|b| format!("{b:02X}")).collect();
        let expected = format!("temp1: u32 = {}", bytes.join(" "));
        check(
            group.get_attribute("aria-label").as_deref() == Some(expected.as_str()),
            &format!("expected {expected:?}, got {:?}", group.get_attribute("aria-label")),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single value of another width is rejected, and a rejected call leaves the text and label as they were.
#[wasm_bindgen_test]
fn a_single_value_of_another_width_is_rejected_and_changes_nothing() -> Result<(), String> {
    let svg = make_svg("set-values-single-bad", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![7]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("set-values-single-bad", 0)?;
    let label = group.get_attribute("aria-label");

    for bad in [NodeValues::U8(vec![1]), NodeValues::U32(vec![1, 2])] {
        check(
            matches!(scene.set_data_values(node, bad), Err(Error::IncompatibleNodeValues(_))),
            "a different width or count was accepted",
        )?;
    }
    check(cell_texts(&group)? == ["00 00 00 07"], "a rejected call changed the text")?;
    check(group.get_attribute("aria-label") == label, "a rejected call changed the label")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `PlainText` node is a single value too. Replacing it with another string of the same length rewrites its text and
/// its accessible name.
#[wasm_bindgen_test]
fn a_plain_text_node_is_replaced_with_a_string_of_the_same_length() -> Result<(), String> {
    let svg = make_svg("set-values-plain-text", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(b"abc".to_vec()), DataFormat::PlainText),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("set-values-plain-text", 0)?;

    scene
        .set_data_values(node, NodeValues::U8(b"xyz".to_vec()))
        .map_err(|e| e.to_string())?;
    check(cell_texts(&group)? == ["xyz"], &format!("{:?}", cell_texts(&group)?))?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("text = xyz"),
        &format!("{:?}", group.get_attribute("aria-label")),
    )?;
    check(
        matches!(
            scene.set_data_values(node, NodeValues::U8(b"too long".to_vec())),
            Err(Error::IncompatibleNodeValues(_))
        ),
        "a string of another length was accepted",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many DOM mutations (text, attribute or child changes) `group` sees while `act` runs.
fn mutations_during(group: &web_sys::Element, act: impl FnOnce() -> Result<(), String>) -> Result<u32, String> {
    let ignore = wasm_bindgen::closure::Closure::wrap(Box::new(|_: js_sys::Array, _: web_sys::MutationObserver| {})
        as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
    let observer = web_sys::MutationObserver::new(ignore.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
    let options = web_sys::MutationObserverInit::new();
    options.set_child_list(true);
    options.set_subtree(true);
    options.set_attributes(true);
    options.set_character_data(true);
    observer.observe_with_options(group, &options).map_err(|e| format!("{e:?}"))?;
    act()?;
    let seen = observer.take_records().length();
    observer.disconnect();
    Ok(seen)
}

/// Replacing a node's values with the ones it already holds leaves the document alone.
#[wasm_bindgen_test]
fn replacing_with_identical_values_changes_nothing_in_the_document() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-identical")?;
    let seen = mutations_during(&group, || {
        scene
            .set_data_values(node, NodeValues::U8(vec![1, 2, 3, 4, 5, 6]))
            .map_err(|e| e.to_string())
    })?;
    check(seen == 0, &format!("expected no DOM mutations, saw {seen}"))
}

/// Only the cell whose value changed is rewritten, so the others keep their text and accessible name untouched.
#[wasm_bindgen_test]
fn only_the_changed_cell_is_rewritten() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-partial")?;
    let seen = mutations_during(&group, || {
        scene
            .set_data_values(node, NodeValues::U8(vec![1, 2, 3, 4, 50, 6]))
            .map_err(|e| e.to_string())
    })?;
    // One cell: its text changes and its `aria-label` is set.
    check(
        seen > 0 && seen <= 3,
        &format!("expected a single cell's own few mutations, saw {seen}"),
    )?;
    let texts = cell_texts(&group)?;
    check(
        texts.contains(&"50".to_owned()) && texts.contains(&"4".to_owned()),
        &format!("got {texts:?}"),
    )
}

/// A single value's accessible name quotes the value, so it follows a change, and is left alone when there is none.
#[wasm_bindgen_test]
fn a_single_value_is_rewritten_only_when_it_changes() -> Result<(), String> {
    let svg = make_svg("set-values-single-skip", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![7]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("set-values-single-skip", 0)?;
    let same = mutations_during(&group, || {
        scene.set_data_values(node, NodeValues::U8(vec![7])).map_err(|e| e.to_string())
    })?;
    check(same == 0, &format!("an unchanged value should change nothing, saw {same}"))?;
    scene
        .set_data_values(node, NodeValues::U8(vec![9]))
        .map_err(|e| e.to_string())?;
    let label = group.get_attribute("aria-label").unwrap_or_default();
    check(label.contains("u8 = 9"), &format!("got {label:?}"))
}
