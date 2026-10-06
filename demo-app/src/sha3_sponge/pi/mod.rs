//! `sha3_sponge::keccak`'s own "Pi" node: a nested `Scene` stepping through SHA3's real `Pi` step, one lane at a
//! time. `Pi` only moves lanes, it never changes a value: the lane at coordinates `(x, y)` is written to
//! `(y, (2x + 3y) mod 5)` of `Pi`'s own output. Each step shows that arithmetic for one lane of `Rho`'s own output,
//! then writes the lane to its new place.

use super::rho::grid_cell;
use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, SceneTitleOptions,
        SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Pi`'s own nested child `<svg>` — a fixed id, for the same reason as `rho::CHILD_SVG_ID`.
pub(super) const CHILD_SVG_ID: &str = "sha3-sponge-keccak-pi-child";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where `Pi` writes lane `lane` (flat `x + 5y`): to flat lane `x' + 5y'`, with `x' = y` and `y' = (2x + 3y) mod 5`.
/// Returns `(x, y, y_prime, destination)`. FIPS 202 section 3.2.3 states `Pi` as `A'[x, y] = A[(x + 3y) mod 5, x]`,
/// which is the same permutation read from the destination's side.
fn destination(lane: usize) -> (usize, usize, usize, usize) {
    let (x, y) = (lane % 5, lane / 5);
    let y_prime = (2 * x + 3 * y) % 5;
    (x, y, y_prime, y + 5 * y_prime)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Pi`: every lane moved to its own [`destination`].
pub(super) fn pi(input: [u64; 25]) -> [u64; 25] {
    let mut output = [0; 25];
    for (lane, value) in input.into_iter().enumerate() {
        output[destination(lane).3] = value;
    }
    output
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `svg_id` from scratch for step `n` (`0..25`, one per lane), `None` meaning unstarted.
///
/// Row 1 is "Rho Output Bytes", `input`'s own 25 lanes, cell `n` selected. Below it, the selected lane (zero while
/// unstarted) and a group of nodes form a pair, centred on one third and two thirds of row 1's own width. The group
/// is `x`, the `2x + 3y mod 5` calculation, and `y` side by side, with `new y` (the calculation's own result)
/// centred beneath the calculation and `new x` (the existing `y`) beneath `y`. The selected lane is vertically
/// centred on that whole group. The lane is then written to the matching cell of "Pi Output Bytes", whose lanes
/// `0..=n` have been written so far. Unlike `Rho`, `Pi` moves each lane to somewhere other than its own position,
/// so the highlighted output cell is not cell `n`.
///
/// Returns "Rho Output Bytes" alongside the `Scene`: the node a caller's own selection toolbar drives.
///
/// # Errors
///
/// Returns `Err` if `svg_id` is missing from the DOM, or if any library call fails.
fn build_scene(svg_id: &str, input: [u64; 25], n: Option<usize>) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    crate::util::frame_nested_scene(&document, svg_id)?;
    required_element(&document, svg_id)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Keccak Pi", SceneTitleOptions::default())
        .map_err(stringify)?;

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const V_GAP: f64 = 58.0;
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);
    let dec = |value: usize| DataNodeContent::new(NodeValues::U8(vec![value as u8]), DataFormat::Decimal);
    let hex25 = |values: [u64; 25]| {
        DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(5))
    };

    let input_node = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Rho Output Bytes", hex25(input))
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input_node).map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(input_node, grid_cell(n, 5)).map_err(stringify)?;
    }

    // Unstarted: the calculation still exists, over lane `0` — but the selected lane itself is shown as zero, since
    // nothing has been selected yet.
    let lane = n.unwrap_or(0);
    let (x, y, y_prime, target) = destination(lane);
    let selected_value = if n.is_some() { input[lane] } else { 0 };
    let top_y = input_rect.origin.y + input_rect.size.height + V_GAP;

    // A node's own size is only known once it exists, so each is added at `top_y` and then moved.
    let add = |label: &str, content: DataNodeContent| -> Result<(NodeId, Rect), String> {
        let id = scene
            .add_named_data_node(Point::new(LEFT_X, top_y), label, content)
            .map_err(stringify)?;
        let rect = scene.node_rect(id).map_err(stringify)?;
        Ok((id, rect))
    };
    let place = |id: NodeId, rect: Rect, x: f64, y: f64| -> Result<Rect, String> {
        scene.move_node(id, Point::new(x, y)).map_err(stringify)?;
        Ok(Rect {
            origin: Point::new(x, y),
            size: rect.size,
        })
    };

    let lane_label = n.map_or("Rho Output[-]".to_string(), |n| format!("Rho Output[{n}]"));
    let (lane_node, lane_rect) = add(&lane_label, hex(selected_value))?;
    let (x_node, x_rect) = add("x", dec(x))?;
    let (calc_node, calc_rect) = add("2x + 3y mod 5", dec(y_prime))?;
    let (y_node, y_rect) = add("y", dec(y))?;
    let (new_y_node, new_y_rect) = add("new y", dec(y_prime))?;
    let (new_x_node, new_x_rect) = add("new x", dec(y))?;

    // `x`, the calculation, and `y` form one group with a small gap between each. That group and the selected lane
    // are then a pair, centred on one third and two thirds of "Rho Output Bytes"'s own width.
    const GROUP_GAP: f64 = 20.0;
    // Between that group and the `new y`/`new x` pair below it.
    const NEW_ROW_GAP: f64 = 40.0;
    let group_width = x_rect.size.width + calc_rect.size.width + y_rect.size.width + 2.0 * GROUP_GAP;
    let group_left = input_rect.origin.x + input_rect.size.width * 2.0 / 3.0 - group_width / 2.0;
    let x_rect = place(x_node, x_rect, group_left, top_y)?;
    let calc_rect = place(calc_node, calc_rect, x_rect.origin.x + x_rect.size.width + GROUP_GAP, top_y)?;
    let y_rect = place(y_node, y_rect, calc_rect.origin.x + calc_rect.size.width + GROUP_GAP, top_y)?;

    // `new y` (the calculation's own result) is centred under the calculation; `new x` (the existing `y`) under `y`.
    let top_row_bottom = [x_rect, calc_rect, y_rect]
        .iter()
        .fold(top_y, |bottom, r| bottom.max(r.origin.y + r.size.height));
    let new_row_y = top_row_bottom + NEW_ROW_GAP;
    let centre = |r: Rect| r.origin.x + r.size.width / 2.0;
    place(
        new_y_node,
        new_y_rect,
        centre(calc_rect) - new_y_rect.size.width / 2.0,
        new_row_y,
    )?;
    place(new_x_node, new_x_rect, centre(y_rect) - new_x_rect.size.width / 2.0, new_row_y)?;
    let group_bottom = new_row_y + new_y_rect.size.height.max(new_x_rect.size.height);

    // The selected lane sits at one third of the width, vertically centred on the whole five-node group.
    place(
        lane_node,
        lane_rect,
        input_rect.origin.x + input_rect.size.width / 3.0 - lane_rect.size.width / 2.0,
        (top_y + group_bottom) / 2.0 - lane_rect.size.height / 2.0,
    )?;
    let lane_bottom = (top_y + group_bottom) / 2.0 + lane_rect.size.height / 2.0;
    let row_bottom = group_bottom.max(lane_bottom);

    // Lanes `0..=n` have been written so far; every other cell is still zero.
    let mut written = [0u64; 25];
    if let Some(n) = n {
        for k in 0..=n {
            written[destination(k).3] = input[k];
        }
    }
    let output = scene
        .add_named_data_node(Point::new(LEFT_X, row_bottom + V_GAP), "Pi Output Bytes", hex25(written))
        .map_err(stringify)?;
    if n.is_some() {
        scene.set_secondary_selection(output, &[target]).map_err(stringify)?;
    }

    let sides = |from, to| ConnectorOptions::default().with_from_side(Some(from)).with_to_side(Some(to));
    let down = || sides(Side::South, Side::North);
    for id in [lane_node, x_node, y_node] {
        scene.add_edge_with(input_node, id, down()).map_err(stringify)?;
    }
    scene
        .add_edge_with(x_node, calc_node, sides(Side::East, Side::West))
        .map_err(stringify)?;
    scene
        .add_edge_with(y_node, calc_node, sides(Side::West, Side::East))
        .map_err(stringify)?;
    scene.add_edge_with(calc_node, new_y_node, down()).map_err(stringify)?;
    scene.add_edge_with(y_node, new_x_node, down()).map_err(stringify)?;
    for id in [lane_node, new_x_node, new_y_node] {
        scene.add_edge_with(id, output, down()).map_err(stringify)?;
    }

    let mut right = 0.0_f64;
    for id in [input_node, output] {
        let rect = scene.node_rect(id).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
    }
    let output_rect = scene.node_rect(output).map_err(stringify)?;
    crate::util::fit_nested_size(&scene, svg_id, right, output_rect.origin.y + output_rect.size.height, true)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, input_node))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state this nested child's own selection toolbar carries across steps.
struct PiState {
    /// `Pi`'s own real input, never mutated.
    input: [u64; 25],
    /// The id of whichever `<svg>` currently backs the nested child — every step needs a fresh one, for the same
    /// reason as `theta::theta_c::rebuild_child`.
    child_svg_id: String,
}

thread_local! {
    // The round `Scene`, this nested `Pi` child, and the container `NodeId` that owns it — the same trio as
    // `theta::theta_d`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `Pi` child currently grafted in.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `Pi` child if it is currently focused, and reports whether it was.
pub(super) fn exit_if_focused() -> bool {
    SCENE.with_borrow(|slot| {
        let Some((_, child, _)) = slot else { return false };
        if !child.is_focused() {
            return false;
        }
        let _ = child.exit();
        true
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `child`, bound to `driver`, driving the next lane — the counterpart to
/// `theta::theta_d::attach_toolbar`.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_toolbar(child: &Scene, driver: NodeId, n: Option<usize>, state: Rc<RefCell<PiState>>) -> Result<(), String> {
    child
        .show_selection_toolbar(driver, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            let _ = rebuild_child(transition.to, state.clone());
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child.set_selection(driver, grid_cell(n, 5)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested `Pi` child for step `to` and grafts it into [`SCENE`]'s own `parent` in place of the one
/// currently shown — the same rebuild as `theta::theta_d::rebuild_child`.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the
/// DOM, or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<PiState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (input, previous_id) = {
        let state = state.borrow();
        (state.input, state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id("sha3-sponge-pi-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, driver) = build_scene(&next_id, input, to)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, driver, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the Pi scene was not initialised")?;
        old_child.exit().map_err(stringify)?;
        parent.replace_container_child(node, new_child.clone()).map_err(stringify)?;
        parent.enter(node).map_err(stringify)?;
        *slot = Some((parent, new_child, node));
        Ok(())
    })?;

    required_element(&document, &previous_id)?.remove();
    state.borrow_mut().child_svg_id = next_id;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds this nested `Pi` child, unstarted, against [`CHILD_SVG_ID`], and wires its own stepping toolbar. Its input
/// is `input`, the lanes `Rho` produced. Called once per round, from `keccak::build_scene`, right
/// before "Pi" is added as a container node.
///
/// Stepping removes the `<svg>` it started from, so [`CHILD_SVG_ID`] may be gone by the next round. This recreates
/// it if so, and removes any stepped clone a previous round left behind.
///
/// # Errors
///
/// Returns `Err` if the stage is missing from the DOM, or if any library call fails.
pub(super) fn build_initial_scene(input: [u64; 25]) -> Result<Scene, String> {
    let document = crate::util::document()?;
    let stale = document
        .query_selector_all("[id^=\"sha3-sponge-pi-child-\"]")
        .map_err(|e| format!("could not search for stale Pi children: {e:?}"))?;
    for i in 0..stale.length() {
        if let Some(element) = stale.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            element.remove();
        }
    }
    crate::util::ensure_svg(
        &document,
        "sha3-sponge-close",
        CHILD_SVG_ID,
        Some("nested-scene nested-scene-depth-2"),
        svg_dom::root::utils::Size::new(1000.0, 900.0),
    )?;

    let (child, driver) = build_scene(CHILD_SVG_ID, input, None)?;
    let state = Rc::new(RefCell::new(PiState {
        input,
        child_svg_id: CHILD_SVG_ID.to_string(),
    }));
    attach_toolbar(&child, driver, None, state)?;
    Ok(child)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
