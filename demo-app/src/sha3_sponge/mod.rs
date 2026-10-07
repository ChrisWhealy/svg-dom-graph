//! `panel-sha3-sponge` / `#sha3-sponge-diagram`: SHA3-256 of one short message, drawn as a sponge — see
//! [`build_scene`]'s own doc comment for what the top-level diagram shows.
//!
//! # What is real
//!
//! All of it. [`keccak_f::sha3_256_run`] hashes [`MESSAGE`] in plain Rust. Every value on screen is a piece of that one
//! calculation. That covers this diagram's "Input block", "XOR" and row 3. It covers the Keccak scene's 24 rounds,
//! where each round's "A Bytes" is the previous round's real output. It also covers `Theta`, `Rho`, `Pi`, `Chi` and
//! `Iota` inside each round, each handed the state the function before it produced. Stepping over a nested scene
//! therefore gives exactly what stepping into it would have ended on. [`keccak_f`]'s own tests check it against FIPS
//! 202's own formulas and the published SHA3-256 digests.
//!
//! # What is not covered
//!
//! Three things are not covered. First, a message of a block (136 bytes) or more, which needs more than one absorb.
//! Second, SHA3's other variants and its XOF modes. Third, the `Theta` scenes' own orientation: they index the state as
//! `A[x][y]`, so they show it transposed relative to the rest (see [`keccak_f::to_theta_grid`]).
//!
//! The standalone `panel-theta` and `panel-selection` demos are separate: they run over their own made-up input, since
//! there is no sponge behind them.

mod chi;
mod iota;
mod keccak;
mod keccak_f;
mod pi;
mod rho;
pub(crate) mod theta;

use crate::util::{add_backdrop_clone, ensure_svg, required_element, stringify};
use keccak_f::sha3_256_run;
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
    // `Scene::show_toolbar`/`Scene::show_selection_toolbar` installs holds only a `Weak` reference back to it. Without
    // a strong handle kept alive somewhere, the current `Scene` would drop the moment [`rebuild`] returns. Every
    // button's own `Weak::upgrade` would then silently fail forever after.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// `SHA3-256`'s own parameters (FIPS 202, table 3). `Keccak-f\[1600\]`'s state is 25 lanes. The first [`RATE_LANES`]
/// hold the "Rate", the part input is XORed into and the digest is read from. The remaining [`CAPACITY_LANES`] hold the
/// "Capacity", which input never touches. This diagram draws "Capacity" on the left, above "Keccak f(1600)", and "Rate"
/// on the right, above "XOR", so each sits over the node it feeds. The state's own lane order is Rate first.
const RATE_LANES: usize = keccak_f::SHA3_256_RATE_LANES;
const CAPACITY_LANES: usize = 25 - RATE_LANES;
/// "Output Hash"'s own lane count: a 256-bit digest.
const HASH_LANES: usize = 4;

/// The one message this demo hashes. Short enough for a single absorb: one block of [`RATE_LANES`] lanes.
const MESSAGE: &[u8] = b"The quick brown fox jumps over the lazy dog";

/// How many stages "Step"'s own selection toolbar walks — one flat position per stage [`apply_stage`] focuses.
const STAGE_COUNT: usize = 6;

/// Every node [`apply_stage`] ever focuses, by name — one small record instead of five separate parameters repeated at
/// every call site.
#[derive(Clone, Copy)]
struct StageNodes {
    message: NodeId,
    rate_in: NodeId,
    input_block: NodeId,
    xor: NodeId,
    keccak: NodeId,
    capacity_out: NodeId,
    rate_out: NodeId,
    output_hash: NodeId,
}

/// Focuses whichever of `nodes` stage `to` puts in focus, via
/// [`Scene::set_focus`](svg_dom_graph::scene::Scene::set_focus), and un-focuses every other node this walk ever touches
/// first. `None` — unstarted, or walked/restarted all the way back — focuses nothing.
///
/// Recomputing the full set from `to` alone, rather than tracking "what was focused last" separately, matches
/// `crate::selection::display_outputs`'s own rule. "Previous" un-focuses a later stage exactly as readily as "Next"
/// focuses one, with nothing left over from before to forget to clear.
fn apply_stage(scene: &Scene, nodes: StageNodes, to: usize) {
    let StageNodes {
        message,
        rate_in,
        input_block,
        xor,
        keccak,
        capacity_out,
        rate_out,
        output_hash,
    } = nodes;
    for node in [message, rate_in, input_block, xor, keccak, capacity_out, rate_out, output_hash] {
        let _ = scene.set_focus(node, false);
    }
    let focus = |ids: &[NodeId]| {
        for &id in ids {
            let _ = scene.set_focus(id, true);
        }
    };
    match to {
        0 => focus(&[message]),
        1 => focus(&[input_block, rate_in]),
        2 => focus(&[xor]),
        3 => focus(&[keccak]),
        4 => focus(&[capacity_out, rate_out]),
        5 => focus(&[output_hash]),
        _ => {},
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates every `<svg>` needed by this panel's `.nested-scene-stage`, if they do not already exist. This is the base
/// diagram, then each nested level, starting with the shallowest. The document order is the paint order, and
/// [`add_backdrop_clone`] relies on a nested child following its own parent. Each is inserted before
/// `#sha3-sponge-close`, which therefore stays on top.
///
/// These sizes are the *initial* ones only: each nested scene fits its own `<svg>` to its content as it is built, and a
/// later call here never reverts that.
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
/// Builds the SHA3 Sponge demo's own top-level `Scene` for walk position `stage`: one 136-byte SHA3-256 rate block
/// passing through a single call to `Keccak-f\[1600\]`. Nothing here drags.
///
/// # What this draws
///
/// Row 1 is the sponge's own starting state, split into two named arrays: "Capacity" ([`CAPACITY_LANES`] lanes) and
/// "Rate" ([`RATE_LANES`] lanes). These two subdivisions are butted up against each other with no gap to indicate that
/// they form a single combined state. Both subdivisions start in an initialised state.
///
/// The "Plain Text Message" sits in row 1 above "Input block". This is text being hashed, shown as printable
/// characters. A connector carries this data down into "Input block".
///
/// The "Keccak f(1600)" node on row 2 holds the nested Keccak-f scene. This node only becomes clickable once the step
/// walk reaches it.
///
/// "XOR" shows all-zero lanes, the same "not yet written" convention row 3 below already follows, until `stage` reaches
/// it — see "Stepping through it" below. Only then does it show "Rate" and "Input block"'s own real, elementwise XOR,
/// computed here in plain Rust.
///
/// Row 3 shows the output version of the internal state, still subdivided into "Capacity" and "Rate".
///
/// # Stepping through it
///
/// "Step" is a small, otherwise-meaningless [`STAGE_COUNT`]-value array placed far off-canvas, purely to drive a
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar. That bar needs a data
/// node with exactly as many cells as the walk has stages, and no node already in this diagram happens to hold that
/// count. [`rebuild`] wires the bar itself; this function only draws "Step" and applies `stage`'s own focus, via
/// [`apply_stage`].
///
/// Each stage outlines a different set of nodes, and removes the outline from those nodes no longer in focus. In this
/// particular scene, the walk is never "not started": it opens on "Message". Attempting to step back from there, or
/// restart has no effect.
///
/// "Message" holds [`MESSAGE`], one character per cell, and sits above "Input block", which is initialised to zeros.
/// The first step copies the text to "Input block", with SHA3's padding: the message bytes, then `06`, then zeros,
/// ending in `80`.
///
/// ***IMPORTANT*** The byte order shown in the data nodes is little-endian!
///
/// # Step over or step into
///
/// Every value is computed in plain Rust from the one before it, so stepping over "Keccak f(1600)" gives exactly what
/// stepping through it would have produced. The output "Rate" and "Capacity" show the state `keccak_f` creates once the
/// walk reaches them. The first 256 bits of "Rate" then become the SHA3-256 digest.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-close` (the anchor `create_stage_svgs` creates this panel's
/// `<svg>`s before), or if any library call fails.
fn build_scene(stage: usize) -> Result<(Scene, NodeId), String> {
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
    // "Rate", "XOR" and "Input block" all hold `RATE_LANES` lanes, in 3 columns; "Capacity" and "Output Hash" are
    // narrower still, in 2.
    let hex_rate = |values: Vec<u64>| hex(values).with_layout(GridLayout::Columns(3));
    let hex_narrow = |values: Vec<u64>| hex(values).with_layout(GridLayout::Columns(2));
    // A digest is bytes, not a number: its first byte is a lane's *least* significant, so the node shows each lane
    // little-endian. Read left to right, the four lanes are then the digest as `sha3sum` prints it.
    let hex_digest =
        |values: Vec<u64>| hex_narrow(values).with_byte_order(svg_dom_graph::scene::ByteOrder::LittleEndian);

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    const H_GAP: f64 = 40.0;
    const V_GAP: f64 = 50.0;
    let keccak_size = Size::new(160.0, 70.0);

    // Row 1: the sponge's own starting state, "Capacity" and "Rate", butted up against each other with no gap. They
    // read as one combined state split into two named halves rather than two separate values. The real sponge
    // construction starts from an all-zero state, before any input is absorbed, so both halves start that way too.
    let capacity_in = scene
        .add_named_data_node(Point::new(LEFT_X, TOP_Y), "Capacity", hex_narrow(vec![0; CAPACITY_LANES]))
        .map_err(stringify)?;
    let capacity_in_rect = scene.node_rect(capacity_in).map_err(stringify)?;

    let rate_in_values = vec![0; RATE_LANES];
    let rate_in = scene
        .add_named_data_node(
            Point::new(capacity_in_rect.origin.x + capacity_in_rect.size.width, TOP_Y),
            "Rate",
            hex_rate(rate_in_values.clone()),
        )
        .map_err(stringify)?;
    let rate_in_rect = scene.node_rect(rate_in).map_err(stringify)?;

    // Row 2: "Keccak f(1600)" under "Capacity", "XOR" under "Rate", "Input block" further right of "XOR".
    let row_2_y = capacity_in_rect.origin.y.max(rate_in_rect.origin.y)
        + capacity_in_rect.size.height.max(rate_in_rect.size.height)
        + V_GAP;

    // "XOR" is a plain data node holding the real elementwise XOR of "Rate" and "Input block", not a genuine operator
    // node. `Scene::add_binary_operator_node`'s own result must be exactly one value, which a `RATE_LANES`-lane array
    // is not. Its own size is measured ahead of drawing it, both to centre it under "Rate" and to centre "Keccak
    // f(1600)" on it below.
    //
    // Shows all-zero lanes until `stage` reaches it (see this function's own doc comment, "Stepping through it"). This
    // is the same "not yet written" convention row 3 below already follows. Real values would otherwise appear before
    // the walk ever visits "XOR", reading as already computed when it is not. The whole hash, computed up front in
    // plain Rust: every value this diagram shows, and every state the nested Keccak scenes show, is a piece of this one
    // calculation.
    let run = sha3_256_run(MESSAGE);
    let input_block_values = run.block.to_vec();
    // "Input block" starts out initialised to zeros. Stage `1` adds the padded text to it.
    let shown_block = if stage >= 1 { input_block_values.clone() } else { vec![0; RATE_LANES] };
    let xor_display_values: Vec<u64> = if stage >= 2 {
        rate_in_values
            .iter()
            .zip(input_block_values.iter())
            .map(|(rate, input)| rate ^ input)
            .collect()
    } else {
        vec![0; RATE_LANES]
    };
    // The real result of the whole absorb-and-permute is computed whether or not the walk, or the user, ever steps
    // inside "Keccak f(1600)". The real XOR of "Rate" and "Input block" joins an all-zero "Capacity" as the combined
    // state. `keccak_f` then runs all 24 rounds over it. Row 3 shows its lanes once the walk reaches them. So stepping
    // over the nested scene still yields the correct output, exactly what stepping into it would have ended on.
    let permuted = run.permuted;
    // Zeros until `stage` reaches `from` — the same "not yet written" convention as "XOR" above.
    let from_stage =
        |from: usize, lanes: &[u64]| -> Vec<u64> { if stage >= from { lanes.to_vec() } else { vec![0; lanes.len()] } };
    let xor_content = hex_rate(xor_display_values.clone());
    let xor_size = scene.measure_named_data_node("XOR", &xor_content).map_err(stringify)?;
    let xor_x = rate_in_rect.origin.x + (rate_in_rect.size.width - xor_size.width) / 2.0;
    let xor = scene
        .add_named_data_node(Point::new(xor_x, row_2_y), "XOR", xor_content)
        .map_err(stringify)?;
    let xor_rect = scene.node_rect(xor).map_err(stringify)?;

    // Centred under "Capacity" horizontally, and vertically centred on "XOR". "XOR" is taller than "Keccak f(1600)", so
    // a shared top edge would not read as level.
    //
    // Three fixing points per side. The connector leaving north (from "Capacity") and the one leaving east (from "XOR")
    // both land on the centre point, since this box's own centre lines up with each of theirs. "Rate" sits well east of
    // this box's own centre, since row 3 shares row 1's own left edge — see `row_3_y`'s own comment below. The
    // connector leaving south towards it still lands on the east fixing point, not the centre one.
    //
    // The connector towards "Capacity", directly below, lands on the centre fixing point instead. Row 3 realigning with
    // row 1 leaves this box's own centre no longer off-centre from "Capacity", unlike from "Rate".
    let keccak_x = capacity_in_rect.origin.x + (capacity_in_rect.size.width - keccak_size.width) / 2.0;
    let keccak_y = xor_rect.origin.y + (xor_rect.size.height - keccak_size.height) / 2.0;

    // The real combined 25-lane state flowing into `Keccak-f[1600]`: "Capacity" (always all-zero in this demo) plus
    // whatever "XOR" currently shows. It follows the same "not yet written" convention as "XOR" itself (see this
    // function's own doc comment, "Stepping through it"). It is all-zero before `stage` reaches "XOR", and the real
    // elementwise XOR afterward. `keccak::build_initial_scene`'s own round 0 starts from exactly this.
    let a_bytes_seed: [u64; 25] =
        std::array::from_fn(|lane| if lane < RATE_LANES { xor_display_values[lane] } else { 0 });
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
    // Clickable only from stage `3`, when the walk reaches "Keccak f(1600)" and its input has been assembled. Before
    // that, entering it would show a view whose own input is not yet the real result of the stages above it. Every step
    // rebuilds this whole `Scene`, so enterability needs no undoing when the walk steps back.
    if stage >= 3 {
        scene.make_enterable(keccak).map_err(stringify)?;
    }
    let keccak_rect = scene.node_rect(keccak).map_err(stringify)?;

    let input_block = scene
        .add_named_data_node(
            Point::new(xor_rect.origin.x + xor_rect.size.width + H_GAP, row_2_y),
            "Input block",
            hex_rate(shown_block),
        )
        .map_err(stringify)?;
    let input_block_rect = scene.node_rect(input_block).map_err(stringify)?;

    // "Message": the text `MESSAGE` holds, one character per cell, 16 to a row with a wider gap after the eighth. It
    // sits just above "Input block", centred on it — the node whose padded bytes it becomes. Added at the top of row 1,
    // then moved once its own size is known.
    let message = scene
        .add_named_data_node(
            Point::new(input_block_rect.origin.x, TOP_Y),
            "Plain Text Message",
            DataNodeContent::new(NodeValues::U8(MESSAGE.to_vec()), DataFormat::Ascii)
                .with_layout(GridLayout::Columns(24))
                .with_column_groups(8),
        )
        .map_err(stringify)?;
    let message_rect = scene.node_rect(message).map_err(stringify)?;
    scene
        .move_node(
            message,
            Point::new(
                input_block_rect.origin.x + (input_block_rect.size.width - message_rect.size.width) / 2.0,
                (input_block_rect.origin.y - V_GAP - message_rect.size.height).max(TOP_Y),
            ),
        )
        .map_err(stringify)?;

    // Row 3: "Capacity"/"Rate" again, fed out of "Keccak f(1600)" above; "Output Hash" has its own row below. Each
    // box's own bottom edge is computed independently now that "Keccak f(1600)" and "XOR" no longer share a common top
    // — see `keccak_y`'s own comment above.
    //
    // The elbow connector from "Keccak f(1600)" to "Rate" jogs horizontally halfway between "Keccak f(1600)"'s own
    // bottom edge and row 3's own top edge — see `elbow_route`'s own `mid_y` rule. A plain `V_GAP` below the taller of
    // "Keccak f(1600)"/"XOR" is not always enough on its own. If that halfway point still sits above "XOR"'s own bottom
    // edge, the connector's own horizontal jog crosses straight through it. `ROW_3_CLEARANCE` is the least headroom to
    // leave below "XOR" once that jog is accounted for.
    const ROW_3_CLEARANCE: f64 = 15.0;
    let keccak_bottom = keccak_rect.origin.y + keccak_rect.size.height;
    let xor_bottom = xor_rect.origin.y + xor_rect.size.height;
    let row_3_y = (xor_bottom + V_GAP)
        .max(keccak_bottom + V_GAP)
        .max(2.0 * (xor_bottom + ROW_3_CLEARANCE) - keccak_bottom);

    // "Capacity"/"Rate" are butted together as before, starting at the same `LEFT_X` as row 1's own pair, so the two
    // stay vertically aligned. See `keccak`'s own construction above for how this shapes which of its own south fixing
    // points each connector out of it lands on.
    let capacity_out = scene
        .add_named_data_node(
            Point::new(LEFT_X, row_3_y),
            "Capacity",
            hex_narrow(from_stage(4, &permuted[RATE_LANES..])),
        )
        .map_err(stringify)?;
    let capacity_out_rect = scene.node_rect(capacity_out).map_err(stringify)?;

    // Butted up against "Capacity" with no gap, the same as row 1's own pair — see its own comment above.
    let rate_out = scene
        .add_named_data_node(
            Point::new(capacity_out_rect.origin.x + capacity_out_rect.size.width, row_3_y),
            "Rate",
            hex_rate(from_stage(4, &permuted[..RATE_LANES])),
        )
        .map_err(stringify)?;
    let rate_out_rect = scene.node_rect(rate_out).map_err(stringify)?;

    // Row 4: "Output Hash", centred directly below "Rate" — the first four lanes of it are the digest. Added, then
    // moved once its own width is known.
    let row_4_y = rate_out_rect.origin.y + rate_out_rect.size.height + V_GAP;
    let output_hash = scene
        .add_named_data_node(
            Point::new(rate_out_rect.origin.x, row_4_y),
            "Output Hash",
            hex_digest(from_stage(5, &permuted[..HASH_LANES])),
        )
        .map_err(stringify)?;
    let output_hash_width = scene.node_rect(output_hash).map_err(stringify)?.size.width;
    scene
        .move_node(
            output_hash,
            Point::new(
                rate_out_rect.origin.x + (rate_out_rect.size.width - output_hash_width) / 2.0,
                row_4_y,
            ),
        )
        .map_err(stringify)?;

    // Every vertical connector below is forced South-to-North, and every horizontal one East/West-to-West/East. This
    // matches this codebase's own convention of never relying on the automatic ray-cast when a clean relationship
    // already exists between two boxes (see e.g. `theta::xor_loop::build_scene`'s own `vertical` helper).
    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    scene.add_edge_with(message, input_block, vertical()).map_err(stringify)?;
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
    // "XOR"'s own result feeds back into "Keccak f(1600)", to its own west — the absorbed rate joining "Capacity" as
    // the combined state the real permutation would run over.
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
    scene.add_edge_with(rate_out, output_hash, vertical()).map_err(stringify)?;

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
        message,
        rate_in,
        input_block,
        xor,
        keccak,
        capacity_out,
        rate_out,
        output_hash,
    };
    apply_stage(&scene, stage_nodes, stage);

    // Fit this diagram's own `<svg>` to its content, plus room for its stepping toolbar. The 17-lane "Rate", "XOR",
    // "Input block" and the rest are taller than a fixed size can anticipate. Nested scenes then size themselves
    // against the stage this makes.
    let mut right = 0.0_f64;
    let mut bottom = 0.0_f64;
    for node in [message, input_block, output_hash, rate_out, capacity_out] {
        let rect = scene.node_rect(node).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
        bottom = bottom.max(rect.origin.y + rect.size.height);
    }
    crate::util::fit_nested_size(&scene, "sha3-sponge-diagram", right, bottom, true)?;

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Backdrops `#sha3-sponge-diagram` itself, so entering "Keccak f(1600)" shows this diagram's own current content
    // behind its frame, instead of the page's own plain background. Safe now that `.nested-scene-backdrop` only ever
    // paints while its own source itself has `visibility="hidden"` (see that CSS rule's own doc comment). The clone no
    // longer sits on top of this diagram's own real, interactive view while nothing is nested. So its own zoom/pan
    // buttons stay visibly correct between rebuilds.
    add_backdrop_clone(&document, "sha3-sponge-diagram")?;
    keccak::init_scene(scene.clone(), keccak_child, keccak);

    Ok((scene, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the whole diagram for walk position `stage`, via [`build_scene`], and wires a fresh
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar onto its own freshly
/// drawn "Step" to drive the *next* step.
///
/// A fresh rebuild is the simplest way here. "XOR" and row 3 switch from all-zero to their own real results partway
/// through this walk (see [`build_scene`]'s own doc comment). The nested Keccak scenes are rebuilt from the state each
/// stage supplies. [`apply_stage`] runs again inside [`build_scene`] itself on every rebuild, so each fresh diagram
/// already shows `stage`'s own correct focus from the moment it is drawn.
///
/// `show_selection_toolbar` always resets "Step" to [`Selection::None`] as its own first committed act, regardless of
/// `stage`. So `stage`'s own real position is reapplied immediately afterward — not for "Step"'s own look, which is
/// never seen. It keeps the toolbar's *next* click advancing from the right position, rather than from unstarted every
/// time.
///
/// A fresh `Scene` would otherwise also reset zoom/pan back to `1.0`/`(0, 0)` — jarring, mid-walk, if the previous
/// step's own view had been zoomed or panned in first. So the outgoing `Scene`'s own
/// [`Scene::view`](svg_dom_graph::scene::Scene::view) is read before it is replaced, and carried over onto the fresh
/// one via [`Scene::set_view`](svg_dom_graph::scene::Scene::set_view).
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-close` (the anchor `create_stage_svgs` creates this panel's
/// `<svg>`s before), or if any library call fails.
fn rebuild(stage: usize) -> Result<(), String> {
    let view = SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));

    let (scene, step) = build_scene(stage)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(step, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            // This walk is never "not started": the message is in focus from the first draw, so a step back from it, or
            // a restart, lands on it again.
            let _ = rebuild(transition.to.unwrap_or(0));
        })
        .map_err(stringify)?;
    scene.set_selection(step, Selection::Cell(stage)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the SHA3 Sponge demo at its first stage, with "Message" in focus — see [`build_scene`]'s own doc comment for
/// what it draws — and wires its own close button.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha3-sponge-close`, or if any library call fails.
pub(crate) fn build_sha3_sponge_demo() -> Result<(), String> {
    rebuild(0)?;
    wire_sha3_sponge_controls(crate::util::document()?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#sha3-sponge-close` — the nested view's own &times; close button — to whichever of "Keccak f(1600)"'s own
/// descendants is currently entered, via [`keccak::exit_if_focused`]. Wired once, here, not from inside
/// [`rebuild`]: unlike the `<svg>` content `rebuild` redraws from scratch on every step, `#sha3-sponge-close` is
/// a plain, static `index.html` element that would otherwise pick up one more duplicate listener per step.
/// [`keccak::exit_if_focused`] always reads `keccak`'s own latest thread-local state at click time, regardless of how
/// many rebuilds happened since this listener was attached.
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
