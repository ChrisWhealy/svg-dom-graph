//! `sha3_sponge`'s own "Keccak f(1600)" node: a nested `Scene` stepping through all 24 real Keccak-f[1600] rounds.
//!
//! Each round shows the same five named sub-functions — `Theta`, `Rho`, `Pi`, `Chi`, `Iota` — arranged as one
//! vertical pipeline, plus the real round constant [`Iota`](Self) would consume. `Theta` is a genuine nested
//! `Scene`, reusing `crate::theta::build_scene` exactly as `panel-theta` does; `Rho`/`Pi`/`Chi`/`Iota` are plain
//! placeholder boxes — see [`fake_round_output`]'s own doc comment for why the round's own output is one too.

use crate::{
    theta,
    util::{create_child_svg, next_child_svg_id, required_element, stringify},
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, SceneTitleOptions, Selection,
        SelectionToolbarOptions, Side, ToolbarOptions,
    },
};

/// How many rounds `Keccak-f[1600]` actually runs.
pub(crate) const ROUND_COUNT: usize = 24;

/// `Theta`'s own nested child `<svg>`, inside this nested Keccak-f view — a fixed id, declared once in
/// `index.html`, not derived from this scene's own `svg_id` the way [`theta::build_scene`]'s own doc comment
/// ("Reuse across more than one host") otherwise recommends. This scene's own `svg_id` changes on every round
/// step (a fresh cloned sibling, via [`rebuild_child`]), but `index.html` can only declare `Theta`'s own child
/// once; [`theta::build_scene`] clearing and redrawing the same static element fresh on every round, exactly as
/// it already does on every call, is what keeps this correct regardless of which round is currently showing.
const THETA_CHILD_SVG_ID: &str = "sha3-sponge-keccak-theta-child";

/// The real Keccak-f[1600] round constants, `RC[0..24]` — see keccak.team's own specification summary
/// (<https://keccak.team/keccak_specs_summary.html>). Real reference data, unlike [`fake_round_output`]'s own
/// formula: this is the one part of this nested scene that is not a placeholder.
const ROUND_CONSTANTS: [u64; ROUND_COUNT] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808a,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808b,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008a,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000a,
    0x0000_0000_8000_808b,
    0x8000_0000_0000_008b,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800a,
    0x8000_0000_8000_000a,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

/// Live state this nested walk's own selection toolbar carries across rounds.
struct KeccakState {
    /// Round `0`'s own real input — the sponge's own combined `Capacity`/`Rate` state, handed down from
    /// `sha3_sponge::build_scene` — never itself mutated; every round's own real input/output is derived afresh
    /// from this via [`round_io`] on every step.
    seed: [u64; 25],
    /// `sha3_sponge::build_scene`'s own fixed prefix for this nested child's `<svg>` id — passed to
    /// `next_child_svg_id` on every step, never `child_svg_id` itself, so ids stay `prefix-0`, `prefix-1`, ...
    /// rather than growing a fresh suffix onto the previous one every round.
    base_svg_id: String,
    /// The id of whichever `<svg>` currently backs this nested child — see `theta::theta_c::rebuild_child`'s own
    /// doc comment for why every step needs a fresh one.
    child_svg_id: String,
}

thread_local! {
    // The top-level SHA3 Sponge `Scene`, this nested Keccak-f child `Scene`, and the container `NodeId` that owns
    // it — the same trio, for the same reasons, as `theta::theta_c`'s own `SCENE`.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
    // This round's own "Theta" node: the `Scene` it wraps — the middle rung between this module's own `SCENE`
    // (whether "Keccak f(1600)" itself is focused) and `theta::exit_if_focused` (whether one of `Theta`'s own
    // further-nested `ThetaC`/`ThetaD`/`XOR loop` is). Without this, [`exit_if_focused`] would have no way to
    // notice "Theta itself is focused" and pop back to this round's own view — only "one of `ThetaC`'s own
    // children is focused" or "nothing nested at all is focused". Replaced fresh every round, in [`build_scene`],
    // right alongside `Theta`'s own container node.
    static THETA_CHILD: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested Keccak-f child currently grafted in — the one [`exit_if_focused`]
/// and [`rebuild_child`] act on from then on. Called once, from `sha3_sponge::build_scene`, right after `child`
/// is grafted into `parent` under `node`.
pub(crate) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits whichever of this nested view's own children is currently focused, deepest first: `Theta`'s own further
/// nested `ThetaC`/`ThetaD`/`XOR loop` (via [`theta::exit_if_focused`]), then `Theta` itself (via [`THETA_CHILD`]),
/// then this Keccak-f child itself. Reports whether anything was exited — what `sha3_sponge`'s own close button
/// needs to know before giving up.
pub(crate) fn exit_if_focused() -> bool {
    if theta::exit_if_focused() {
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
/// `input`'s own clearly fake "round output" — `rotl` every lane by a round-dependent amount, then `XOR` the real
/// [`ROUND_CONSTANTS`] of `round` into every lane. Real Keccak-f's own five sub-functions compute something far
/// more structured than this — in particular `Chi`'s own real step is non-linear (`AND`/`NOT`, not `XOR`) — so
/// this formula could never be mistaken for the real permutation. It exists only to give each round's own output
/// array visibly different, round-dependent content, matching this demo's confirmed "placeholder output" scope.
fn fake_round_output(input: [u64; 25], round: usize) -> [u64; 25] {
    let shift = (round as u32 % 63) + 1;
    std::array::from_fn(|lane| input[lane].rotate_left(shift) ^ ROUND_CONSTANTS[round])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every round's own `(input, output)` pair, chained from `seed`: round `0`'s own input is `seed` itself, and
/// every later round's own input is the previous round's own [`fake_round_output`] — see
/// `sha3_sponge::keccak`'s own module doc comment, "the bottom A Bytes node is initialised and the data copied to
/// the top A Bytes node" (point 7 of the original request), for why this chains rather than recomputing `seed`
/// fresh every round.
fn round_io(seed: [u64; 25]) -> [([u64; 25], [u64; 25]); ROUND_COUNT] {
    let mut pairs = [([0u64; 25], [0u64; 25]); ROUND_COUNT];
    let mut current = seed;
    for (round, pair) in pairs.iter_mut().enumerate() {
        let output = fake_round_output(current, round);
        *pair = (current, output);
        current = output;
    }
    pairs
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `svg_id` from scratch for round `round` (`0..24`), `None` meaning round `0`, unstarted.
///
/// # What this draws
///
/// Row 1 holds "A Bytes - round `{round}`" — the real 25-lane state this round starts from, [`round_io`]'s own
/// `round`th input, derived from `seed`. Beside it, "Round Constants" holds all 24 real [`ROUND_CONSTANTS`] as a
/// 4-column, 6-row grid — a single column reads just as well but takes far more vertical space. Neither is
/// especially tall, so both fit row 1 together, keeping this round's own overall height down further still,
/// rather than stacking "Round Constants" under the function row below instead.
/// `Selection::Cell` addresses flat index `round` regardless of the grid's own shape, so the cell highlighted
/// stays correct either way.
///
/// Below row 1, five named boxes run left to right, not stacked top to bottom: `Theta`, `Rho`, `Pi`, `Chi`,
/// `Iota`. A horizontal row keeps this round's own overall height close to row 1's own, instead of adding five
/// more row-heights on top — a nested Scene's own viewBox, however tall, still only ever displays within its own
/// parent's fixed-size frame. `Theta` is a genuine container node, nesting [`theta::build_scene`] exactly as
/// `panel-theta` draws it standalone — click it to drill in, the &times; in its own frame's corner to come back.
/// `Rho`, `Pi`, `Chi`, and `Iota` are plain placeholder boxes: implementing SHA3's real rotate-by-offset,
/// lane-permute, non-linear-combine, and constant-XOR steps is out of scope here — see [`fake_round_output`]'s
/// own doc comment. A connector from "Round Constants" down into `Iota`'s own east side marks it as the one real
/// value `Iota` actually consumes each round. `Iota`'s own output is not actually computed from it, though.
///
/// The bottom row is "A Bytes Output" — [`round_io`]'s own `round`th output, [`fake_round_output`]'s placeholder
/// result, not a real permutation of row 1.
///
/// # Stepping through it
///
/// "Round" is a small, otherwise-meaningless [`ROUND_COUNT`]-value array placed far off-canvas, purely to drive a
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar — the same trick
/// `sha3_sponge::build_scene`'s own "Step" uses, for the same reason: no node already in this diagram happens to
/// have exactly [`ROUND_COUNT`] cells. [`rebuild_child`] wires the bar itself.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `svg_id`, or if any library call fails.
fn build_scene(svg_id: &str, seed: [u64; 25], round: Option<usize>) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let r = round.unwrap_or(0);
    scene
        .show_scene_title(format!("Keccak f(1600) Round {r}"), SceneTitleOptions::default())
        .map_err(stringify)?;

    let (a_in, a_out) = round_io(seed)[r];
    let hex25 = |values: [u64; 25]| DataNodeContent::new(NodeValues::U64(values.to_vec()), DataFormat::Hexadecimal);

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const V_GAP: f64 = 50.0;
    const H_GAP: f64 = 80.0;
    let fn_size = Size::new(130.0, 60.0);
    let fn_gap = 30.0;

    let a_in_label = format!("A Bytes - round {r}");
    let a_top = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), &a_in_label, hex25(a_in))
        .map_err(stringify)?;
    let a_top_rect = scene.node_rect(a_top).map_err(stringify)?;

    // "Round Constants" sits beside "A Bytes - round {round}", in the same row, not below the function row —
    // row 1 has room for both, and this keeps the round's own overall height down further still.
    let rc_x = a_top_rect.origin.x + a_top_rect.size.width + H_GAP;
    let rc = scene
        .add_named_data_node(
            Point::new(rc_x, TOP_Y),
            "Round Constants",
            DataNodeContent::new(NodeValues::U64(ROUND_CONSTANTS.to_vec()), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Columns(4)),
        )
        .map_err(stringify)?;
    scene.set_selection(rc, Selection::Cell(r)).map_err(stringify)?;
    let rc_rect = scene.node_rect(rc).map_err(stringify)?;

    // Theta/Rho/Pi/Chi/Iota run left to right, not stacked top to bottom: a horizontal row keeps this round's own
    // overall height close to its row 1's own height, rather than adding five more row-heights on top —
    // important since a nested Scene's own viewBox, however tall, still only ever displays within its own
    // parent's fixed-size frame.
    let row_stride = fn_size.width + fn_gap;
    let row_width = 5.0 * fn_size.width + 4.0 * fn_gap;
    let row_x = a_top_rect.origin.x + (a_top_rect.size.width - row_width) / 2.0;
    // Below whichever of "A Bytes" or "Round Constants" extends further down — 24 real values, even as 4 columns,
    // can still make "Round Constants" taller than "A Bytes - round {round}"'s own 5 rows.
    let row_y = (a_top_rect.origin.y + a_top_rect.size.height).max(rc_rect.origin.y + rc_rect.size.height) + V_GAP;

    // Fixed, not derived from `svg_id`: `svg_id` itself changes on every round step (a fresh sibling `<svg>`,
    // exactly like `theta::theta_c`'s own per-step clones), but `index.html` can only ever declare one static
    // element for Theta's own child — see this constant's own doc comment.
    // `false`: this round's own `svg_id` is not the shallowest level in `#sha3-sponge-diagram`'s own
    // `.nested-scene-stage` — see `theta::build_scene`'s own doc comment ("Reuse across more than one host") for
    // why a backdrop here would wrongly cover whatever shallower sibling sits behind it.
    let theta_child = theta::build_scene(THETA_CHILD_SVG_ID, false)?;
    THETA_CHILD.with_borrow_mut(|slot| *slot = Some(theta_child.clone()));
    let theta = scene
        .add_container_node(Point::new(row_x, row_y), fn_size, "Theta", theta_child)
        .map_err(stringify)?;
    scene.make_enterable(theta).map_err(stringify)?;

    let rho = scene
        .add_node(Point::new(row_x + row_stride, row_y), fn_size, "Rho")
        .map_err(stringify)?;
    let pi = scene
        .add_node(Point::new(row_x + 2.0 * row_stride, row_y), fn_size, "Pi")
        .map_err(stringify)?;
    let chi = scene
        .add_node(Point::new(row_x + 3.0 * row_stride, row_y), fn_size, "Chi")
        .map_err(stringify)?;
    let iota = scene
        .add_node(Point::new(row_x + 4.0 * row_stride, row_y), fn_size, "Iota")
        .map_err(stringify)?;
    let iota_rect = scene.node_rect(iota).map_err(stringify)?;

    let a_out_y = iota_rect.origin.y + iota_rect.size.height + V_GAP;
    let a_out = scene
        .add_named_data_node(Point::new(LEFT_X, a_out_y), "A Bytes Output", hex25(a_out))
        .map_err(stringify)?;

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
                .with_from_side(Some(Side::South))
                .with_to_side(Some(Side::East)),
        )
        .map_err(stringify)?;

    // "Round": see this function's own doc comment, "Stepping through it", for why this exists and why it sits
    // far off-canvas rather than anywhere a reader would actually see it.
    let round_values: Vec<u8> = (1..=ROUND_COUNT as u8).collect();
    let round_driver = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Round",
            DataNodeContent::new(NodeValues::U8(round_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    // No backdrop of `svg_id` here, unlike `theta::build_scene`'s own call for itself: `svg_id` is not the
    // shallowest level in `#sha3-sponge-diagram`'s own `.nested-scene-stage` either — see that function's own
    // doc comment ("Reuse across more than one host") for why one here would wrongly cover
    // `#sha3-sponge-diagram` itself. Entering `Theta` from here shows `#sha3-sponge-diagram`'s own backdrop in
    // the margin instead, not this round's own content — a smaller visual polish this feature does without.

    Ok((scene, round_driver))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `child`, bound to `round_driver`, driving this nested walk's *next* round — the
/// counterpart to `theta::theta_c::attach_toolbar`. Reapplies `Selection::Cell(round)` afterward for the same
/// reason that function's own doc comment gives: `show_selection_toolbar` always resets to unstarted first.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_toolbar(
    child: &Scene,
    round_driver: NodeId,
    round: Option<usize>,
    state: Rc<RefCell<KeccakState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(
            round_driver,
            SelectionToolbarOptions::default(),
            move |_scene, _node, transition| {
                let _ = rebuild_child(transition.to, state.clone());
            },
        )
        .map_err(stringify)?;
    if let Some(round) = round {
        child.set_selection(round_driver, Selection::Cell(round)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested Keccak-f child for round `to`, and grafts it into [`SCENE`]'s own `parent` in place of
/// whichever child is currently shown — the nested counterpart to `theta::theta_c::rebuild_child`; see that
/// function's own doc comment for why a fresh `Scene`, a fresh sibling `<svg>`, and a view carried over by hand
/// are all needed here for exactly the same reasons.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the
/// DOM, or if any library call fails.
fn rebuild_child(to: Option<usize>, state: Rc<RefCell<KeccakState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let (seed, base_svg_id, previous_id) = {
        let state = state.borrow();
        (state.seed, state.base_svg_id.clone(), state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id(&base_svg_id);
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, round_driver) = build_scene(&next_id, seed, to)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, round_driver, to, state.clone())?;

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
/// Builds this nested Keccak-f child, unstarted (round `1`), against `svg_id`, and wires its own stepping
/// toolbar. `seed` is round `1`'s own real input — the sponge's own combined state flowing into `Keccak-f[1600]`.
/// Called once, from `sha3_sponge::build_scene`, right when "Keccak f(1600)" is added as a container node.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `svg_id`, or if any library call fails.
pub(crate) fn build_initial_scene(svg_id: &str, seed: [u64; 25]) -> Result<Scene, String> {
    let (child, round_driver) = build_scene(svg_id, seed, None)?;
    let state = Rc::new(RefCell::new(KeccakState {
        seed,
        base_svg_id: svg_id.to_string(),
        child_svg_id: svg_id.to_string(),
    }));
    attach_toolbar(&child, round_driver, None, state)?;
    Ok(child)
}
