//! `sha3_sponge`'s own "Keccak f(1600)" node: a nested `Scene` stepping through all 24 real Keccak-f\[1600\] rounds.
//!
//! Each round shows the same five named sub-functions — `Theta`, `Rho`, `Pi`, `Chi`, `Iota` — arranged as one pipeline,
//! plus the real round constant `Iota` consumes. Each is a genuine nested `Scene`, and each is handed the state it
//! consumes. [`keccak_f::round_traces`](super::keccak_f::round_traces) runs the real permutation over the sponge's own
//! input state and keeps the state after every function. So every number in a child scene is derived from the one
//! immediately upstream of it. "A Bytes Output" is the real result of the round.

use super::keccak_f::{ROUND_CONSTANTS, ROUND_COUNT, round_traces, to_theta_grid};
use crate::{
    sha3_sponge::{chi, iota, pi, rho, theta},
    util::{create_child_svg, next_child_svg_id, required_element, stringify},
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, SceneTitleOptions, Selection,
        SelectionStride, Side, ToolbarOptions,
    },
};

/// How many named functions each round runs: `Theta`, `Rho`, `Pi`, `Chi`, `Iota`.
const FUNCTIONS_PER_ROUND: usize = 5;

/// `Theta`'s own nested child `<svg>`, inside this nested Keccak-f view. It has a fixed id, created once by
/// `sha3_sponge::create_stage_svgs`. The id is not derived from this scene's own `svg_id`, as [`theta::build_scene`]'s
/// own doc comment ("Reuse across more than one host") otherwise recommends. This scene's own `svg_id` changes on every
/// step (a fresh cloned sibling, via [`rebuild_child`]). Only the one `<svg>` for `Theta`'s own child is ever created,
/// though. [`theta::build_scene`] clears and redraws that same element fresh on every call. That keeps this correct
/// regardless of which round is currently showing.
const THETA_CHILD_SVG_ID: &str = "sha3-sponge-keccak-theta-child";

/// Live state this nested walk's own selection toolbar carries across rounds.
struct KeccakState {
    /// Round `0`'s own real input is the sponge's own combined `Capacity`/`Rate` state, handed down from
    /// `sha3_sponge::build_scene`. It is never itself mutated. Every round's own real input and output is derived
    /// afresh from it via [`round_traces`] on every step.
    seed: [u64; 25],
    /// `sha3_sponge::build_scene`'s own fixed prefix for this nested child's `<svg>` id. It is passed to
    /// `next_child_svg_id` on every step, never `child_svg_id` itself. So ids stay `prefix-0`, `prefix-1`, ... rather
    /// than growing a fresh suffix onto the previous one every step.
    base_svg_id: String,
    /// The id of whichever `<svg>` currently backs this nested child — see `theta::theta_c::rebuild_child`'s own doc
    /// comment for why every step needs a fresh one.
    child_svg_id: String,
}

thread_local! {
    // The top-level SHA3 Sponge `Scene`, this nested Keccak-f child `Scene`, and the container `NodeId` that owns it —
    // the same trio, for the same reasons, as `theta::theta_c`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
    // This round's own "Theta" node: the `Scene` it wraps. It is the middle rung between this module's own `SCENE`
    // (whether "Keccak f(1600)" itself is focused) and `theta::exit_if_focused` (whether one of `Theta`'s own
    // further-nested `ThetaC`/`ThetaD`/`XOR loop` is). Without this, [`exit_if_focused`] could not notice "Theta itself
    // is focused" and pop back to this round's own view. It could only notice "one of `ThetaC`'s own children is
    // focused" or "nothing nested at all is focused". Replaced fresh every round, in [`build_scene`], right alongside
    // `Theta`'s own container node.
    static THETA_CHILD: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested Keccak-f child currently grafted in — the one [`exit_if_focused`] and
/// [`rebuild_child`] act on from then on. Called once, from `sha3_sponge::build_scene`, right after `child` is grafted
/// into `parent` under `node`.
pub(crate) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits whichever of this nested view's own children is currently focused, deepest first: `Theta`'s own further nested
/// `ThetaC`/`ThetaD`/`XOR loop` (via [`theta::exit_if_focused`]), then `Theta` itself (via [`THETA_CHILD`]), then this
/// Keccak-f child itself. Reports whether anything was exited — what `sha3_sponge`'s own close button needs to know
/// before giving up.
pub(crate) fn exit_if_focused() -> bool {
    if theta::exit_if_focused()
        || rho::exit_if_focused()
        || pi::exit_if_focused()
        || chi::exit_if_focused()
        || iota::exit_if_focused()
    {
        return true;
    }
    let theta_itself_exited = THETA_CHILD.with_borrow(|slot| {
        let Some(child) = slot else { return false };
        if !child.is_focused() {
            return false;
        }
        let _ = child.exit();
        true
    });
    if theta_itself_exited {
        return true;
    }
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
/// Builds `svg_id` from scratch for walk position `position` (`0..120`). There are five functions in each of 24 rounds,
/// so the round is `position / 5`. The function in focus is the `position % 5`th of `Theta`, `Rho`, `Pi`, `Chi`,
/// `Iota`.
///
/// # What this draws
///
/// Row 1 holds "A Bytes - round `{round}`" — the real 25-lane state this round starts from, [`round_traces`]'s own
/// `round`th input, derived from `seed`. Beside it, "Round Constants" holds all 24 real [`ROUND_CONSTANTS`] as a
/// 4-column, 6-row grid — a single column reads just as well but takes far more vertical space. Neither is especially
/// tall, so both fit row 1 together, keeping this round's own overall height down further still, rather than stacking
/// "Round Constants" under the function row below instead. `Selection::Cell` addresses flat index `round` regardless of
/// the grid's own shape, so the cell highlighted stays correct either way.
///
/// Below row 1, five named boxes run left to right, not stacked top to bottom: `Theta`, `Rho`, `Pi`, `Chi`, `Iota`. A
/// horizontal row keeps this round's own overall height close to row 1's own, instead of adding five more row-heights
/// on top. A nested Scene's own viewBox, however tall, still only ever displays within its own parent's fixed-size
/// frame. All five are genuine container nodes, each nesting a scene that works through its own real function over the
/// state the one before it produced. `Theta` nests [`theta::build_scene`] — the scenes `panel-theta` draws standalone,
/// run here over this round's own input — and drills one level further, into `ThetaC`, `ThetaD` and `XOR loop`. Click
/// the function in focus to drill in, and the &times; in its own frame's corner to come back. Only the function in
/// focus is clickable. A connector from "Round Constants" into `Iota`'s own east side marks the one value `Iota`
/// consumes each round.
///
/// The bottom row is "A Bytes Output" — [`round_traces`]'s own `round`th output: the real result of this round, and the
/// next round's own "A Bytes" input. It stays all zeros until the walk reaches `Iota`, the round's own last function,
/// since the round has produced no result before then.
///
/// # Stepping through it
///
/// "Step" is a small, otherwise-meaningless array of one cell per function of every round. It is placed far off-canvas,
/// purely to drive a [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar. This
/// is the same trick `sha3_sponge::build_scene`'s own "Step" uses, for the same reason: no node already in this diagram
/// happens to have that many cells. `Prev`/`Next` move one function, rolling on into the next round after `Iota`, and
/// `Prev Round`/`Next Round` move five at once. This walk is never "not started": it opens on round `0`'s `Theta`.
/// [`rebuild_child`] wires the bar itself.
///
/// # Errors
///
/// Returns `Err` if `svg_id` is missing from the DOM, or if any library call fails.
fn build_scene(svg_id: &str, seed: [u64; 25], position: usize) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    // `position` counts every function of every round in order: five per round, so round `r` is `position / 5`, and the
    // function in focus is the `position % 5`th of `Theta`/`Rho`/`Pi`/`Chi`/`Iota`.
    let r = position / FUNCTIONS_PER_ROUND;
    let focused_function = position % FUNCTIONS_PER_ROUND;
    scene
        .show_scene_title(format!("Keccak f(1600) Round {r}"), SceneTitleOptions::default())
        .map_err(stringify)?;

    // Every number below is derived from the one upstream of it. This round's own trace holds the state at the start of
    // the round and after each of its five functions. Each nested scene is handed the one it consumes.
    let trace = round_traces(seed)[r];
    let (a_in, a_out) = (trace.input, trace.output);
    let hex25 = |values: [u64; 25]| DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal);

    const LEFT_X: f64 = 20.0;
    // Below the scene's own title and its stepping toolbar, which is at the top.
    const TOP_Y: f64 = crate::util::CONTENT_TOP;
    const V_GAP: f64 = 50.0;
    const H_GAP: f64 = 80.0;
    let fn_size = Size::new(130.0, 60.0);
    let fn_gap = 30.0;

    let a_in_label = format!("A Bytes - round {r}");
    let a_top = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), &a_in_label, hex25(a_in))
        .map_err(stringify)?;
    let a_top_rect = scene.node_rect(a_top).map_err(stringify)?;

    // Theta/Rho/Pi/Chi/Iota run left to right, not stacked top to bottom. A horizontal row keeps this round's own
    // overall height close to its row 1's own height, rather than adding five more row-heights on top. That matters
    // because a nested Scene's own viewBox, however tall, still only ever displays within its own parent's fixed-size
    // frame.
    let row_stride = fn_size.width + fn_gap;
    let row_width = 5.0 * fn_size.width + 4.0 * fn_gap;
    let row_x = a_top_rect.origin.x + (a_top_rect.size.width - row_width) / 2.0;
    // Directly below "A Bytes - round {round}". "Round Constants" sits beside it, rather than above the function row,
    // so it does not push the row down.
    let row_y = a_top_rect.origin.y + a_top_rect.size.height + V_GAP;

    // Fixed, not derived from `svg_id`. `svg_id` itself changes on every step (a fresh sibling `<svg>`, exactly like
    // `theta::theta_c`'s own per-step clones). Only one `<svg>` is ever created for Theta's own child, though. See this
    // constant's own doc comment. `false`: this round's own `svg_id` is not the shallowest level in
    // `#sha3-sponge-diagram`'s own `.nested-scene-stage`. See `theta::build_scene`'s own doc comment ("Reuse across
    // more than one host") for why a backdrop here would wrongly cover whatever shallower sibling sits behind it.
    let theta_child = theta::build_scene(THETA_CHILD_SVG_ID, false, to_theta_grid(trace.input))?;
    THETA_CHILD.with_borrow_mut(|slot| *slot = Some(theta_child.clone()));
    let theta = scene
        .add_container_node(Point::new(row_x, row_y), fn_size, "Theta", theta_child)
        .map_err(stringify)?;

    let rho_child = rho::build_initial_scene(trace.theta)?;
    let rho = scene
        .add_container_node(Point::new(row_x + row_stride, row_y), fn_size, "Rho", rho_child.clone())
        .map_err(stringify)?;
    rho::init_scene(scene.clone(), rho_child, rho);
    let pi_child = pi::build_initial_scene(trace.rho)?;
    let pi = scene
        .add_container_node(Point::new(row_x + 2.0 * row_stride, row_y), fn_size, "Pi", pi_child.clone())
        .map_err(stringify)?;
    pi::init_scene(scene.clone(), pi_child, pi);
    let chi_child = chi::build_initial_scene(trace.pi)?;
    let chi = scene
        .add_container_node(Point::new(row_x + 3.0 * row_stride, row_y), fn_size, "Chi", chi_child.clone())
        .map_err(stringify)?;
    chi::init_scene(scene.clone(), chi_child, chi);
    let iota_child = iota::build_initial_scene(r, trace.chi)?;
    let iota = scene
        .add_container_node(Point::new(row_x + 4.0 * row_stride, row_y), fn_size, "Iota", iota_child.clone())
        .map_err(stringify)?;
    iota::init_scene(scene.clone(), iota_child, iota);
    let iota_rect = scene.node_rect(iota).map_err(stringify)?;

    let a_out_y = iota_rect.origin.y + iota_rect.size.height + V_GAP;
    // Until the walk reaches `Iota`, the round's own last function, the round has no result yet. The output stays
    // initialised to zeros, the same "not yet written" look the sponge scene's own "XOR" uses.
    let shown_output = if focused_function == FUNCTIONS_PER_ROUND - 1 { a_out } else { [0; 25] };
    let a_out = scene
        .add_named_data_node(Point::new(LEFT_X, a_out_y), "A Bytes Output", hex25(shown_output))
        .map_err(stringify)?;
    // Until the walk reaches `Iota` the output holds placeholders. Round `r`'s own input and its function row are real.
    crate::util::mark_unreached(
        &scene,
        a_out,
        &crate::util::all_if(focused_function != FUNCTIONS_PER_ROUND - 1, 25),
    )?;

    // "Round Constants" sits to the right of "A Bytes - round {round}" and the function row, not below them. Its
    // midpoint is level with `Iota`'s, which keeps the round's own overall height down.
    let rc_x = a_top_rect.origin.x + a_top_rect.size.width + H_GAP;
    let rc = scene
        .add_named_data_node(
            Point::new(rc_x, TOP_Y),
            "Round Constants",
            DataNodeContent::new(NodeValues::U64(ROUND_CONSTANTS.to_vec()), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Columns(2)),
        )
        .map_err(stringify)?;
    scene.set_selection(rc, Selection::Cell(r)).map_err(stringify)?;
    // The node's own height is only known once it exists, so centre its midpoint on `Iota`'s own afterward.
    let rc_rect = scene.node_rect(rc).map_err(stringify)?;
    let iota_mid_y = iota_rect.origin.y + iota_rect.size.height / 2.0;
    scene
        .move_node(rc, Point::new(rc_x, iota_mid_y - rc_rect.size.height / 2.0))
        .map_err(stringify)?;
    let rc_rect = scene.node_rect(rc).map_err(stringify)?;

    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    let horizontal = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::East))
            .with_to_side(Some(Side::West))
    };
    scene.add_edge_with(a_top, theta, vertical()).map_err(stringify)?;
    scene.add_edge_with(theta, rho, horizontal()).map_err(stringify)?;
    scene.add_edge_with(rho, pi, horizontal()).map_err(stringify)?;
    scene.add_edge_with(pi, chi, horizontal()).map_err(stringify)?;
    scene.add_edge_with(chi, iota, horizontal()).map_err(stringify)?;
    scene.add_edge_with(iota, a_out, vertical()).map_err(stringify)?;
    scene
        .add_edge_with(
            rc,
            iota,
            ConnectorOptions::default()
                .with_from_side(Some(Side::West))
                .with_to_side(Some(Side::East)),
        )
        .map_err(stringify)?;

    // Only the function the walk is currently on is clickable, and rung. Entering one before the walk reaches it would
    // show a view whose own input is not yet that function's real input. Every step rebuilds this whole `Scene`, so
    // nothing needs undoing when the walk moves on, or back.
    let current = [theta, rho, pi, chi, iota][focused_function];
    scene.make_enterable(current).map_err(stringify)?;
    scene.set_focus(current, true).map_err(stringify)?;

    // "Round": see this function's own doc comment, "Stepping through it", for why this exists and why it sits far
    // off-canvas rather than anywhere a reader would actually see it.
    let step_values: Vec<u8> = (1..=(ROUND_COUNT * FUNCTIONS_PER_ROUND) as u8).collect();
    let step_driver = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U8(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    // No backdrop of `svg_id` here, unlike `theta::build_scene`'s own call for itself. `svg_id` is not the shallowest
    // level in `#sha3-sponge-diagram`'s own `.nested-scene-stage` either. See that function's own doc comment ("Reuse
    // across more than one host") for why one here would wrongly cover `#sha3-sponge-diagram` itself. Entering `Theta`
    // from here shows `#sha3-sponge-diagram`'s own backdrop in the margin instead, not this round's own content — a
    // smaller visual polish this feature does without.

    // Shrink this round's own `<svg>` to its content. The width is the rightmost of "Round Constants" and the function
    // row, and the height is the bottom of "A Bytes Output". Each gets the left margin, and the width also gets room for
    // the East toolbar. The stepping toolbar [`attach_toolbar`] adds is at the top, inside `TOP_Y`.
    const TOOLBAR_ALLOWANCE: f64 = 60.0;
    let a_out_rect = scene.node_rect(a_out).map_err(stringify)?;
    let right = (rc_rect.origin.x + rc_rect.size.width).max(iota_rect.origin.x + iota_rect.size.width);
    let bottom = a_out_rect.origin.y + a_out_rect.size.height;
    crate::util::resize_svg(
        &document,
        svg_id,
        Size::new(right + LEFT_X + TOOLBAR_ALLOWANCE, bottom + LEFT_X),
    )?;
    // The title and toolbar were positioned against the `<svg>`'s old size; the scene cannot observe a resize.
    scene.refresh_layout().map_err(stringify)?;

    Ok((scene, step_driver))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `child`, bound to `step_driver`, driving this nested walk's *next* position — the
/// counterpart to `theta::theta_c::attach_toolbar`. A position is one function of one round, `0..120`: five functions
/// in each of 24 rounds. `Prev`/`Next` move one position, and the stride buttons move a whole round of five. Reapplies
/// `Selection::Cell(position)` afterward for the same reason that function's own doc comment gives:
/// `show_selection_toolbar` always resets to unstarted first.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_toolbar(
    child: &Scene,
    step_driver: NodeId,
    position: Option<usize>,
    state: Rc<RefCell<KeccakState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(
            step_driver,
            // `Prev`/`Next` step one function; the stride buttons step a whole round of five.
            crate::util::step_toolbar_options().with_stride(SelectionStride::new(FUNCTIONS_PER_ROUND, "Round")),
            move |_scene, _node, transition| {
                crate::sha3_sponge::report_step(rebuild_child(transition.to, state.clone()));
            },
        )
        .map_err(stringify)?;
    if let Some(position) = position {
        child.set_selection(step_driver, Selection::Cell(position)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested Keccak-f child for walk position `to`, and grafts it into [`SCENE`]'s own `parent` in place of
/// whichever child is currently shown. The position is one function of a round, `0..120`, not a round number. This is
/// the nested counterpart to `theta::theta_c::rebuild_child`. See that function's own doc comment for why a fresh
/// `Scene`, a fresh sibling `<svg>` and a view carried over by hand are all needed here.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the DOM,
/// or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<KeccakState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (seed, base_svg_id, previous_id) = {
        let state = state.borrow();
        (state.seed, state.base_svg_id.clone(), state.child_svg_id.clone())
    };
    // This walk is never "not started": it always has round `0`'s own input. A step back from the first position (round
    // `0`'s `Theta`), or a restart, therefore lands on that position itself, selected, not on an unselected one.
    let next_id = next_child_svg_id(&base_svg_id);
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, step_driver) = build_scene(&next_id, seed, to.unwrap_or(0))?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, step_driver, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the Keccak-f scene was not initialised")?;
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
/// Builds this nested Keccak-f child at its first position — round `0`'s `Theta` — selected, against `svg_id`, and
/// wires its own stepping toolbar. `seed` is round `0`'s own real input — the sponge's own combined state flowing into
/// `Keccak-f\[1600\]`. Called once, from `sha3_sponge::build_scene`, right when "Keccak f(1600)" is added as a
/// container node.
///
/// # Errors
///
/// Returns `Err` if `svg_id` is missing from the DOM, or if any library call fails.
pub(crate) fn build_initial_scene(svg_id: &str, seed: [u64; 25]) -> Result<Scene, String> {
    let (child, step_driver) = build_scene(svg_id, seed, 0)?;
    let state = Rc::new(RefCell::new(KeccakState {
        seed,
        base_svg_id: svg_id.to_string(),
        child_svg_id: svg_id.to_string(),
    }));
    attach_toolbar(&child, step_driver, Some(0), state)?;
    Ok(child)
}
