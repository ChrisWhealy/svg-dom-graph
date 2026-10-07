//! `sha3_sponge::keccak`'s own "Chi" node: a nested `Scene` stepping through SHA3's real `Chi` step, one lane at a
//! time. `Chi` is the only non-linear step: lane `(x, y)` becomes `A[x, y] ^ (!A[x + 1, y] & A[x + 2, y])`, with `x`
//! taken modulo 5. Each step shows that expression for one lane of `Pi`'s own output, then writes the result.

use super::rho::grid_cell;
use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect};
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene,
        SceneTitleOptions, SelectionToolbarOptions, Side, ToolbarOptions, UnaryOperator,
    },
};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Chi`'s own nested child `<svg>` — a fixed id, for the same reason as `rho::CHILD_SVG_ID`.
pub(super) const CHILD_SVG_ID: &str = "sha3-sponge-keccak-chi-child";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The flat lane numbers `Chi` reads for lane `lane` (`x + 5y`): `(x, y)`, `(x + 1 mod 5, y)` and `(x + 2 mod 5, y)`.
fn window(lane: usize) -> [usize; 3] {
    let (x, y) = (lane % 5, lane / 5);
    [x + 5 * y, (x + 1) % 5 + 5 * y, (x + 2) % 5 + 5 * y]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Chi` for one lane: `w0 ^ (!w1 & w2)`.
fn chi_lane(w0: u64, w1: u64, w2: u64) -> u64 {
    w0 ^ (!w1 & w2)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Chi`, over a whole 25-lane state.
pub(super) fn chi(input: [u64; 25]) -> [u64; 25] {
    std::array::from_fn(|lane| {
        let [i0, i1, i2] = window(lane);
        chi_lane(input[i0], input[i1], input[i2])
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `svg_id` from scratch for step `n` (`0..25`, one per lane), `None` meaning unstarted.
///
/// Rows, top to bottom:
/// 1. "Pi Output Bytes", `input`'s own 25 lanes, cell `n` selected.
/// 2. `W0`, `W1` and `W2`: the lanes at `(x, y)`, `(x + 1 mod 5, y)` and `(x + 2 mod 5, y)`, centred on a quarter, a
///    half and three quarters of row 1's own width.
/// 3. A `NOT` operator, taking `W1` and centred beneath it.
/// 4. The `NOT` result as a data node, centred beneath `W1` too, and an `AND` operator to its right, centred beneath
///    `W2` and taking the `NOT` result and `W2`. Both are vertically centred on the same line.
/// 5. An `XOR` operator, taking `W0` and the `AND` result, which sits in its own data node beneath the `AND` operator.
///    The `XOR` operator is centred beneath `W0`.
/// 6. "Chi Output Bytes", whose lanes `0..=n` have been written so far.
///
/// While unstarted, every lane shown is zero.
///
/// Returns "Pi Output Bytes" alongside the `Scene`: the node a caller's own selection toolbar drives.
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
        .show_scene_title("Keccak Chi", SceneTitleOptions::default())
        .map_err(stringify)?;

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const V_GAP: f64 = 40.0;
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);
    let hex25 = |values: [u64; 25]| {
        DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(5))
    };

    let input_node = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Pi Output Bytes", hex25(input))
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input_node).map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(input_node, grid_cell(n, 5)).map_err(stringify)?;
    }
    // `W1` and `W2` come from two other cells of the same row — secondary, since the toolbar only drives `W0`'s.
    if n.is_some() {
        let [_, i1, i2] = window(n.unwrap_or(0));
        scene.set_secondary_selection(input_node, &[i1, i2]).map_err(stringify)?;
    }

    // Unstarted: the whole calculation still exists, over zeros.
    let lane = n.unwrap_or(0);
    let [i0, i1, i2] = window(lane);
    let (w0, w1, w2) = if n.is_some() { (input[i0], input[i1], input[i2]) } else { (0, 0, 0) };
    let not_w1 = !w1;
    let and = not_w1 & w2;
    let result = w0 ^ and;

    // A node's own size is only known once it exists, so each is added at the top of its row and then moved: the `x`
    // for the node's own centre is a fraction of "Pi Output Bytes"'s own width, `y` its own top edge.
    let place = |id: NodeId, fraction: f64, y: f64| -> Result<Rect, String> {
        let size = scene.node_rect(id).map_err(stringify)?.size;
        let origin = Point::new(input_rect.origin.x + fraction * input_rect.size.width - size.width / 2.0, y);
        scene.move_node(id, origin).map_err(stringify)?;
        Ok(Rect { origin, size })
    };
    let bottom = |r: Rect| r.origin.y + r.size.height;
    let row_2_y = bottom(input_rect) + V_GAP;

    let add_data = |label: &str, value: u64| -> Result<NodeId, String> {
        scene
            .add_named_data_node(Point::new(LEFT_X, row_2_y), label, hex(value))
            .map_err(stringify)
    };
    let w0_node = add_data("W0 (x,y)", w0)?;
    let w1_node = add_data("W1 (x+1 % 5, y)", w1)?;
    let w2_node = add_data("W2 (x+2 % 5, y)", w2)?;
    let row_2 = [
        place(w0_node, 0.25, row_2_y)?,
        place(w1_node, 0.5, row_2_y)?,
        place(w2_node, 0.75, row_2_y)?,
    ];
    let row_2_bottom = row_2.iter().fold(row_2_y, |b, r| b.max(bottom(*r)));

    let not_node = scene
        .add_unary_operator_node(
            Point::new(LEFT_X, row_2_bottom + V_GAP),
            UnaryOperator::Not,
            w1_node,
            hex(not_w1),
        )
        .map_err(stringify)?;
    let not_rect = place(not_node, 0.5, row_2_bottom + V_GAP)?;

    let row_4_y = bottom(not_rect) + V_GAP;
    let not_out = scene
        .add_named_data_node(Point::new(LEFT_X, row_4_y), "NOT W1", hex(not_w1))
        .map_err(stringify)?;
    let and_node = scene
        .add_binary_operator_node(Point::new(LEFT_X, row_4_y), BinaryOperator::And, (not_out, w2_node), hex(and))
        .map_err(stringify)?;
    // Vertically centred on one line, whichever of the two is taller.
    let row_4_height = [not_out, and_node]
        .into_iter()
        .map(|id| scene.node_rect(id).map(|r| r.size.height))
        .collect::<Result<Vec<_>, _>>()
        .map_err(stringify)?
        .into_iter()
        .fold(0.0, f64::max);
    let row_4_mid = row_4_y + row_4_height / 2.0;
    for (id, fraction) in [(not_out, 0.5), (and_node, 0.75)] {
        let height = scene.node_rect(id).map_err(stringify)?.size.height;
        place(id, fraction, row_4_mid - height / 2.0)?;
    }

    let row_5_y = row_4_y + row_4_height + V_GAP;
    let and_out = scene
        .add_named_data_node(Point::new(LEFT_X, row_5_y), "NOT W1 AND W2", hex(and))
        .map_err(stringify)?;
    let xor_node = scene
        .add_binary_operator_node(
            Point::new(LEFT_X, row_5_y),
            BinaryOperator::Xor,
            (w0_node, and_out),
            hex(result),
        )
        .map_err(stringify)?;
    let and_out_rect = place(and_out, 0.75, row_5_y)?;
    let xor_rect = place(xor_node, 0.25, row_5_y)?;
    let row_5_bottom = bottom(and_out_rect).max(bottom(xor_rect));

    let mut written = [0u64; 25];
    if let Some(n) = n {
        let all = chi(input);
        written[..=n].copy_from_slice(&all[..=n]);
    }
    let output = scene
        .add_named_data_node(Point::new(LEFT_X, row_5_bottom + V_GAP), "Chi Output Bytes", hex25(written))
        .map_err(stringify)?;
    if let Some(n) = n {
        scene.set_secondary_selection(output, &[n]).map_err(stringify)?;
    }

    let sides = |from, to| ConnectorOptions::default().with_from_side(Some(from)).with_to_side(Some(to));
    let down = || sides(Side::South, Side::North);
    for id in [w0_node, w1_node, w2_node] {
        scene.add_edge_with(input_node, id, down()).map_err(stringify)?;
    }
    scene.add_edge_with(not_node, not_out, down()).map_err(stringify)?;
    scene.add_edge_with(and_node, and_out, down()).map_err(stringify)?;
    scene.add_edge_with(xor_node, output, down()).map_err(stringify)?;

    let output_rect = scene.node_rect(output).map_err(stringify)?;
    let right = (input_rect.origin.x + input_rect.size.width).max(output_rect.origin.x + output_rect.size.width);
    crate::util::fit_nested_size(&scene, svg_id, right, bottom(output_rect), true)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, input_node))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state this nested child's own selection toolbar carries across steps.
struct ChiState {
    /// `Chi`'s own real input, never mutated.
    input: [u64; 25],
    /// The id of whichever `<svg>` currently backs the nested child — every step needs a fresh one, for the same reason
    /// as `theta::theta_c::rebuild_child`.
    child_svg_id: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    // The round `Scene`, this nested `Chi` child, and the container `NodeId` that owns it — the same trio as
    // `theta::theta_d`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `Chi` child currently grafted in.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `Chi` child if it is currently focused, and reports whether it was.
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
fn attach_toolbar(child: &Scene, driver: NodeId, n: Option<usize>, state: Rc<RefCell<ChiState>>) -> Result<(), String> {
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
/// Rebuilds the nested `Chi` child for step `to` and grafts it into [`SCENE`]'s own `parent` in place of the one
/// currently shown — the same rebuild as `theta::theta_d::rebuild_child`.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the DOM,
/// or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<ChiState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (input, previous_id) = {
        let state = state.borrow();
        (state.input, state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id("sha3-sponge-chi-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, driver) = build_scene(&next_id, input, to)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, driver, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the Chi scene was not initialised")?;
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
/// Builds this nested `Chi` child, unstarted, against [`CHILD_SVG_ID`], and wires its own stepping toolbar. Its input
/// is `input`, the lanes `Pi` produced. Called once per round, from `keccak::build_scene`, right before "Chi" is added
/// as a container node.
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
        .query_selector_all("[id^=\"sha3-sponge-chi-child-\"]")
        .map_err(|e| format!("could not search for stale Chi children: {e:?}"))?;
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
        svg_dom::root::utils::Size::new(1000.0, 1000.0),
    )?;

    let (child, driver) = build_scene(CHILD_SVG_ID, input, None)?;
    let state = Rc::new(RefCell::new(ChiState {
        input,
        child_svg_id: CHILD_SVG_ID.to_string(),
    }));
    attach_toolbar(&child, driver, None, state)?;
    Ok(child)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
