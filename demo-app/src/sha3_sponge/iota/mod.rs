//! `sha3_sponge::keccak`'s own "Iota" node: a nested `Scene` showing SHA3's real `Iota` step. `Iota` changes exactly
//! one lane: the round constant of the current round is XORed into lane `(0, 0)`. Every other lane is copied to the
//! output as it is, so there is a single step to take — lane `0` — not one per lane.

use super::{keccak_f::ROUND_CONSTANTS, rho::grid_cell};
use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect};
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use wasm_bindgen::JsCast;

/// `Iota`'s own nested child `<svg>` — a fixed id, for the same reason as `rho::CHILD_SVG_ID`.
pub(super) const CHILD_SVG_ID: &str = "sha3-sponge-keccak-iota-child";

/// The constant `Iota` XORs into lane `lane` during round `round`: the round's own constant for lane `0`, and `0` for
/// every other lane.
fn constant_for(lane: usize, round: usize) -> u64 {
    if lane == 0 { ROUND_CONSTANTS[round] } else { 0 }
}

/// SHA3's real `Iota` for round `round`.
pub(super) fn iota(input: [u64; 25], round: usize) -> [u64; 25] {
    std::array::from_fn(|lane| input[lane] ^ constant_for(lane, round))
}

/// Builds `svg_id` from scratch for round `round`; `n` is `Some(0)` once the one step has been taken, `None` while
/// unstarted.
///
/// Rows, top to bottom:
/// 1. "Chi Output Bytes", `input`'s own 25 lanes. Once started, lane `0` — the only one `Iota` reads — is selected.
/// 2. Lane `0` and the round constant, a pair centred on one third and two thirds of row 1's own width.
/// 3. An `XOR` operator taking both, centred beneath the pair.
/// 4. "Iota Output Bytes". Taking the step writes all 25 lanes: lane `0` as the `XOR` result, marked as secondary, and
///    every other lane copied as it is.
///
/// While unstarted, every value shown below row 1 is zero. The selection toolbar drives lane `0`'s own node, which has
/// exactly one cell, so it offers one step.
///
/// Returns that lane `0` node alongside the `Scene`: the node a caller's own selection toolbar drives.
///
/// # Errors
///
/// Returns `Err` if `svg_id` is missing from the DOM, or if any library call fails.
fn build_scene(svg_id: &str, input: [u64; 25], round: usize, n: Option<usize>) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    crate::util::frame_nested_scene(&document, svg_id)?;
    required_element(&document, svg_id)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title(format!("Keccak Iota Round {round}"), SceneTitleOptions::default())
        .map_err(stringify)?;

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const V_GAP: f64 = 40.0;
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);
    let hex25 = |values: [u64; 25]| {
        DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(5))
    };

    let input_node = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Chi Output Bytes", hex25(input))
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input_node).map_err(stringify)?;
    let started = n.is_some();
    if started {
        scene.set_selection(input_node, grid_cell(0, 5)).map_err(stringify)?;
    }

    // Unstarted: the calculation still exists, over zeros.
    let (lane_value, constant) = if started { (input[0], constant_for(0, round)) } else { (0, 0) };
    let result = lane_value ^ constant;

    // A node's own size is only known once it exists, so each is added at the top of its row and then moved.
    let place = |id: NodeId, fraction: f64, y: f64| -> Result<Rect, String> {
        let size = scene.node_rect(id).map_err(stringify)?.size;
        let origin = Point::new(input_rect.origin.x + fraction * input_rect.size.width - size.width / 2.0, y);
        scene.move_node(id, origin).map_err(stringify)?;
        Ok(Rect { origin, size })
    };
    let bottom = |r: Rect| r.origin.y + r.size.height;
    let row_2_y = bottom(input_rect) + V_GAP;

    let lane_node = scene
        .add_named_data_node(Point::new(LEFT_X, row_2_y), "Chi Output[0]", hex(lane_value))
        .map_err(stringify)?;
    let constant_label = format!("RC[{round}]");
    let constant_node = scene
        .add_named_data_node(Point::new(LEFT_X, row_2_y), &constant_label, hex(constant))
        .map_err(stringify)?;
    let row_2 = [place(lane_node, 1.0 / 3.0, row_2_y)?, place(constant_node, 2.0 / 3.0, row_2_y)?];
    let row_2_bottom = row_2.iter().fold(row_2_y, |b, r| b.max(bottom(*r)));

    let row_3_y = row_2_bottom + V_GAP;
    let xor_node = scene
        .add_binary_operator_node(
            Point::new(LEFT_X, row_3_y),
            BinaryOperator::Xor,
            (lane_node, constant_node),
            hex(result),
        )
        .map_err(stringify)?;
    let xor_rect = place(xor_node, 0.5, row_3_y)?;

    // Taking the step writes every lane: lane `0` XORed with the constant, and the other 24 copied as they are.
    let written = if started { iota(input, round) } else { [0u64; 25] };
    let output = scene
        .add_named_data_node(
            Point::new(LEFT_X, bottom(xor_rect) + V_GAP),
            "Iota Output Bytes",
            hex25(written),
        )
        .map_err(stringify)?;
    if started {
        scene.set_secondary_selection(output, &[0]).map_err(stringify)?;
    }

    let sides = |from, to| ConnectorOptions::default().with_from_side(Some(from)).with_to_side(Some(to));
    let down = || sides(Side::South, Side::North);
    scene.add_edge_with(input_node, lane_node, down()).map_err(stringify)?;
    scene.add_edge_with(xor_node, output, down()).map_err(stringify)?;

    let output_rect = scene.node_rect(output).map_err(stringify)?;
    let right = (input_rect.origin.x + input_rect.size.width).max(output_rect.origin.x + output_rect.size.width);
    crate::util::fit_nested_size(&scene, svg_id, right, bottom(output_rect), true)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, lane_node))
}

/// Live state this nested child's own selection toolbar carries across steps.
struct IotaState {
    /// `Iota`'s own real input, never mutated.
    input: [u64; 25],
    /// The round whose constant this `Iota` XORs into lane `0`.
    round: usize,
    /// The id of whichever `<svg>` currently backs the nested child — every step needs a fresh one, for the same reason
    /// as `theta::theta_c::rebuild_child`.
    child_svg_id: String,
}

thread_local! {
    // The round `Scene`, this nested `Iota` child, and the container `NodeId` that owns it — the same trio as
    // `theta::theta_d`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `Iota` child currently grafted in.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `Iota` child if it is currently focused, and reports whether it was.
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
/// Shows a selection toolbar on `child`, bound to `driver`, lane `0`'s own one-cell node, so it offers the single step
/// — the counterpart to `theta::theta_d::attach_toolbar`.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_toolbar(
    child: &Scene,
    driver: NodeId,
    n: Option<usize>,
    state: Rc<RefCell<IotaState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(driver, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            let _ = rebuild_child(transition.to, state.clone());
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child.set_selection(driver, Selection::Cell(n)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested `Iota` child for step `to` and grafts it into [`SCENE`]'s own `parent` in place of the one
/// currently shown — the same rebuild as `theta::theta_d::rebuild_child`.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the DOM,
/// or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<IotaState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (input, round, previous_id) = {
        let state = state.borrow();
        (state.input, state.round, state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id("sha3-sponge-iota-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, driver) = build_scene(&next_id, input, round, to)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, driver, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the Iota scene was not initialised")?;
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
/// Builds this nested `Iota` child for round `round`, unstarted, against [`CHILD_SVG_ID`], and wires its own stepping
/// toolbar. Its input is `input`, the lanes `Chi` produced. Called once per round, from `keccak::build_scene`, right
/// before "Iota" is added as a container node.
///
/// Stepping removes the `<svg>` it started from, so [`CHILD_SVG_ID`] may be gone by the next round. This recreates it
/// if so, and removes any stepped clone a previous round left behind.
///
/// # Errors
///
/// Returns `Err` if the stage is missing from the DOM, or if any library call fails.
pub(super) fn build_initial_scene(round: usize, input: [u64; 25]) -> Result<Scene, String> {
    let document = crate::util::document()?;
    let stale = document
        .query_selector_all("[id^=\"sha3-sponge-iota-child-\"]")
        .map_err(|e| format!("could not search for stale Iota children: {e:?}"))?;
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
        svg_dom::root::utils::Size::new(1000.0, 800.0),
    )?;

    let (child, driver) = build_scene(CHILD_SVG_ID, input, round, None)?;
    let state = Rc::new(RefCell::new(IotaState {
        input,
        round,
        child_svg_id: CHILD_SVG_ID.to_string(),
    }));
    attach_toolbar(&child, driver, None, state)?;
    Ok(child)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
