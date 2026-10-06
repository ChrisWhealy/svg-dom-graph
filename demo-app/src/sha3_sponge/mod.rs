//! `panel-sha3-sponge` / `#sha3-sponge-diagram`: SHA3's own sponge construction, one 64-byte input block, top-level
//! view only — see [`build_scene`]'s own doc comment for exactly what this first step draws, and what it
//! deliberately does not yet.

mod chi;
mod iota;
mod keccak;
mod pi;
mod rho;
pub(crate) mod theta;

use crate::util::{add_backdrop_clone, ensure_svg, required_element, stringify};
use std::cell::RefCell;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeOptions, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`. Every listener
    // `Scene::show_toolbar`/`Scene::show_selection_toolbar` installs holds only a `Weak` reference back to it, so
    // without a strong handle kept alive somewhere, the current `Scene` would drop the moment [`rebuild`] returns,
    // and every button's own `Weak::upgrade` would silently fail forever after.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// `Keccak-f[1600]`'s own state is 25 `u64` lanes. This demo splits them 17/8 between "Capacity" and "Rate" —
/// a split chosen for this demo, not the real rate/capacity of any named SHA3 variant.
const CAPACITY_LANES: usize = 17;
/// "Rate" and "Input block" both hold this many lanes — see [`CAPACITY_LANES`]'s own doc comment.
const RATE_LANES: usize = 8;
/// "Output Hash"'s own lane count — a real hash length, deliberately not [`RATE_LANES`].
const HASH_LANES: usize = 4;

/// "Input block"'s own [`RATE_LANES`] lanes, the one real input this demo absorbs.
const INPUT_BLOCK: [u64; RATE_LANES] = [
    0xea27_f99a_ae29_90e9,
    0x725d_7fa6_a3fc_2f70,
    0x6779_6880_0823_53e4,
    0xe8c0_2848_00b1_9533,
    0xfec1_294f_291c_1bc8,
    0xadea_5a3b_0abf_3906,
    0xa94e_bb2f_1bd3_f309,
    0xbb11_4511_e928_a5df,
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `count` clearly fake, deterministic `u64` lanes, starting at `base` and counting up by one. Nothing here is a
/// real Keccak state — see [`build_scene`]'s own doc comment for why. `base` only keeps one array's own lanes
/// visibly distinct from another's.
fn placeholder_lanes(base: u64, count: usize) -> Vec<u64> {
    (0..count as u64).map(|i| base + i).collect()
}

/// How many stages "Step"'s own selection toolbar walks — one flat position per stage [`apply_stage`] focuses.
const STAGE_COUNT: usize = 5;

/// Every node [`apply_stage`] ever focuses, by name — one small record instead of five separate parameters
/// repeated at every call site.
#[derive(Clone, Copy)]
struct StageNodes {
    rate_in: NodeId,
    input_block: NodeId,
    xor: NodeId,
    keccak: NodeId,
    capacity_out: NodeId,
    rate_out: NodeId,
    output_hash: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Focuses whichever of `nodes` stage `to` puts in focus, via
/// [`Scene::set_focus`](svg_dom_graph::scene::Scene::set_focus), and un-focuses every other node this walk ever
/// touches first. `None` — unstarted, or walked/restarted all the way back — focuses nothing.
///
/// Recomputing the full set from `to` alone, rather than tracking "what was focused last" separately, matches
/// `crate::selection::display_outputs`'s own rule. "Previous" un-focuses a later stage exactly as readily as
/// "Next" focuses one, with nothing left over from before to forget to clear.
fn apply_stage(scene: &Scene, nodes: StageNodes, to: Option<usize>) {
    let StageNodes {
        rate_in,
        input_block,
        xor,
        keccak,
        capacity_out,
        rate_out,
        output_hash,
    } = nodes;
    for node in [rate_in, input_block, xor, keccak, capacity_out, rate_out, output_hash] {
        let _ = scene.set_focus(node, false);
    }
    let focus = |ids: &[NodeId]| {
        for &id in ids {
            let _ = scene.set_focus(id, true);
        }
    };
    match to {
        Some(0) => focus(&[input_block, rate_in]),
        Some(1) => focus(&[xor]),
        Some(2) => focus(&[keccak]),
        Some(3) => focus(&[capacity_out, rate_out]),
        Some(4) => focus(&[output_hash]),
        _ => {},
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates every `<svg>` this panel's own `.nested-scene-stage` needs, if they do not already exist: the base
/// diagram, then each nested level, shallowest first — document order is paint order, and
/// [`add_backdrop_clone`] relies on a nested child following its own parent. Each is inserted before
/// `#sha3-sponge-close`, which therefore stays on top. These sizes are the *initial* ones only: each nested
/// scene fits its own `<svg>` to its content as it is built, and a later call here never reverts that.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-close`, or if creating any `<svg>` fails.
fn create_stage_svgs(document: &web_sys::Document) -> Result<(), String> {
    const ANCHOR: &str = "sha3-sponge-close";
    let levels: [(&str, Option<&str>, Size); 10] = [
        ("sha3-sponge-diagram", None, Size::new(1920.0, 950.0)),
        ("sha3-sponge-keccak-child", Some("nested-scene"), Size::new(1980.0, 820.0)),
        (
            "sha3-sponge-keccak-iota-child",
            Some("nested-scene nested-scene-depth-2"),
            Size::new(1000.0, 800.0),
        ),
        (
            "sha3-sponge-keccak-chi-child",
            Some("nested-scene nested-scene-depth-2"),
            Size::new(1000.0, 1000.0),
        ),
        (
            "sha3-sponge-keccak-pi-child",
            Some("nested-scene nested-scene-depth-2"),
            Size::new(1000.0, 900.0),
        ),
        (
            "sha3-sponge-keccak-rho-child",
            Some("nested-scene nested-scene-depth-2"),
            Size::new(1000.0, 800.0),
        ),
        (
            "sha3-sponge-keccak-theta-child",
            Some("nested-scene nested-scene-depth-2"),
            Size::new(1260.0, 1090.0),
        ),
        (
            "sha3-sponge-keccak-theta-child-thetac-child",
            Some("nested-scene nested-scene-depth-3"),
            Size::new(1180.0, 1010.0),
        ),
        (
            "sha3-sponge-keccak-theta-child-thetad-child",
            Some("nested-scene nested-scene-depth-3"),
            Size::new(1130.0, 600.0),
        ),
        (
            "sha3-sponge-keccak-theta-child-xorloop-child",
            Some("nested-scene nested-scene-depth-3"),
            Size::new(1400.0, 780.0),
        ),
    ];
    for (id, class, size) in levels {
        ensure_svg(document, ANCHOR, id, class, size)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the SHA3 Sponge demo's own top-level `Scene` for walk position `stage`: one 64-byte input block passing
/// through a single call to `Keccak-f[1600]`. Nothing here drags.
///
/// # What this draws
///
/// Row 1 is the sponge's own starting state, split into two named arrays: "Capacity"
/// ([`CAPACITY_LANES`] lanes) and "Rate" ([`RATE_LANES`] lanes). The two are butted up against each other with no
/// gap — one combined state, drawn as two named halves. Both start all-zero, the real sponge construction's own
/// state before any input is absorbed.
///
/// Row 2 holds three boxes. "Keccak f(1600)" sits under "Capacity", centred vertically on "XOR" rather than
/// sharing its own top edge, fed by a connector from "Capacity" above. It is a plain box for now, not a container
/// node — there is no nested `Scene` behind it yet (see "What this does not draw yet" below). Its own three
/// [`EdgeAnchors`] per side put north and east on their shared centre, south's two connectors on the centre and
/// one outer point — see "Row 3" below for why. "XOR" sits under "Rate", fed by connectors from "Rate" and from
/// "Input block" (further right) — [`INPUT_BLOCK`] itself, the one real input this demo absorbs, not a
/// placeholder. A further connector carries "XOR"'s own result back into "Keccak f(1600)" — the absorbed rate
/// joining "Capacity" as the combined state the real permutation would run over.
///
/// "XOR" shows all-zero lanes, the same "not yet written" convention row 3 below already follows, until `stage`
/// reaches it — see "Stepping through it" below. Only then does it show "Rate" and "Input block"'s own real,
/// elementwise XOR, computed here in plain Rust.
///
/// Row 3 repeats the "Capacity"/"Rate" pair, sharing row 1's own left edge so the two rows stay vertically
/// aligned. "Rate" sits far enough east of "Keccak f(1600)"'s own centre that the connector into it still lands
/// on an outer fixing point. "Capacity" sits directly below, so its own connector lands on the centre one
/// instead. The one into "Rate" enters its own north side, the same as "Capacity"'s own. "Output Hash", further
/// right, holds its own [`HASH_LANES`] lanes — a real hash length, not this row's own wider "Rate" — fed by a
/// connector from it.
///
/// # Stepping through it
///
/// "Step" is a small, otherwise-meaningless [`STAGE_COUNT`]-value array placed far off-canvas, purely to drive a
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar. That bar needs a
/// data node with exactly as many cells as the walk has stages, and no node already in this diagram happens to
/// hold that count. [`rebuild`] wires the bar itself; this function only draws "Step" and applies `stage`'s own
/// focus, via [`apply_stage`].
///
/// Each stage rings a different set of nodes, un-ringing whichever it moves away from. In order: "Input block"
/// and "Rate" together, "XOR" alone, "Keccak f(1600)" alone, row 3's own "Capacity" and "Rate" together, then
/// "Output Hash" alone. Unstarted, nothing is focused.
///
/// # What this does not draw yet
///
/// "Keccak f(1600)" is a plain box, not a container node: drilling into a real nested Keccak `Scene` is a later
/// step. It does not yet compute anything, so row 3's own "Capacity"/"Rate" are only [`placeholder_lanes`], not
/// real outputs of row 1's own values. "XOR" is the one real computation this diagram performs, since both of its
/// own inputs — row 1's own all-zero "Rate" and the real [`INPUT_BLOCK`] — are already on the page. Processing
/// more than one 64-byte block, and SHA3's XOF mode, are both out of scope here too.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-diagram`, or if any library call fails.
fn build_scene(stage: Option<usize>) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    create_stage_svgs(&document)?;
    let container = required_element(&document, "sha3-sponge-diagram")?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach("sha3-sponge-diagram").map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("SHA3 Sponge", SceneTitleOptions::default())
        .map_err(stringify)?;

    let hex = |values: Vec<u64>| DataNodeContent::new(NodeValues::U64(values), DataFormat::Hexadecimal);
    // "Rate", "XOR", and "Input block" all hold `RATE_LANES` lanes; `GridLayout::Automatic`'s own default for that
    // count is 4 columns of 2, wider than this diagram needs. 2 columns of 4 instead, to keep the whole diagram
    // narrower.
    let hex_narrow = |values: Vec<u64>| hex(values).with_layout(GridLayout::Columns(2));

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const H_GAP: f64 = 40.0;
    const V_GAP: f64 = 50.0;
    let keccak_size = Size::new(160.0, 70.0);

    // Row 1: the sponge's own starting state — "Capacity" and "Rate", butted up against each other with no gap,
    // reading as one combined state split into two named halves rather than two separate values. The real sponge
    // construction starts from an all-zero state, before any input is absorbed, so both halves start that way too.
    let capacity_in = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Capacity", hex(vec![0; CAPACITY_LANES]))
        .map_err(stringify)?;
    let capacity_in_rect = scene.node_rect(capacity_in).map_err(stringify)?;

    let rate_in_values = vec![0; RATE_LANES];
    let rate_in = scene
        .add_named_data_node(
            Point::new(capacity_in_rect.origin.x + capacity_in_rect.size.width, TOP_Y),
            "Rate",
            hex_narrow(rate_in_values.clone()),
        )
        .map_err(stringify)?;
    let rate_in_rect = scene.node_rect(rate_in).map_err(stringify)?;

    // Row 2: "Keccak f(1600)" under "Capacity", "XOR" under "Rate", "Input block" further right of "XOR".
    let row_2_y = capacity_in_rect.origin.y.max(rate_in_rect.origin.y)
        + capacity_in_rect.size.height.max(rate_in_rect.size.height)
        + V_GAP;

    // "XOR" is a plain data node holding the real elementwise XOR of "Rate" and "Input block", not a genuine
    // operator node: `Scene::add_binary_operator_node`'s own result must be exactly one value, which an
    // `RATE_LANES`-lane array is not. Its own size is measured ahead of drawing it, both to centre it under
    // "Rate" and to centre "Keccak f(1600)" on it below.
    //
    // Shows all-zero lanes until `stage` reaches it (see this function's own doc comment, "Stepping through it"),
    // the same "not yet written" convention row 3 below already follows — real values would otherwise appear
    // before the walk ever visits "XOR", reading as already computed when it is not.
    let input_block_values = INPUT_BLOCK.to_vec();
    let xor_display_values: Vec<u64> = if stage.is_some_and(|stage| stage >= 1) {
        rate_in_values
            .iter()
            .zip(input_block_values.iter())
            .map(|(rate, input)| rate ^ input)
            .collect()
    } else {
        vec![0; RATE_LANES]
    };
    let xor_content = hex_narrow(xor_display_values.clone());
    let xor_size = scene.measure_named_data_node("XOR", &xor_content).map_err(stringify)?;
    let xor_x = rate_in_rect.origin.x + (rate_in_rect.size.width - xor_size.width) / 2.0;
    let xor = scene
        .add_named_data_node(Point::new(xor_x, row_2_y), "XOR", xor_content)
        .map_err(stringify)?;
    let xor_rect = scene.node_rect(xor).map_err(stringify)?;

    // Centred under "Capacity" horizontally, and vertically centred on "XOR" — "XOR" is taller now that it is
    // drawn 2 columns wide (see `hex_narrow`'s own doc comment), so a shared top edge no longer reads as level.
    //
    // Three fixing points per side. The connector leaving north (from "Capacity") and the one leaving east (from
    // "XOR") both land on the centre point, since this box's own centre lines up with each of theirs. "Rate" sits
    // well east of this box's own centre, since row 3 shares row 1's own left edge — see `row_3_y`'s own comment
    // below. The connector leaving south towards it still lands on the east fixing point, not the centre one.
    //
    // The connector towards "Capacity", directly below, lands on the centre fixing point instead. Row 3
    // realigning with row 1 leaves this box's own centre no longer off-centre from "Capacity", unlike from "Rate".
    let keccak_x = capacity_in_rect.origin.x + (capacity_in_rect.size.width - keccak_size.width) / 2.0;
    let keccak_y = xor_rect.origin.y + (xor_rect.size.height - keccak_size.height) / 2.0;

    // The real combined 25-lane state flowing into `Keccak-f[1600]` — "Capacity" (always all-zero in this demo)
    // plus whatever "XOR" currently shows, the same "not yet written" convention XOR itself follows (see this
    // function's own doc comment, "Stepping through it"): all-zero before `stage` reaches it, the real elementwise
    // XOR afterward. `keccak::build_initial_scene`'s own round 1 starts from exactly this.
    let a_bytes_seed: [u64; 25] = std::array::from_fn(|lane| {
        if lane < CAPACITY_LANES {
            0
        } else {
            xor_display_values[lane - CAPACITY_LANES]
        }
    });
    let keccak_child = keccak::build_initial_scene("sha3-sponge-keccak-child", a_bytes_seed)?;
    let keccak = scene
        .add_container_node_with(
            Point::new(keccak_x, keccak_y),
            keccak_size,
            "Keccak f(1600)",
            keccak_child.clone(),
            NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3))),
        )
        .map_err(stringify)?;
    scene.make_enterable(keccak).map_err(stringify)?;
    let keccak_rect = scene.node_rect(keccak).map_err(stringify)?;

    let input_block = scene
        .add_named_data_node(
            Point::new(xor_rect.origin.x + xor_rect.size.width + H_GAP, row_2_y),
            "Input block",
            hex_narrow(input_block_values),
        )
        .map_err(stringify)?;

    // Row 3: "Capacity"/"Rate" again, fed out of "Keccak f(1600)" above, then "Output Hash" further right. Each
    // box's own bottom edge is computed independently now that "Keccak f(1600)" and "XOR" no longer share a
    // common top — see `keccak_y`'s own comment above.
    //
    // The elbow connector from "Keccak f(1600)" to "Rate" jogs horizontally halfway between "Keccak f(1600)"'s own
    // bottom edge and row 3's own top edge — see `elbow_route`'s own `mid_y` rule. A plain `V_GAP` below the taller
    // of "Keccak f(1600)"/"XOR" is not always enough on its own. If that halfway point still sits above "XOR"'s
    // own bottom edge, the connector's own horizontal jog crosses straight through it. `ROW_3_CLEARANCE` is the
    // least headroom to leave below "XOR" once that jog is accounted for.
    const ROW_3_CLEARANCE: f64 = 15.0;
    let keccak_bottom = keccak_rect.origin.y + keccak_rect.size.height;
    let xor_bottom = xor_rect.origin.y + xor_rect.size.height;
    let row_3_y = (xor_bottom + V_GAP)
        .max(keccak_bottom + V_GAP)
        .max(2.0 * (xor_bottom + ROW_3_CLEARANCE) - keccak_bottom);

    // "Capacity"/"Rate" are butted together as before, starting at the same `LEFT_X` as row 1's own pair, so the
    // two stay vertically aligned. See `keccak`'s own construction above for how this shapes which of its own
    // south fixing points each connector out of it lands on.
    let capacity_out = scene
        .add_named_data_node(
            Point::new(LEFT_X, row_3_y),
            "Capacity",
            hex(placeholder_lanes(0x0400, CAPACITY_LANES)),
        )
        .map_err(stringify)?;
    let capacity_out_rect = scene.node_rect(capacity_out).map_err(stringify)?;

    // Butted up against "Capacity" with no gap, the same as row 1's own pair — see its own comment above.
    let rate_out = scene
        .add_named_data_node(
            Point::new(capacity_out_rect.origin.x + capacity_out_rect.size.width, row_3_y),
            "Rate",
            hex_narrow(placeholder_lanes(0x0500, RATE_LANES)),
        )
        .map_err(stringify)?;
    let rate_out_rect = scene.node_rect(rate_out).map_err(stringify)?;

    let output_hash = scene
        .add_named_data_node(
            Point::new(rate_out_rect.origin.x + rate_out_rect.size.width + H_GAP, row_3_y),
            "Output Hash",
            hex_narrow(placeholder_lanes(0x0, HASH_LANES)),
        )
        .map_err(stringify)?;

    // Every vertical connector below is forced South-to-North, and every horizontal one East/West-to-West/East —
    // matching this codebase's own convention of never relying on the automatic ray-cast when a clean relationship
    // already exists between two boxes (see e.g. `theta::xor_loop::build_scene`'s own `vertical` helper).
    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    scene.add_edge_with(capacity_in, keccak, vertical()).map_err(stringify)?;
    scene.add_edge_with(rate_in, xor, vertical()).map_err(stringify)?;
    scene
        .add_edge_with(
            input_block,
            xor,
            ConnectorOptions::default()
                .with_from_side(Some(Side::West))
                .with_to_side(Some(Side::East)),
        )
        .map_err(stringify)?;
    // "XOR"'s own result feeds back into "Keccak f(1600)", to its own west — the absorbed rate joining "Capacity"
    // as the combined state the real permutation would run over.
    scene
        .add_edge_with(
            xor,
            keccak,
            ConnectorOptions::default()
                .with_from_side(Some(Side::West))
                .with_to_side(Some(Side::East)),
        )
        .map_err(stringify)?;
    scene.add_edge_with(keccak, capacity_out, vertical()).map_err(stringify)?;
    scene.add_edge_with(keccak, rate_out, vertical()).map_err(stringify)?;
    scene
        .add_edge_with(
            rate_out,
            output_hash,
            ConnectorOptions::default()
                .with_from_side(Some(Side::East))
                .with_to_side(Some(Side::West)),
        )
        .map_err(stringify)?;

    // "Step": see this function's own doc comment, "Stepping through it", for why this exists and why it sits far
    // off-canvas rather than anywhere a reader would actually see it.
    let step_values: Vec<u8> = (1..=STAGE_COUNT as u8).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U8(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    let stage_nodes = StageNodes {
        rate_in,
        input_block,
        xor,
        keccak,
        capacity_out,
        rate_out,
        output_hash,
    };
    apply_stage(&scene, stage_nodes, stage);

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Backdrops `#sha3-sponge-diagram` itself, so entering "Keccak f(1600)" shows this diagram's own current
    // content behind its frame, instead of the page's own plain background. Safe now that `.nested-scene-backdrop`
    // only ever paints while its own source itself has `visibility="hidden"` (see that CSS rule's own doc
    // comment) — the clone no longer sits on top of this diagram's own real, interactive view while nothing is
    // nested, so its own zoom/pan buttons stay visibly correct between rebuilds.
    add_backdrop_clone(&document, "sha3-sponge-diagram")?;
    keccak::init_scene(scene.clone(), keccak_child, keccak);

    Ok((scene, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the whole diagram for walk position `stage`, via [`build_scene`], and wires a fresh
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar onto its own
/// freshly drawn "Step" to drive the *next* step.
///
/// A fresh rebuild is unavoidable here: `svg-dom-graph` has no way to change a node's own displayed value once
/// drawn. "XOR" must switch from all-zero to its own real result partway through this walk — see
/// [`build_scene`]'s own doc comment. [`apply_stage`] runs again inside [`build_scene`] itself on every rebuild,
/// so each fresh diagram already shows `stage`'s own correct focus from the moment it is drawn.
///
/// `show_selection_toolbar` always resets "Step" to [`Selection::None`] as its own first committed act,
/// regardless of `stage`. So `stage`'s own real position is reapplied immediately afterward — not for "Step"'s
/// own look, which is never seen. It keeps the toolbar's *next* click advancing from the right position, rather
/// than from unstarted every time.
///
/// A fresh `Scene` would otherwise also reset zoom/pan back to `1.0`/`(0, 0)` — jarring, mid-walk, if the
/// previous step's own view had been zoomed or panned in first. So the outgoing `Scene`'s own
/// [`Scene::view`](svg_dom_graph::scene::Scene::view) is read before it is replaced, and carried over onto the
/// fresh one via [`Scene::set_view`](svg_dom_graph::scene::Scene::set_view).
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-diagram`, or if any library call fails.
fn rebuild(stage: Option<usize>) -> Result<(), String> {
    let view = SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));

    let (scene, step) = build_scene(stage)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(step, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            let _ = rebuild(transition.to);
        })
        .map_err(stringify)?;
    if let Some(stage) = stage {
        scene.set_selection(step, Selection::Cell(stage)).map_err(stringify)?;
    }

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the SHA3 Sponge demo, unstarted — see [`build_scene`]'s own doc comment for what it draws — and wires
/// its own close button.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-diagram`/`#sha3-sponge-close`, or if any library call
/// fails.
pub(crate) fn build_sha3_sponge_demo() -> Result<(), String> {
    rebuild(None)?;
    wire_sha3_sponge_controls(crate::util::document()?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#sha3-sponge-close` — the nested view's own &times; close button — to whichever of "Keccak f(1600)"'s
/// own descendants is currently entered, via [`keccak::exit_if_focused`]. Wired once, here, not from inside
/// [`rebuild`]: unlike the `<svg>` content `rebuild` redraws from scratch on every step, `#sha3-sponge-close` is
/// a plain, static `index.html` element that would otherwise pick up one more duplicate listener per step.
/// [`keccak::exit_if_focused`] always reads `keccak`'s own latest thread-local state at click time, regardless of
/// how many rebuilds happened since this listener was attached.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-close`, or if a listener could not be attached to it.
fn wire_sha3_sponge_controls(document: web_sys::Document) -> Result<(), String> {
    let close = required_element(&document, "sha3-sponge-close")?;

    let close_closure = Closure::<dyn FnMut()>::new(move || {
        keccak::exit_if_focused();
    });
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the SHA3 Sponge close-button listener: {e:?}"))?;
    close_closure.forget();

    Ok(())
}
