//! `sha3_sponge::keccak`'s own "Rho" node: a nested `Scene` stepping through SHA3's real `Rho` step, one lane at a
//! time. Each step rotates one lane of `Rho`'s own input left by that lane's own entry in the rotation-offset table of
//! NIST FIPS 202. [`ROTATION_OFFSETS`] holds those offsets reduced modulo 64. The result is written into the matching
//! cell of `Rho`'s own output. Its input is the state `Theta` produced.

use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, SceneTitleOptions, Selection,
        Side, ToolbarOptions, UnaryOperator,
    },
};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Rho`'s own nested child `<svg>`: a fixed id, created once by `sha3_sponge::create_stage_svgs`. It is fixed for the
/// same reason `keccak::THETA_CHILD_SVG_ID` is. The round scene around it is rebuilt on every round step, but this one
/// element is only ever created once.
pub(super) const CHILD_SVG_ID: &str = "sha3-sponge-keccak-rho-child";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `Rho` rotation offsets for lanes `1..25`, **reduced modulo 64**, indexed by flat lane number `x + 5y`.
///
/// NIST FIPS 202, section 3.2.2 (Algorithm 2, and its Table 2) defines the offsets as the triangular numbers `(t + 1)(t
/// + 2) / 2` for `t = 0..24`, assigned to lanes by walking `(x, y) -> (y, (2x + 3y) mod 5)` from `(1, 0)`. Table 2
///   lists those values unreduced — 1, 3, 6, ..., 300 — since a rotation is only ever taken modulo the lane width. For
///   `Keccak-f\[1600\]` that width is `w = 64`, so each value here is the table's own value `mod 64`. For example, the
///   153 in Table 2 appears here as `25`, and the 300 as `44`. Rotating a `u64` left by either gives the same result,
///   but this is the amount `u64::rotate_left` actually needs.
///
/// Lane `0` always rotates by `0`, so it has no entry: this is the 24-value table `Rho` actually needs. The values are
/// stored by lane, not in the order the walk visits them. A unit test checks every entry against that derivation.
const ROTATION_OFFSETS: [u8; 24] = [
    1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14,
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `Selection` addressing flat cell `n` of a grid `cols` columns wide — `Selection::Row` with a `col`, the one form
/// [`DataNodeContent::natural_selection`] produces for a genuinely two-dimensional grid. A selection toolbar reads its
/// node's own `Selection` back through `flat_index`. That treats `Selection::Cell` on such a grid as "unstarted". So
/// the toolbar's driver must use this form, or its own next press restarts from cell `0`.
pub(super) fn grid_cell(n: usize, cols: usize) -> Selection {
    Selection::Row {
        row: n / cols,
        col: Some(n % cols),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rotation offset for flat lane `lane` (`0..25`) — `0` for lane `0`.
fn offset(lane: usize) -> u8 {
    if lane == 0 { 0 } else { ROTATION_OFFSETS[lane - 1] }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Rho`: every lane rotated left by its own [`offset`].
pub(super) fn rho(input: [u64; 25]) -> [u64; 25] {
    std::array::from_fn(|lane| input[lane].rotate_left(u32::from(offset(lane))))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state this nested child's own selection toolbar carries across steps.
struct RhoState {
    /// `Rho`'s own real input, never mutated.
    input: [u64; 25],
    /// The id of whichever `<svg>` currently backs the nested child — every step needs a fresh one, for the same reason
    /// as `theta::theta_c::rebuild_child`.
    child_svg_id: String,
}

thread_local! {
    // The round `Scene`, this nested `Rho` child, and the container `NodeId` that owns it — the same trio as
    // `theta::theta_d`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `Rho` child currently grafted in.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `Rho` child if it is currently focused, and reports whether it was.
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
/// Builds `svg_id` from scratch for step `n` (`0..25`, one per lane), `None` meaning unstarted.
///
/// Row 1 is "Theta Output Bytes", `input`'s own 25 lanes, cell `n` selected. Row 2 holds the selected lane on its own,
/// a `ROTL` operator rotating it, and the 24-value rotation table ([`ROTATION_OFFSETS`], reduced modulo 64, and
/// labelled as such). The table's own cell for lane `n` is marked, and the table feeds the operator its amount. Row 3
/// is "Rho Output Bytes", the same shape as row 1, with lanes `0..=n` filled in so far and the rest still zero.
///
/// The operator is `ROTL`, not a shift: a shift would discard bits, so the output could never be real `Rho`. Swap in
/// `UnaryOperator::ShiftRight` to see that.
///
/// Returns "Theta Output Bytes" alongside the `Scene`: the node a caller's own selection toolbar drives.
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
        .show_scene_title("Keccak Rho", SceneTitleOptions::default())
        .map_err(stringify)?;

    const LEFT_X: f64 = 20.0;
    // Below the scene's own title and its stepping toolbar, which is at the top.
    const TOP_Y: f64 = crate::util::CONTENT_TOP;
    const V_GAP: f64 = 58.0;
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);
    let hex25 = |values: [u64; 25]| {
        DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(5))
    };

    let input_node = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Theta Output Bytes", hex25(input))
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input_node).map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(input_node, grid_cell(n, 5)).map_err(stringify)?;
    }

    // Unstarted: the chain still exists, over lane `0`.
    let lane = n.unwrap_or(0);
    let amount = offset(lane);
    let rotated = input[lane].rotate_left(u32::from(amount));
    let row_2_y = input_rect.origin.y + input_rect.size.height + V_GAP;

    // Row 2's three nodes are centred on the quarter, half, and three-quarter points of "Theta Output Bytes"'s own
    // width. A node's own width is only known once it exists, so each is added at its own row's `y` and then moved.
    let centre_on = |id: NodeId, fraction: f64| -> Result<(), String> {
        let rect = scene.node_rect(id).map_err(stringify)?;
        let centre_x = input_rect.origin.x + fraction * input_rect.size.width;
        scene
            .move_node(id, Point::new(centre_x - rect.size.width / 2.0, rect.origin.y))
            .map_err(stringify)
    };

    let lane_label = n.map_or("Theta Output[-]".to_string(), |n| format!("Theta Output[{n}]"));
    let lane_node = scene
        .add_named_data_node(Point::new(LEFT_X, row_2_y), &lane_label, hex(input[lane]))
        .map_err(stringify)?;
    centre_on(lane_node, 0.25)?;

    let rotl_node = scene
        .add_unary_operator_node(
            Point::new(LEFT_X, row_2_y),
            UnaryOperator::RotateLeft(amount),
            lane_node,
            hex(rotated),
        )
        .map_err(stringify)?;
    centre_on(rotl_node, 0.5)?;
    // Unstarted, the lane and its rotation are placeholders: nothing has been selected to rotate yet.
    crate::util::mark_unreached(&scene, lane_node, &crate::util::all_if(n.is_none(), 1))?;
    crate::util::mark_unreached(&scene, rotl_node, &crate::util::all_if(n.is_none(), 1))?;

    let table_node = scene
        .add_named_data_node(
            Point::new(LEFT_X, row_2_y),
            "Rotation offsets (mod 64)",
            DataNodeContent::new(NodeValues::U8(ROTATION_OFFSETS.to_vec()), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(6)),
        )
        .map_err(stringify)?;
    centre_on(table_node, 0.75)?;
    if let Some(n) = n.filter(|n| *n > 0) {
        scene.set_secondary_selection(table_node, &[n - 1]).map_err(stringify)?;
    }
    let table_rect = scene.node_rect(table_node).map_err(stringify)?;

    // The lane and the operator are shorter than the table, so centre each on the table's own height.
    let table_mid_y = table_rect.origin.y + table_rect.size.height / 2.0;
    for id in [lane_node, rotl_node] {
        let rect = scene.node_rect(id).map_err(stringify)?;
        scene
            .move_node(id, Point::new(rect.origin.x, table_mid_y - rect.size.height / 2.0))
            .map_err(stringify)?;
    }

    let row_2_bottom = [lane_node, rotl_node]
        .into_iter()
        .map(|id| scene.node_rect(id).map(|r| r.origin.y + r.size.height))
        .collect::<Result<Vec<_>, _>>()
        .map_err(stringify)?
        .into_iter()
        .fold(table_rect.origin.y + table_rect.size.height, f64::max);

    // Lanes `0..=n` are written so far; everything after is still zero.
    let result = rho(input);
    let written: [u64; 25] = std::array::from_fn(|i| if n.is_some_and(|n| i <= n) { result[i] } else { 0 });
    let output = scene
        .add_named_data_node(Point::new(LEFT_X, row_2_bottom + V_GAP), "Rho Output Bytes", hex25(written))
        .map_err(stringify)?;
    if let Some(n) = n {
        scene.set_secondary_selection(output, &[n]).map_err(stringify)?;
    }
    // Lanes after `n` are not written yet.
    crate::util::mark_unreached(&scene, output, &crate::util::after(n, 25))?;

    let sides = |from, to| ConnectorOptions::default().with_from_side(Some(from)).with_to_side(Some(to));
    scene
        .add_edge_with(input_node, lane_node, sides(Side::South, Side::North))
        .map_err(stringify)?;
    scene
        .add_edge_with(lane_node, rotl_node, sides(Side::East, Side::West))
        .map_err(stringify)?;
    scene
        .add_edge_with(table_node, rotl_node, sides(Side::West, Side::East))
        .map_err(stringify)?;
    scene
        .add_edge_with(rotl_node, output, sides(Side::South, Side::North))
        .map_err(stringify)?;

    let right = [input_node, table_node, output]
        .into_iter()
        .map(|id| scene.node_rect(id).map(|r| r.origin.x + r.size.width))
        .collect::<Result<Vec<_>, _>>()
        .map_err(stringify)?
        .into_iter()
        .fold(0.0, f64::max);
    let output_rect = scene.node_rect(output).map_err(stringify)?;
    crate::util::fit_nested_size(&scene, svg_id, right, output_rect.origin.y + output_rect.size.height, false)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, input_node))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `child`, bound to `driver`, driving the next lane — the counterpart to
/// `theta::theta_d::attach_toolbar`.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_toolbar(child: &Scene, driver: NodeId, n: Option<usize>, state: Rc<RefCell<RhoState>>) -> Result<(), String> {
    child
        .show_selection_toolbar(driver, crate::util::step_toolbar_options(), move |_scene, _node, transition| {
            crate::sha3_sponge::report_step(rebuild_child(transition.to, state.clone()));
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child.set_selection(driver, grid_cell(n, 5)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested `Rho` child for step `to` and grafts it into [`SCENE`]'s own `parent` in place of the one
/// currently shown — the same rebuild as `theta::theta_d::rebuild_child`.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the DOM,
/// or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<RhoState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (input, previous_id) = {
        let state = state.borrow();
        (state.input, state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id("sha3-sponge-rho-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, driver) = build_scene(&next_id, input, to)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, driver, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the Rho scene was not initialised")?;
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
/// Builds this nested `Rho` child, unstarted, against [`CHILD_SVG_ID`], and wires its own stepping toolbar. Its input
/// is `input`, the lanes `Theta` produced. Called once per round, from `keccak::build_scene`, right before "Rho" is
/// added as a container node.
///
/// Stepping removes the `<svg>` it started from, so [`CHILD_SVG_ID`] may be gone by the next round. This recreates it
/// if so, and removes any stepped clone a previous round left behind.
///
/// # Errors
///
/// Returns `Err` if the stage is missing from the DOM, or if any library call fails.
pub(super) fn build_initial_scene(input: [u64; 25]) -> Result<Scene, String> {
    let document = crate::util::document()?;
    let stale = document
        .query_selector_all("[id^=\"sha3-sponge-rho-child-\"]")
        .map_err(|e| format!("could not search for stale Rho children: {e:?}"))?;
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

    let (child, driver) = build_scene(CHILD_SVG_ID, input, None)?;
    let state = Rc::new(RefCell::new(RhoState {
        input,
        child_svg_id: CHILD_SVG_ID.to_string(),
    }));
    attach_toolbar(&child, driver, None, state)?;
    Ok(child)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
