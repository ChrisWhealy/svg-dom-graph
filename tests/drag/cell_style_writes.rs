//! Restyling a cell writes only the attributes whose value changes. Moving the focus from one cell to the next changes
//! two attributes on each of them (the fill and the stroke width), not all six a full restyle would write. The result
//! on screen is the same either way, so the other tests check that. These count the writes.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn grid(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, web_sys::Element), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((1..=10).collect()), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(5)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node, nth_group(id, 0)?))
}

/// The attribute writes `act` makes to `<rect>` and `<text>` elements under `group`, as their attribute names, in
/// order. A write to the group itself, such as its `aria-label`, is not counted.
fn cell_attribute_writes(
    group: &web_sys::Element,
    act: impl FnOnce() -> Result<(), String>,
) -> Result<Vec<String>, String> {
    let names = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen = names.clone();
    let record =
        wasm_bindgen::closure::Closure::wrap(Box::new(move |records: js_sys::Array, _: web_sys::MutationObserver| {
            for r in records.iter() {
                let r = r.unchecked_into::<web_sys::MutationRecord>();
                let element = r.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
                if let (Some(el), Some(name)) = (element, r.attribute_name()) {
                    if matches!(el.local_name().as_str(), "rect" | "text") {
                        seen.borrow_mut().push(name);
                    }
                }
            }
        }) as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
    let observer = web_sys::MutationObserver::new(record.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
    let options = web_sys::MutationObserverInit::new();
    options.set_subtree(true);
    options.set_attributes(true);
    observer.observe_with_options(group, &options).map_err(|e| format!("{e:?}"))?;
    act()?;
    // Delivers the pending records to the callback now, instead of at the next microtask.
    let pending = observer.take_records();
    for r in pending.iter() {
        let r = r.unchecked_into::<web_sys::MutationRecord>();
        let element = r.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        if let (Some(el), Some(name)) = (element, r.attribute_name()) {
            if matches!(el.local_name().as_str(), "rect" | "text") {
                names.borrow_mut().push(name);
            }
        }
    }
    observer.disconnect();
    let out = names.borrow().clone();
    Ok(out)
}

#[wasm_bindgen_test]
fn moving_the_focus_writes_only_the_fill_and_stroke_width_of_each_cell() -> Result<(), String> {
    let (scene, node, group) = grid("style-writes-focus")?;
    scene.set_selection(node, Selection::Cell(0)).map_err(|e| e.to_string())?;
    let writes = cell_attribute_writes(&group, || {
        scene.set_selection(node, Selection::Cell(1)).map_err(|e| e.to_string())
    })?;
    // Two cells change, and each needs its fill and its stroke width: four writes, not twelve.
    check(writes.len() == 4, &format!("expected 4 attribute writes, got {writes:?}"))?;
    check(
        writes.iter().all(|w| w == "fill" || w == "stroke-width"),
        &format!("only the fill and stroke width should change, got {writes:?}"),
    )
}

#[wasm_bindgen_test]
fn marking_a_cell_unreached_writes_only_its_opacity() -> Result<(), String> {
    let (scene, node, group) = grid("style-writes-unreached")?;
    let writes = cell_attribute_writes(&group, || scene.set_unreached_cells(node, &[3]).map_err(|e| e.to_string()))?;
    // The box and its digits each get an opacity, and nothing else.
    check(
        writes == ["opacity", "opacity"],
        &format!("expected the rect's and the text's opacity only, got {writes:?}"),
    )
}

#[wasm_bindgen_test]
fn a_secondary_cell_writes_the_four_attributes_that_differ_from_the_default() -> Result<(), String> {
    let (scene, node, group) = grid("style-writes-secondary")?;
    let writes =
        cell_attribute_writes(&group, || scene.set_secondary_selection(node, &[2]).map_err(|e| e.to_string()))?;
    // Fill, stroke width, stroke colour and dash all differ. Its opacity does not.
    check(
        writes.len() == 4 && !writes.iter().any(|w| w == "opacity"),
        &format!("got {writes:?}"),
    )
}

#[wasm_bindgen_test]
fn a_single_value_node_is_still_restyled_in_full() -> Result<(), String> {
    let svg = make_svg("style-writes-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![5]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("style-writes-single", 0)?;
    // `Scene::set_focus` styles the same rectangle, so a full restyle puts every attribute back.
    scene.set_focus(node, true).map_err(|e| e.to_string())?;
    scene.set_selection(node, Selection::Cell(0)).map_err(|e| e.to_string())?;
    scene.set_selection(node, Selection::None).map_err(|e| e.to_string())?;
    let rect = group.query_selector("rect").map_err(|e| format!("{e:?}"))?.ok_or("no rect")?;
    check(
        rect.get_attribute("stroke").is_some(),
        "a full restyle should leave the stroke written",
    )?;
    let writes = cell_attribute_writes(&group, || {
        scene.set_selection(node, Selection::Cell(0)).map_err(|e| e.to_string())
    })?;
    check(
        writes.len() >= 5,
        &format!("a single value is rewritten in full, got {writes:?}"),
    )
}
