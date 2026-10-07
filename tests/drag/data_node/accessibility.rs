//! Accessible names: the type as text, not only colour, on the group, its tooltip, and each cell's own row and column.

use super::support::{rect_children, text_children, title_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data node's type colour is not the only place its type is recorded. The node's own `<g>` carries a `<title>`: a
/// native browser tooltip when the mouse pointer hovers over any child — the rect or the digits, not just one of them.
/// `<title>` carries the exact same text as `aria-label` below, so the tooltip a sighted mouse user sees always matches
/// what a screen reader announces. It also carries `role="group"` and an `aria-label` summarising it. The explicit role
/// matters: a bare `<g>` has no implicit role, so `aria-label` alone may go unexposed. A caller who cannot distinguish
/// colours can still recover the type this way. Assistive technology cannot perceive fill colour at all either.
///
/// Neither is visible in the rendered digits themselves. Just as importantly, neither corrupts them. `<title>` sits on
/// the group, not as a child of the `<text>` element. So `text_content()` on the digits stays exactly the formatted
/// value, with nothing appended.
#[wasm_bindgen_test]
fn a_single_value_data_node_names_its_type_as_text_not_only_colour() -> Result<(), String> {
    let svg = make_svg("data-node-a11y-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-a11y-single", 0)?;
    check(
        group.get_attribute("role").as_deref() == Some("group"),
        &format!("unexpected role: {:?}", group.get_attribute("role")),
    )?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("u64 = F0 E1 D2 C3 B4 A5 96 87"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group)?.as_deref() == Some("u64 = F0 E1 D2 C3 B4 A5 96 87"),
        &format!("unexpected <title> on the node's own <g>: {:?}", title_of(&group)?),
    )?;

    // The tooltip lives on the group, so it must not have leaked into the digits' own text content.
    let texts = text_children(&group)?;
    check(
        texts[0].text_content().as_deref() == Some("F0 E1 D2 C3 B4 A5 96 87"),
        &format!("digit text content was corrupted by the <title>: {:?}", texts[0].text_content()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The multi-value counterpart of the test above. One `<title>` on the group still applies wherever the mouse pointer
/// hovers, over any cell's rect or digits alike, and still carries the same text as `aria-label`. The group's own
/// `aria-label` — and so `<title>` too — reports how many values the node holds.
#[wasm_bindgen_test]
fn a_multi_value_data_node_names_its_type_on_the_group() -> Result<(), String> {
    let svg = make_svg("data-node-a11y-multi", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAA, 0xBB]), DataFormat::Hexadecimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-a11y-multi", 0)?;
    check(
        group.get_attribute("role").as_deref() == Some("group"),
        &format!("unexpected role: {:?}", group.get_attribute("role")),
    )?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("u8 data grid, 2 rows by 1 column, 2 values"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group)?.as_deref() == Some("u8 data grid, 2 rows by 1 column, 2 values"),
        &format!("unexpected <title> on the node's own <g>: {:?}", title_of(&group)?),
    )?;

    // Neither inner cell rect nor either digit run carries its own separate <title>. One shared title on the group is
    // enough. Putting one on each rect instead would not even work: a rect is a sibling of its own text, not an
    // ancestor of it. So the tooltip still would not appear when hovering over the digits themselves.
    let rects = rect_children(&group)?;
    for (i, cell_rect) in rects[1..].iter().enumerate() {
        check(
            title_of(cell_rect)?.is_none(),
            &format!("expected no <title> on inner cell {i}, found {:?}", title_of(cell_rect)?),
        )?;
    }
    let texts = text_children(&group)?;
    check(
        texts[0].text_content().as_deref() == Some("AA"),
        "digit text content 0 was corrupted",
    )?;
    check(
        texts[1].text_content().as_deref() == Some("BB"),
        "digit text content 1 was corrupted",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 2-D grid's own row/column shape is spelled out on the group, not just implicit in each cell's own `x`/`y`
/// position. Each cell's own `<text>` also names its row and column directly, not just its bare digits.
#[wasm_bindgen_test]
fn a_two_dimensional_grids_own_shape_and_cells_are_named_by_row_and_column() -> Result<(), String> {
    let svg = make_svg("data-node-a11y-grid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-a11y-grid", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("u8 data grid, 2 rows by 3 columns, 6 values"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;

    // Flat index 3 is row 1, column 0 (value 4); flat index 5 is row 1, column 2 (value 6) — row-major, per
    // `GridLayout::Rows(2)`.
    let texts = text_children(&group)?;
    check(
        texts[3].get_attribute("aria-label").as_deref() == Some("row 1, column 0: 4"),
        &format!("unexpected cell 3 aria-label: {:?}", texts[3].get_attribute("aria-label")),
    )?;
    check(
        texts[5].get_attribute("aria-label").as_deref() == Some("row 1, column 2: 6"),
        &format!("unexpected cell 5 aria-label: {:?}", texts[5].get_attribute("aria-label")),
    )
}
