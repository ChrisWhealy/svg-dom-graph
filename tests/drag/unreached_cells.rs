//! `Scene::set_unreached_cells` marks the cells whose values are not computed yet. They are drawn faint, box and digits
//! alike. It is independent of `Scene::set_selection` and `Scene::set_secondary_selection`, which win over it on a cell,
//! and it is described in the node's own `aria-label`.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

const FAINT: &str = "0.35";
const FOCUS_FILL: &str = "#ff6b4a";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 5-column, 2-row `u8` grid in a fresh scene, with its inner cell `<rect>`s and `<text>`s, and its own `<g>`.
struct Grid {
    scene: Scene,
    node: svg_dom_graph::NodeId,
    rects: Vec<web_sys::Element>,
    texts: Vec<web_sys::Element>,
    group: web_sys::Element,
}

fn elements(group: &web_sys::Element, selector: &str, skip: u32) -> Result<Vec<web_sys::Element>, String> {
    let nodes = group.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
    (skip..nodes.length())
        .map(|i| {
            nodes
                .get(i)
                .ok_or("query_selector_all reported a longer length than it returned".to_owned())?
                .dyn_into::<web_sys::Element>()
                .map_err(|_| format!("{selector} is not an Element"))
        })
        .collect()
}

fn grid(id: &str) -> Result<Grid, String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((1..=10).collect()), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(5)),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group(id, 0)?;
    // Index `0` of the rects is the outer box, which is not a cell.
    let rects = elements(&group, "rect", 1)?;
    let texts = elements(&group, "text", 0)?;
    Ok(Grid {
        scene,
        node,
        rects,
        texts,
        group,
    })
}

fn opacity(element: &web_sys::Element) -> String {
    element.get_attribute("opacity").unwrap_or_default()
}

fn label(group: &web_sys::Element) -> String {
    group.get_attribute("aria-label").unwrap_or_default()
}

fn is_faint(grid: &Grid, i: usize) -> bool {
    opacity(&grid.rects[i]) == FAINT && opacity(&grid.texts[i]) == FAINT
}

fn is_full(grid: &Grid, i: usize) -> bool {
    let full = |e: &web_sys::Element| matches!(opacity(e).as_str(), "" | "1");
    full(&grid.rects[i]) && full(&grid.texts[i])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A marked cell's box and digits both go faint, an unmarked cell is untouched, and clearing restores the cell.
#[wasm_bindgen_test]
fn marked_cells_go_faint_box_and_digits_and_clearing_restores_them() -> Result<(), String> {
    let g = grid("unreached-basic")?;
    g.scene.set_unreached_cells(g.node, &[6, 7, 8, 9]).map_err(|e| e.to_string())?;

    for i in 0..10 {
        let marked = i >= 6;
        check(
            if marked { is_faint(&g, i) } else { is_full(&g, i) },
            &format!("cell {i}: opacity {:?} / {:?}", opacity(&g.rects[i]), opacity(&g.texts[i])),
        )?;
    }

    g.scene.set_unreached_cells(g.node, &[]).map_err(|e| e.to_string())?;
    for i in 0..10 {
        check(is_full(&g, i), &format!("cell {i} was left faint after clearing"))?;
    }
    check(!label(&g.group).contains("not yet computed"), &label(&g.group))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The node's own accessible name says which cells are not yet computed, as a range when there are three or more in a
/// row, and the tooltip says the same.
#[wasm_bindgen_test]
fn the_accessible_name_and_tooltip_name_the_cells() -> Result<(), String> {
    let g = grid("unreached-label")?;
    g.scene
        .set_unreached_cells(g.node, &[2, 6, 7, 8, 9])
        .map_err(|e| e.to_string())?;

    let name = label(&g.group);
    check(name.ends_with(", not yet computed: cells 2, 6 to 9"), &name)?;
    let title = g
        .group
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .and_then(|t| t.text_content())
        .unwrap_or_default();
    check(
        title == name,
        &format!("the tooltip {title:?} does not match the name {name:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A selected cell shows at full strength even while it is marked, and goes faint again when the selection moves on.
#[wasm_bindgen_test]
fn a_selection_wins_over_unreached_and_gives_the_cell_back_when_it_moves() -> Result<(), String> {
    let g = grid("unreached-selection")?;
    g.scene.set_unreached_cells(g.node, &[3, 4, 5]).map_err(|e| e.to_string())?;

    g.scene.set_selection(g.node, Selection::Cell(4)).map_err(|e| e.to_string())?;
    check(
        g.rects[4].get_attribute("fill").as_deref() == Some(FOCUS_FILL) && is_full(&g, 4),
        "the selected cell is not at full strength",
    )?;
    check(is_faint(&g, 3) && is_faint(&g, 5), "its neighbours lost their mark")?;

    g.scene.set_selection(g.node, Selection::Cell(0)).map_err(|e| e.to_string())?;
    check(
        is_faint(&g, 4),
        "the cell did not go faint again once the selection moved off it",
    )?;
    check(is_full(&g, 0), "the newly selected cell is not at full strength")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A secondary cell also wins over unreached, and the two sets stay independent of each other.
#[wasm_bindgen_test]
fn a_secondary_cell_wins_over_unreached_and_the_two_sets_are_independent() -> Result<(), String> {
    let g = grid("unreached-secondary")?;
    g.scene.set_unreached_cells(g.node, &[1, 2, 3]).map_err(|e| e.to_string())?;
    g.scene.set_secondary_selection(g.node, &[2]).map_err(|e| e.to_string())?;
    check(
        is_full(&g, 2) && is_faint(&g, 1) && is_faint(&g, 3),
        "the secondary cell is not at full strength",
    )?;

    // Clearing the secondary set hands cell 2 back to unreached, without touching the unreached set itself.
    g.scene.set_secondary_selection(g.node, &[]).map_err(|e| e.to_string())?;
    check(is_faint(&g, 2), "cell 2 did not go faint again")?;
    check(label(&g.group).contains("not yet computed: cells 1 to 3"), &label(&g.group))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// New values written into a marked cell leave it faint until it is taken out of the set.
#[wasm_bindgen_test]
fn writing_a_value_does_not_clear_the_mark() -> Result<(), String> {
    let g = grid("unreached-values")?;
    g.scene.set_unreached_cells(g.node, &[9]).map_err(|e| e.to_string())?;
    g.scene
        .set_data_values(g.node, NodeValues::U8((11..=20).collect()))
        .map_err(|e| e.to_string())?;
    check(is_faint(&g, 9), "writing a value cleared the mark")?;

    g.scene.set_unreached_cells(g.node, &[]).map_err(|e| e.to_string())?;
    check(is_full(&g, 9), "taking the cell out of the set did not restore it")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-value node is one cell: the whole box goes faint, and its name just says it is not yet computed.
#[wasm_bindgen_test]
fn a_single_value_node_goes_faint_as_a_whole() -> Result<(), String> {
    let svg = make_svg("unreached-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![0]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let group = nth_group("unreached-single", 0)?;

    scene.set_unreached_cells(node, &[0]).map_err(|e| e.to_string())?;
    let rect = elements(&group, "rect", 0)?.remove(0);
    let text = elements(&group, "text", 0)?.remove(0);
    check(
        opacity(&rect) == FAINT && opacity(&text) == FAINT,
        "the single cell is not faint",
    )?;
    check(label(&group).ends_with(", not yet computed"), &label(&group))?;

    scene.set_unreached_cells(node, &[]).map_err(|e| e.to_string())?;
    check(
        opacity(&rect) == "1" && opacity(&text) == "1",
        "clearing did not restore the single cell",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An index out of range, or a plain label node, is rejected, and a rejected call changes nothing.
#[wasm_bindgen_test]
fn out_of_range_and_label_nodes_are_rejected_and_change_nothing() -> Result<(), String> {
    let g = grid("unreached-reject")?;
    let plain = g
        .scene
        .add_node(Point::new(200.0, 150.0), Size::new(60.0, 30.0), "plain")
        .map_err(|e| e.to_string())?;
    g.scene.set_unreached_cells(g.node, &[8]).map_err(|e| e.to_string())?;
    let before = label(&g.group);

    check(
        matches!(
            g.scene.set_unreached_cells(g.node, &[2, 10]),
            Err(Error::InvalidSelection(_, Selection::Cell(10)))
        ),
        "an out-of-range index was accepted",
    )?;
    check(
        matches!(g.scene.set_unreached_cells(plain, &[0]), Err(Error::InvalidSelection(..))),
        "a plain label node was accepted",
    )?;
    check(is_faint(&g, 8) && is_full(&g, 2), "a rejected call changed the cells")?;
    check(label(&g.group) == before, "a rejected call changed the label")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Order and duplicates do not matter, and an identical set is accepted again without changing anything.
#[wasm_bindgen_test]
fn order_and_duplicates_do_not_matter_and_a_repeat_is_harmless() -> Result<(), String> {
    let g = grid("unreached-order")?;
    g.scene.set_unreached_cells(g.node, &[9, 7, 8, 7]).map_err(|e| e.to_string())?;
    let first = label(&g.group);
    g.scene.set_unreached_cells(g.node, &[7, 8, 9]).map_err(|e| e.to_string())?;
    check(label(&g.group) == first, &label(&g.group))?;
    check(first.ends_with(", not yet computed: cells 7 to 9"), &first)
}
