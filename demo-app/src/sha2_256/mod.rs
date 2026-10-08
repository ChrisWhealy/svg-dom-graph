//! `panel-sha2-256` / `#sha2-256-diagram`: SHA-256 of one short message, drawn as the standard describes it. See
//! [`build_scene`]'s own doc comment for what the diagram shows.
//!
//! # What is real
//!
//! All of it. [`algorithm::trace`] hashes [`walk::MESSAGE`] in plain Rust, and every value on screen is a piece of that
//! one calculation: the padded block, the 64-word message schedule, the 64 rounds of compression, and the digest. The
//! constants are computed, not pasted in. `algorithm`'s own tests check them, and the digests of several messages,
//! against the published values.
//!
//! # What is not covered
//!
//! A message of more than one block, which needs the blocks chained one into the next, and SHA-256's relatives.

mod algorithm;
mod walk;

use crate::util::{ensure_svg_in, fit_nested_size, stringify};
use algorithm::{INITIAL_HASH, ROUND_CONSTANTS, Trace};
use std::cell::RefCell;
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, SceneTitleOptions, Selection,
        SelectionStride, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use walk::{MESSAGE, Ring, STEPS, shown};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

thread_local! {
    // Keeps the current `Scene` alive for the page's lifetime. Every listener the toolbars install holds only a `Weak`
    // reference back to it, so without a strong handle kept here it would drop the moment [`rebuild`] returns.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// The id of the `<svg>` this demo draws into, and of the `<div>` in the panel that hosts it.
const DIAGRAM_ID: &str = "sha2-256-diagram";
const STAGE_ID: &str = "sha2-256-diagram-stage";

/// The space left between one node and the next, and the margin the first row and column leave.
const LEFT: f64 = 20.0;
const TOP: f64 = 50.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 70.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's own content: `values` as `u32` words, in hexadecimal, `columns` to a row. A word is four bytes, so
/// hexadecimal reads as the bytes of a big-endian word, the way the standard writes them.
fn words(values: &[u32], columns: usize) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values.to_vec()), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(columns))
}

/// A forced south-to-north connector, for a flow that runs straight down the diagram.
fn downward() -> ConnectorOptions {
    ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a named data node at `(x, y)` and returns it with its measured rectangle, so the next node can be placed
/// relative to it.
fn add(scene: &Scene, x: f64, y: f64, name: &str, content: DataNodeContent) -> Result<(NodeId, Rect), String> {
    let id = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
    let rect = scene.node_rect(id).map_err(stringify)?;
    Ok((id, rect))
}

/// The lowest edge of any of `rects`.
fn bottom_of(rects: &[Rect]) -> f64 {
    rects.iter().map(|r| r.origin.y + r.size.height).fold(0.0, f64::max)
}

/// Adds a one-word node, named `name`, showing `value`.
fn word(scene: &Scene, x: f64, y: f64, name: &str, value: u32) -> Result<(NodeId, Rect), String> {
    add(scene, x, y, name, words(&[value], 1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rings or marks the cells the walk is on, per `shown`: the whole node for a step that names one, the schedule word
/// being built or used, the four words it reads, and the round constant in use.
fn apply_marks(scene: &Scene, shown: &walk::Shown, nodes: &Marked) -> Result<(), String> {
    for (ring, node) in [
        (Ring::Message, nodes.message),
        (Ring::Block, nodes.block),
        (Ring::Digest, nodes.digest),
    ] {
        scene.set_focus(node, shown.ring == Some(ring)).map_err(stringify)?;
    }
    if let Some(focus) = shown.schedule_focus {
        let cell = nodes.schedule_content.natural_selection(focus).unwrap_or(Selection::None);
        scene.set_selection(nodes.schedule, cell).map_err(stringify)?;
        scene
            .set_secondary_selection(nodes.schedule, &shown.schedule_secondary)
            .map_err(stringify)?;
    }
    if let Some(focus) = shown.k_focus {
        let cell = nodes.k_content.natural_selection(focus).unwrap_or(Selection::None);
        scene.set_selection(nodes.k, cell).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes [`apply_marks`] reaches, with the content of the two tables it addresses by cell.
struct Marked {
    message: NodeId,
    block: NodeId,
    digest: NodeId,
    schedule: NodeId,
    schedule_content: DataNodeContent,
    k: NodeId,
    k_content: DataNodeContent,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the whole diagram for walk position `position`, from scratch, and returns the "Step" node that drives its
/// toolbar.
///
/// # What this draws
///
/// Row 1 is the input. "Plain Text Message" is the text being hashed, and "Message Block" is that text as 16 big-endian
/// words, followed by the `0x80` terminator byte, with the message's length in bits in the last word.
///
/// Row 2 is two tables. "Message Schedule" holds 64 words. The first 16 are the block, and each of the other 48 is
/// `ms[n-16] + sigma0(ms[n-15]) + ms[n-7] + sigma1(ms[n-2])`, ignoring overflow. While a word is being built, that word
/// is marked, and the four words it reads are marked as secondary. "Round Constants K" holds the first 32 bits of the
/// fractional part of the cube roots of the first 64 primes.
///
/// Rows 3 to 6 are one round of compression. "Working variables" holds `a..h`, running from "Initial hash H", the same
/// fractions of the square roots of the first 8 primes. Below it sit the five terms of `temp1 = h + bigSigma1(e) +
/// choice(e, f, g) + K[i] + W[i]` and the two of `temp2 = bigSigma0(a) + majority(a, b, c)`. The next working variables
/// are `h=g, g=f, f=e, e=d+temp1, d=c, c=b, b=a, a=temp1+temp2`.
///
/// After 64 rounds, "Digest" is each working variable added to the initial hash word it began as. Without that last
/// addition the result would not be a hash of the block.
///
/// # Stepping through it
///
/// "Step" is a small array of one cell per walk position, placed far off-canvas, purely to drive a selection toolbar.
/// See [`walk`] for what each position shows. Anything the walk has not reached yet is zeros.
///
/// # Errors
///
/// Returns `Err` if the message is too long for one block, or if any library call fails.
fn build_scene(position: usize) -> Result<(Scene, NodeId), String> {
    let trace: Trace = algorithm::trace(MESSAGE).ok_or("the message does not fit one block")?;
    let shown = shown(&trace, position);

    let document = crate::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1900.0, 1700.0))?;
    crate::util::required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("SHA-256", SceneTitleOptions::default())
        .map_err(stringify)?;

    // Row 1: the message, and the block it becomes.
    let (message, message_rect) = add(
        &scene,
        LEFT,
        TOP,
        "Plain Text Message",
        DataNodeContent::new(NodeValues::U8(MESSAGE.to_vec()), DataFormat::PlainText),
    )?;
    let (block, block_rect) = add(
        &scene,
        message_rect.origin.x + message_rect.size.width + H_GAP,
        TOP,
        "Message Block",
        words(&shown.block, 4),
    )?;
    scene
        .add_edge_with(
            message,
            block,
            ConnectorOptions::default()
                .with_from_side(Some(Side::East))
                .with_to_side(Some(Side::West)),
        )
        .map_err(stringify)?;

    // Row 2: the message schedule, and the round constants beside it.
    let row_2 = bottom_of(&[message_rect, block_rect]) + V_GAP;
    let schedule_content = words(&shown.schedule, 8);
    let (schedule, schedule_rect) = add(&scene, LEFT, row_2, "Message Schedule", schedule_content.clone())?;
    let k_content = words(&ROUND_CONSTANTS, 8);
    let (k, k_rect) = add(
        &scene,
        schedule_rect.origin.x + schedule_rect.size.width + H_GAP,
        row_2,
        "Round Constants K",
        k_content.clone(),
    )?;
    scene.add_edge_with(block, schedule, downward()).map_err(stringify)?;

    // Row 3: the working variables, the digest they feed, and the initial hash the digest adds them to.
    let row_3 = bottom_of(&[schedule_rect, k_rect]) + V_GAP;
    let (working, working_rect) = add(&scene, LEFT, row_3, "Working variables a..h", words(&shown.working, 4))?;
    let (digest, digest_rect) = add(
        &scene,
        working_rect.origin.x + working_rect.size.width + H_GAP,
        row_3,
        "Digest",
        words(&shown.digest, 4),
    )?;
    let (initial, _) = add(
        &scene,
        digest_rect.origin.x + digest_rect.size.width + H_GAP,
        row_3,
        "Initial hash H",
        words(&INITIAL_HASH, 4),
    )?;
    scene
        .add_edge_with(
            working,
            digest,
            ConnectorOptions::default()
                .with_from_side(Some(Side::East))
                .with_to_side(Some(Side::West)),
        )
        .map_err(stringify)?;
    scene
        .add_edge_with(
            initial,
            digest,
            ConnectorOptions::default()
                .with_from_side(Some(Side::West))
                .with_to_side(Some(Side::East)),
        )
        .map_err(stringify)?;

    // Row 4: the five terms of temp1 and the two of temp2, each one word. Values are zero until a round is running.
    let round = shown.round;
    let value = |f: fn(&algorithm::Round) -> u32| round.as_ref().map_or(0, f);
    let index = |i: &str| shown.k_focus.map_or_else(|| format!("{i}[i]"), |n| format!("{i}[{n}]"));
    let row_4 = working_rect.origin.y + working_rect.size.height + V_GAP;
    let terms: [(&str, u32); 7] = [
        ("bigSigma1(e)", value(|r| r.big_sigma1)),
        ("choice(e,f,g)", value(|r| r.choice)),
        ("h", round.as_ref().map_or(0, |r| r.before[7])),
        (&index("K"), shown.k_focus.map_or(0, |n| ROUND_CONSTANTS[n])),
        (&index("W"), shown.k_focus.map_or(0, |n| shown.schedule[n])),
        ("bigSigma0(a)", value(|r| r.big_sigma0)),
        ("majority(a,b,c)", value(|r| r.majority)),
    ];
    let mut x = LEFT;
    let mut term_nodes: Vec<(NodeId, Rect)> = Vec::new();
    for (name, v) in &terms {
        let (id, rect) = word(&scene, x, row_4, name, *v)?;
        x = rect.origin.x + rect.size.width + 40.0;
        term_nodes.push((id, rect));
    }

    // Row 5: temp1 under its five terms, temp2 under its two.
    let row_5 = bottom_of(&term_nodes.iter().map(|&(_, r)| r).collect::<Vec<_>>()) + V_GAP;
    let centre_of = |nodes: &[(NodeId, Rect)]| {
        let (first, last) = (nodes[0].1, nodes[nodes.len() - 1].1);
        (first.origin.x + last.origin.x + last.size.width) / 2.0
    };
    let place = |name: &str, v: u32, centre: f64| -> Result<(NodeId, Rect), String> {
        let (id, rect) = word(&scene, 0.0, row_5, name, v)?;
        let moved = Point::new(centre - rect.size.width / 2.0, row_5);
        scene.move_node(id, moved).map_err(stringify)?;
        Ok((id, scene.node_rect(id).map_err(stringify)?))
    };
    let (temp1, temp1_rect) = place("temp1", value(|r| r.temp1), centre_of(&term_nodes[..5]))?;
    let (temp2, temp2_rect) = place("temp2", value(|r| r.temp2), centre_of(&term_nodes[5..]))?;

    // Row 6: the working variables the round produces.
    let row_6 = bottom_of(&[temp1_rect, temp2_rect]) + V_GAP;
    let next = round.as_ref().map_or([0; 8], |r| r.after);
    let (next_working, next_rect) = add(&scene, LEFT, row_6, "Next working variables a..h", words(&next, 4))?;

    // The flows: the working variables into the terms that read them, the terms into the temps, the temps into the next
    // working variables, and the tables into the two terms taken from them.
    for &i in &[0, 1, 2, 5, 6] {
        scene.add_edge_with(working, term_nodes[i].0, downward()).map_err(stringify)?;
    }
    for node in &term_nodes[..5] {
        scene.add_edge_with(node.0, temp1, downward()).map_err(stringify)?;
    }
    for node in &term_nodes[5..] {
        scene.add_edge_with(node.0, temp2, downward()).map_err(stringify)?;
    }
    scene.add_edge_with(temp1, next_working, downward()).map_err(stringify)?;
    scene.add_edge_with(temp2, next_working, downward()).map_err(stringify)?;

    apply_marks(
        &scene,
        &shown,
        &Marked {
            message,
            block,
            digest,
            schedule,
            schedule_content,
            k,
            k_content,
        },
    )?;

    // "Step": see this function's own doc comment, "Stepping through it".
    let step_values: Vec<u8> = (1..=STEPS as u8).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U8(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    // Fit this diagram's own `<svg>` to its content, plus room for its stepping toolbar.
    let right = [message_rect, block_rect, schedule_rect, k_rect, next_rect]
        .iter()
        .chain(term_nodes.iter().map(|(_, r)| r))
        .map(|r| r.origin.x + r.size.width)
        .fold(0.0, f64::max);
    let initial_rect = scene.node_rect(initial).map_err(stringify)?;
    fit_nested_size(
        &scene,
        DIAGRAM_ID,
        right.max(initial_rect.origin.x + initial_rect.size.width),
        bottom_of(&[next_rect]),
        true,
    )?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the whole diagram for walk position `position`, via [`build_scene`], and wires a fresh selection toolbar
/// onto its own freshly drawn "Step" to drive the *next* step.
///
/// A fresh rebuild is the simplest way here. A node cannot change the value it shows, and nearly every node does as the
/// walk advances. `show_selection_toolbar` always resets "Step" to unstarted as its first committed act, so `position`
/// is reapplied immediately afterward. That keeps the toolbar's *next* click advancing from the right place. A fresh
/// `Scene` would also reset zoom and pan, so the outgoing one's view is carried over onto it.
///
/// # Errors
///
/// Returns `Err` if the panel's own stage is missing, or if any library call fails.
fn rebuild(position: usize) -> Result<(), String> {
    let view = SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));

    let (scene, step) = build_scene(position)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(
            step,
            SelectionToolbarOptions::default().with_stride(SelectionStride::new(8, "8")),
            move |_scene, _node, transition| {
                // This walk is never "not started": the message is in focus from the first draw, so a step back from it,
                // or a restart, lands on it again.
                let _ = rebuild(transition.to.unwrap_or(0));
            },
        )
        .map_err(stringify)?;
    scene.set_selection(step, Selection::Cell(position)).map_err(stringify)?;

    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the SHA-256 demo at its first step, with the message in focus.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha2-256-diagram-stage`, or if any library call fails.
pub(crate) fn build_sha2_256_demo() -> Result<(), String> {
    rebuild(0)
}
