//! `Scene::add_data_node`/`add_data_node_with`: a node whose visible content is a [`DataNodeContent`] grid, not a
//! plain text label. Covers rendering, colour-coded value cells, auto-sizing, and `GridLayout` overrides. Also
//! covers empty-content and bad-layout rejection, dragging every cell (not just the box), and ordinary connector
//! routing to/from one.

use crate::common::{
    attr_f64, check, check_close, dispatch_pointer_event, group_translate, make_svg, nth_group, the_connector,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeOptions, NodeValues, Scene},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every `<text>` child of `group`, in document order.
fn text_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "text")
}

/// Every `<rect>` child of `group`, in document order — for a data node, index 0 is always the outer box, and any
/// further entries are the per-value inner cells (see `draw_content_box`'s own doc comment).
fn rect_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "rect")
}

fn elements_matching(group: &web_sys::Element, selector: &str) -> Result<Vec<web_sys::Element>, String> {
    let nodes = group.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::with_capacity(nodes.length() as usize);
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a length longer than it could actually return")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| format!("{selector} is not an Element"))?;
        out.push(el);
    }
    Ok(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single value gets no inner cell box — the outer box itself is filled with the value's own type colour — and
/// its text reads exactly the byte-group hex text the feature request's own example describes: no `0x` prefix,
/// uppercase, single-space byte separation.
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
/// Two or more values each get their own type-coloured inner cell box, inside the node's own unchanged, light
/// blue outer box — the second example from the feature request (two values stack into two rows of one column),
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
/// byte-separated, render far wider than the cell is tall. `GridLayout`/`Automatic`'s own doc comment names this
/// exact case as the reason `GridLayout::MaxColumns` exists.
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
/// `draw_content_box` measures only the widest cell's own rendered text, by character count, to size every cell
/// alike — see that function's own doc comment. Every `Hexadecimal`/`Binary` value of one integer type already
/// renders the same character count regardless of magnitude, so `Decimal` values of genuinely different digit
/// counts are the one scenario that actually exercises "pick the *right* cell to measure," not merely "measuring
/// is skipped for the rest."
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

    // Both cells share one uniform width — the one-digit value's own cell is not narrower than the ten-digit
    // value's.
    let rects = rect_children(&group)?;
    let narrow_width = attr_f64(&rects[1], "width")?;
    let wide_width = attr_f64(&rects[2], "width")?;
    check_close(narrow_width, wide_width)?;

    // That shared width is wide enough for the ten-digit value, not just the one-digit value — proving the
    // longer string, not the shorter one, was the one actually measured.
    let height = attr_f64(&rects[1], "height")?;
    check(
        wide_width > height * 2.0,
        &format!("expected a cell wide enough for a 10-digit value (width={wide_width}, height={height})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Five values under [`GridLayout::Automatic`] render a `3 x 2` grid (see
/// `grid_shape_of_five_values_is_three_rows_of_two_columns` in `model::content::unit_tests`) with the last row
/// only half full — a non-complete final row, rather than the exact multiple of columns every other rendering
/// test here happens to use.
#[wasm_bindgen_test]
fn add_data_node_with_five_values_leaves_the_last_row_incomplete() -> Result<(), String> {
    let svg = make_svg("data-node-five-values", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5]), DataFormat::Decimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-five-values", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 5, &format!("expected 5 texts, found {}", texts.len()))?;

    let rects = rect_children(&group)?;
    check(
        rects.len() == 6,
        &format!("expected 1 outer + 5 inner cell rects, found {}", rects.len()),
    )?;

    let mut xs: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "x").map(|x| x.round() as i64))
        .collect::<Result<_, _>>()?;
    let mut ys: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "y").map(|y| y.round() as i64))
        .collect::<Result<_, _>>()?;
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    check(
        xs.len() == 2,
        &format!("expected 2 distinct column positions for a 3x2 grid, found {}", xs.len()),
    )?;
    check(
        ys.len() == 3,
        &format!("expected 3 distinct row positions for a 3x2 grid, found {}", ys.len()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `<title>` child text, or `None` if `element` has no direct `<title>` child.
fn title_of(element: &web_sys::Element) -> Result<Option<String>, String> {
    Ok(element
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .map(|title| title.text_content().unwrap_or_default()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data node's type colour is not the only place its type is recorded. The node's own `<g>` carries a
/// `<title>`: a native browser tooltip when the mouse pointer hovers over any child — the rect or the digits,
/// not just one of them. `<title>` carries the exact same text as `aria-label` below, so the tooltip a sighted
/// mouse user sees always matches what a screen reader announces.
/// It also carries `role="group"` and an `aria-label` summarising it. The explicit role matters: a bare `<g>`
/// has no implicit role, so `aria-label` alone may go unexposed. A caller who cannot distinguish colours can
/// still recover the type this way. Assistive technology cannot perceive fill colour at all either.
///
/// Neither is visible in the rendered digits themselves. Just as importantly, neither corrupts them. `<title>`
/// sits on the group, not as a child of the `<text>` element. So `text_content()` on the digits stays exactly the
/// formatted value, with nothing appended.
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
/// The multi-value counterpart of the test above. One `<title>` on the group still applies wherever the mouse
/// pointer hovers, over any cell's rect or digits alike, and still carries the same text as `aria-label`. The
/// group's own `aria-label` — and so `<title>` too — reports how many values the node holds.
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

    // Neither inner cell rect nor either digit run carries its own separate <title>. One shared title on the
    // group is enough. Putting one on each rect instead would not even work: a rect is a sibling of its own
    // text, not an ancestor of it. So the tooltip still would not appear when hovering over the digits
    // themselves.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_data_node`/`add_data_node_with` rejects an empty `DataNodeContent` before drawing anything or touching
/// the graph's model — mirrors `add_node_with`'s own `EdgeAnchors(0)` rejection test.
#[wasm_bindgen_test]
fn add_data_node_rejects_empty_content_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-empty", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let empty = DataNodeContent::new(NodeValues::U8(vec![]), DataFormat::Decimal);
    let result = scene.add_data_node(Point::new(10.0, 10.0), empty);
    check(
        matches!(result, Err(Error::EmptyNodeContent)),
        &format!("expected Err(Error::EmptyNodeContent), got {result:?}"),
    )?;
    check(
        nth_group("data-node-empty", 0).is_err(),
        "a rejected add_data_node call left a <g> rendered in the scene",
    )?;

    let valid = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    scene.add_data_node(Point::new(10.0, 10.0), valid).map_err(|e| e.to_string())?;
    nth_group("data-node-empty", 0)?;
    check(
        nth_group("data-node-empty", 1).is_err(),
        "expected exactly one <g> after the rejected call and one valid add_data_node call",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `DataNodeContent::with_layout(GridLayout::MaxColumns(n))` overrides `GridLayout::Automatic`'s own cell-count-only
/// choice. 8 values render as 2 rows of 4 under `Automatic` — see
/// `grid_shape_of_eight_values_prefers_two_rows_of_four` in `model::content::unit_tests`. `MaxColumns(2)` caps
/// that at 2 columns, giving 4 rows of 2 instead. This is checked by counting each inner cell's own distinct
/// `x`/`y` — the row/column count, not any specific pixel value.
#[wasm_bindgen_test]
fn with_layout_max_columns_overrides_automatics_own_shape() -> Result<(), String> {
    let svg = make_svg("data-node-max-columns", Size::new(400.0, 400.0), Size::new(400.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal)
        .with_layout(GridLayout::MaxColumns(2));
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-max-columns", 0)?;
    let rects = rect_children(&group)?;
    check(
        rects.len() == 9,
        &format!("expected 1 outer + 8 inner cell rects, found {}", rects.len()),
    )?;

    // rects[0] is the outer box; the 8 inner cells follow.
    let mut xs: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "x").map(|x| x.round() as i64))
        .collect::<Result<_, _>>()?;
    let mut ys: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "y").map(|y| y.round() as i64))
        .collect::<Result<_, _>>()?;
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    check(
        xs.len() == 2,
        &format!("expected 2 distinct column positions under MaxColumns(2), found {}", xs.len()),
    )?;
    check(
        ys.len() == 4,
        &format!("expected 4 distinct row positions under MaxColumns(2), found {}", ys.len()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_data_node`/`add_data_node_with` rejects a `GridLayout` wrapping `0`, before drawing anything or
/// touching the graph's model. This mirrors `add_data_node_rejects_empty_content_before_touching_the_scene` above.
#[wasm_bindgen_test]
fn add_data_node_rejects_a_zero_grid_layout_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-bad-layout", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    for layout in [GridLayout::Columns(0), GridLayout::Rows(0), GridLayout::MaxColumns(0)] {
        let content = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal).with_layout(layout);
        let result = scene.add_data_node(Point::new(10.0, 10.0), content);
        check(
            matches!(result, Err(Error::InvalidGridLayout(_))),
            &format!("expected Err(Error::InvalidGridLayout(_)) for {layout:?}, got {result:?}"),
        )?;
    }
    check(
        nth_group("data-node-bad-layout", 0).is_err(),
        "a rejected add_data_node call left a <g> rendered in the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a data node moves the whole box — every value's own text and inner cell rect included. This works
/// by moving just the node's own `<g>` `transform`, not by rewriting every cell's own coordinates.
///
/// Every cell is drawn once, at creation, in local coordinates relative to `(0, 0)`. See `draw_content_box`'s own
/// doc comment. So a data node with hundreds of cells moves exactly as cheaply as one with a handful. A pointer
/// move only ever rewrites the group's one `transform`. This checks both halves of that: the group's translate
/// changes by the drag delta, and every cell's own local `x`/`y` stays exactly as it was.
#[wasm_bindgen_test]
fn dragging_a_data_node_moves_the_outer_box_and_every_cell() -> Result<(), String> {
    let svg = make_svg("data-node-drag", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    // 4 values -> a 2x2 grid (see grid_shape), so four inner cells exist to prove all four moved.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    let node = scene
        .add_data_node(Point::new(20.0, 20.0), content)
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;

    let group = nth_group("data-node-drag", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    check(
        rects.len() == 5,
        &format!("expected 1 outer + 4 inner cell rects, found {}", rects.len()),
    )?;
    check(
        texts.len() == 4,
        &format!("expected 4 texts for a 2x2 grid, found {}", texts.len()),
    )?;

    let group_xy_before = group_translate(&group)?;
    let rect_positions_before: Vec<(f64, f64)> = rects
        .iter()
        .map(|r| Ok::<_, String>((attr_f64(r, "x")?, attr_f64(r, "y")?)))
        .collect::<Result<_, _>>()?;
    let text_positions_before: Vec<(f64, f64)> = texts
        .iter()
        .map(|t| Ok::<_, String>((attr_f64(t, "x")?, attr_f64(t, "y")?)))
        .collect::<Result<_, _>>()?;

    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", 140, 125, 1)?;
    dispatch_pointer_event(&group, "pointerup", 140, 125, 1)?;

    // The group's own translate moved by exactly the drag delta.
    let (group_x_after, group_y_after) = group_translate(&group)?;
    check_close(group_x_after, group_xy_before.0 + 40.0)?;
    check_close(group_y_after, group_xy_before.1 + 25.0)?;

    // Every cell's own local coordinates are untouched — the move never rewrote a single one of them.
    for (i, rect) in rects.iter().enumerate() {
        let (before_x, before_y) = rect_positions_before[i];
        check_close(attr_f64(rect, "x")?, before_x)?;
        check_close(attr_f64(rect, "y")?, before_y)?;
    }
    for (i, text) in texts.iter().enumerate() {
        let (before_x, before_y) = text_positions_before[i];
        check_close(attr_f64(text, "x")?, before_x)?;
        check_close(attr_f64(text, "y")?, before_y)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector routes to/from a data node exactly as it would an ordinary label node: the same
/// `boundary_point`/elbow routing logic, since it only ever looks at a node's `Rect`, never its content.
#[wasm_bindgen_test]
fn a_connector_routes_to_a_data_node_like_any_other_node() -> Result<(), String> {
    let svg = make_svg("data-node-connector", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    let b = scene
        .add_data_node(Point::new(200.0, 150.0), content)
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    check(
        crate::common::connector_count("data-node-connector")? == 1,
        "expected exactly one connector between a label node and a data node",
    )?;

    let group_b = nth_group("data-node-connector", 1)?; // B was added second.
    let rect_b = &rect_children(&group_b)?[0]; // the outer box — B holds a single value, so it is the only rect.
    // The rect itself is drawn at local (0, 0); B's world-space box origin lives on the group's own transform.
    let (bx, by) = group_translate(&group_b)?;
    let bw = attr_f64(rect_b, "width")?;
    let bh = attr_f64(rect_b, "height")?;

    let d = crate::common::path_d(&the_connector("data-node-connector")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;
    let side_midpoints = [
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx + bw / 2.0, by),
        (bx + bw / 2.0, by + bh),
    ];
    check(
        side_midpoints
            .iter()
            .any(|&(mx, my)| (end_x - mx).abs() < 0.01 && (end_y - my).abs() < 0.01),
        &format!("connector end ({end_x}, {end_y}) is not the midpoint of any side of the data node's rect"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `EdgeAnchors` configures a data node's own connector fixing points exactly as it would an ordinary node's.
/// Snapping only ever looks at the node's `Rect`, never its content.
///
/// # Expected anchor, worked by hand
///
/// `B` (the data node) holds a single value, so its own box is measured after creation, not assumed.
/// Unconfigured, `edge_anchor` always returns the exact midpoint of whichever side is chosen, regardless of the
/// other endpoint's exact position — see `a_connector_routes_to_a_data_node_like_any_other_node` above, and
/// `edge_anchor`'s own doc comment.
///
/// `A` is placed far enough above and to the left of `B` that `snapped_anchor` still picks `B`'s west side.
/// But the unsnapped ray crosses deep inside its topmost quarter. With `EdgeAnchors(3)`, that side is divided
/// into 4 equal segments. So the connector snaps to the first of the 3 candidates: `B`'s own
/// `(bx, by + bh / 4)`. A plain, unconfigured node would have used the midpoint `(bx, by + bh / 2)` instead.
#[wasm_bindgen_test]
fn a_data_node_with_custom_edge_anchors_snaps_like_any_other_node() -> Result<(), String> {
    let svg = make_svg("data-node-edge-anchors", Size::new(500.0, 300.0), Size::new(500.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    let options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3)));
    let b = scene
        .add_data_node_with(Point::new(250.0, 150.0), content, options)
        .map_err(|e| e.to_string())?;

    let group_b = nth_group("data-node-edge-anchors", 0)?; // B was added first.
    let rect_b = &rect_children(&group_b)?[0]; // B holds a single value, so it is the only rect.
    let (bx, by) = group_translate(&group_b)?;
    let bw = attr_f64(rect_b, "width")?;
    let bh = attr_f64(rect_b, "height")?;
    let (half_w, half_h) = (bw / 2.0, bh / 2.0);
    let b_centre = Point::new(bx + half_w, by + half_h);

    // `snapped_anchor`'s own crossing formula is `centre.y + dy * (half_w / dx.abs())`. Choosing
    // `dy = -0.9 * (half_h / half_w) * dx.abs()` makes the `dx` term cancel out algebraically. That leaves the
    // crossing point fixed at `0.9 * half_h` above B's own centre, deep inside the topmost quarter of its west
    // side. This holds for any `dx` at all, as long as `dx` stays large enough to keep the west side selected.
    let dx: f64 = -300.0;
    let dy = -0.9 * (half_h / half_w) * dx.abs();
    let a_centre = Point::new(b_centre.x + dx, b_centre.y + dy);
    let a_size = Size::new(60.0, 30.0);
    let a_origin = Point::new(a_centre.x - a_size.width / 2.0, a_centre.y - a_size.height / 2.0);
    let a = scene.add_node(a_origin, a_size, "A").map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let d = crate::common::path_d(&the_connector("data-node-edge-anchors")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;
    check_close(end_x, bx)?;
    check_close(end_y, by + bh * 0.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Named data nodes
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// `Scene::add_named_data_node` wraps a single-value node's own box in a further outer box of its own, labelled
/// with the given name — the same "outer box labelled with a name, inset value box beneath it" shape an operator
/// node already draws for its own result. The inner value box stays inset from every outer edge, exactly as an
/// operator's own value cell does.
#[wasm_bindgen_test]
fn a_named_single_value_data_node_wraps_it_in_a_further_labelled_outer_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "A", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-single", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    check(
        rects.len() == 2,
        &format!("expected 1 outer named box + 1 inner value box, found {}", rects.len()),
    )?;
    check(
        texts.len() == 2,
        &format!("expected 1 name label + 1 value text, found {}", texts.len()),
    )?;
    check(
        texts[0].text_content().as_deref() == Some("A"),
        &format!(
            "expected the first text to be the name label \"A\", got {:?}",
            texts[0].text_content()
        ),
    )?;
    check(
        texts[1].text_content().as_deref() == Some("AB"),
        &format!(
            "expected the second text to be the value \"AB\", got {:?}",
            texts[1].text_content()
        ),
    )?;

    let outer = &rects[0];
    let inner = &rects[1];
    let outer_width = attr_f64(outer, "width")?;
    let outer_height = attr_f64(outer, "height")?;
    let inner_x = attr_f64(inner, "x")?;
    let inner_y = attr_f64(inner, "y")?;
    let inner_width = attr_f64(inner, "width")?;
    let inner_height = attr_f64(inner, "height")?;

    check(
        inner_x > 0.0,
        &format!("expected the inner value box's left edge inset from the outer box's own, got x={inner_x}"),
    )?;
    check(
        inner_x + inner_width < outer_width,
        &format!(
            "expected the inner value box's right edge inset from the outer box's own, got {} against an outer \
             width of {outer_width}",
            inner_x + inner_width
        ),
    )?;
    check(
        inner_y > 0.0,
        &format!("expected the inner value box's top edge below the outer box's own label row, got y={inner_y}"),
    )?;
    check(
        inner_y + inner_height < outer_height,
        &format!(
            "expected the inner value box's bottom edge inset from the outer box's own, got {} against an outer \
             height of {outer_height}",
            inner_y + inner_height
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A named single-value node's own `aria-label` reads `"{name}: {type} = {value}"` — the node's own name, its
/// type, and its real formatted value, all as text, not just the type a plain (unnamed) node's own label gives.
/// Its `<title>` — the browser's own mouse-hover tooltip — carries that exact same text.
#[wasm_bindgen_test]
fn a_named_single_value_data_nodes_own_aria_label_includes_the_name_and_the_real_value() -> Result<(), String> {
    let svg = make_svg("data-node-named-aria-label", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xABCD_EF01_2345_6789]), DataFormat::Hexadecimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "B", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-aria-label", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("B: u64 = AB CD EF 01 23 45 67 89"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group)?.as_deref() == Some("B: u64 = AB CD EF 01 23 45 67 89"),
        &format!("unexpected <title>: {:?}", title_of(&group)?),
    )
}

/// Same shape again, but for a multi-value grid: the outer named box wraps the grid's own existing box (itself
/// unchanged — still its own light-blue background plus one coloured `<rect>` per value), rather than replacing it.
#[wasm_bindgen_test]
fn a_named_multi_value_data_node_wraps_the_grid_in_a_further_labelled_outer_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-multi", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), "values", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-multi", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    // 1 outer named box + 1 inner grid box + 4 per-value cells.
    check(
        rects.len() == 6,
        &format!("expected 6 rects (outer named box + grid box + 4 cells), found {}", rects.len()),
    )?;
    // 1 name label + 4 cell values.
    check(
        texts.len() == 5,
        &format!("expected 5 texts (name label + 4 cell values), found {}", texts.len()),
    )?;
    check(
        texts[0].text_content().as_deref() == Some("values"),
        &format!(
            "expected the first text to be the name label \"values\", got {:?}",
            texts[0].text_content()
        ),
    )
}

/// An incoming connector anchors to a named data node's own *outer* named box, never to the inner value box `name`
/// wraps — the same guarantee an operator node's own result cell already gets, for the same reason: a connector
/// landing on the inner box would look like it terminates at the value, not at the named quantity as a whole.
#[wasm_bindgen_test]
fn a_named_data_nodes_own_connector_anchors_to_the_outer_named_box() -> Result<(), String> {
    let svg = make_svg("data-node-named-anchor", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let plain = scene
        .add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "src")
        .map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    let named = scene
        .add_named_data_node(Point::new(200.0, 100.0), "A", content)
        .map_err(|e| e.to_string())?;
    scene.add_edge(plain, named).map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-anchor", 1)?;
    let (nx, ny) = group_translate(&group)?;
    let outer = &rect_children(&group)?[0];
    let outer_width = attr_f64(outer, "width")?;
    let outer_height = attr_f64(outer, "height")?;

    let d = crate::common::path_d(&the_connector("data-node-named-anchor")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;

    // The anchor must land somewhere on the outer box's own perimeter — on one of its four edges — never strictly
    // inside it, where the inner value box sits.
    let on_perimeter = check_close(end_x, nx).is_ok()
        || check_close(end_x, nx + outer_width).is_ok()
        || check_close(end_y, ny).is_ok()
        || check_close(end_y, ny + outer_height).is_ok();
    check(
        on_perimeter,
        &format!(
            "expected the connector to land on the outer named box's own perimeter, got ({end_x}, {end_y}) \
             against an outer box at ({nx}, {ny}) sized {outer_width}x{outer_height}"
        ),
    )
}

/// `add_named_data_node` validates `content` exactly as `add_data_node` does — the shared implementation behind
/// both — before drawing anything or touching the graph.
#[wasm_bindgen_test]
fn add_named_data_node_rejects_empty_content_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-named-empty", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let empty = DataNodeContent::new(NodeValues::U8(vec![]), DataFormat::Decimal);
    let result = scene.add_named_data_node(Point::new(10.0, 10.0), "A", empty);
    check(
        matches!(result, Err(Error::EmptyNodeContent)),
        &format!("expected Err(Error::EmptyNodeContent), got {result:?}"),
    )?;
    check(
        nth_group("data-node-named-empty", 0).is_err(),
        "a rejected add_named_data_node call left a <g> rendered in the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_named_data_node` rejects an empty `name`, and one holding only whitespace, before drawing anything or
/// touching the graph — an accessible name of `": u8 = 12"` names nothing.
#[wasm_bindgen_test]
fn add_named_data_node_rejects_an_empty_or_whitespace_only_name_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("data-node-named-blank", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    for blank in ["", "   ", "\t\n"] {
        let content = DataNodeContent::new(NodeValues::U8(vec![12]), DataFormat::Decimal);
        let result = scene.add_named_data_node(Point::new(10.0, 10.0), blank, content);
        check(
            matches!(result, Err(Error::EmptyNodeName)),
            &format!("name {blank:?}: expected Err(Error::EmptyNodeName), got {result:?}"),
        )?;
    }
    check(
        nth_group("data-node-named-blank", 0).is_err(),
        "a rejected add_named_data_node call left a <g> rendered in the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `name` that is not blank, only padded with surrounding whitespace, is accepted — and used exactly as given,
/// not silently trimmed.
#[wasm_bindgen_test]
fn add_named_data_node_accepts_a_name_padded_with_whitespace_verbatim() -> Result<(), String> {
    let svg = make_svg("data-node-named-padded", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let content = DataNodeContent::new(NodeValues::U8(vec![12]), DataFormat::Decimal);
    scene
        .add_named_data_node(Point::new(10.0, 10.0), " B ", content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-named-padded", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some(" B : u8 = 12"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )
}
