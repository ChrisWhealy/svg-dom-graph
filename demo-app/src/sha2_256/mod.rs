//! `panel-sha2-256` / `#sha2-256-diagram`: SHA-256 of one short message, drawn as the standard describes it. See
//! [`build_scene`]'s own doc comment for what the diagram shows. The scene is large, so it runs in a window of its own.
//! The gallery's panel keeps the description and opens that window: see [`build_sha2_256_demo`].
//!
//! # Two halves
//!
//! The hash has two halves, and the diagram shows only the half in progress. While the message schedule is being built,
//! only the message, its block and the schedule are on screen, with the sums that make each new schedule word. Once it
//! is complete, the round constants, the initial hash, the working variables and the round's own terms appear beside
//! it. A node cannot be removed once drawn, so crossing from one half to the other draws the diagram again. Every step
//! inside a half only writes new values and marks into the nodes already there, via [`apply`]. Which values and marks a
//! step shows comes from [`walk::shown`], a pure function of the position alone, so stepping backwards is the same
//! write as stepping forwards. Zoom and pan are kept across the redraw, and so is the keyboard focus on the button that
//! was pressed.
//!
//! # What is real
//!
//! All of it. [`algorithm::trace`] hashes [`walk::MESSAGE`] in plain Rust, and every value on screen is a piece of that
//! one calculation: the padded block, the 64-word message schedule, the 64 rounds of compression, and the digest. The
//! SHA-256 constants are fixed tables. Their values are recomputed independently, from the primes, and verified by
//! `algorithm`'s unit tests, which also check the digests of several messages against the published values.
//!
//! # What is not covered
//!
//! A message of more than one block, which needs the blocks chained one into the next, and SHA-256's relatives.

mod algorithm;
mod walk;

use crate::util::{ensure_svg_in, fit_nested_size, stringify};
use algorithm::{INITIAL_HASH, ROUND_CONSTANTS, Trace};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionStride, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use walk::{MESSAGE, Ring, STEPS, Stage, shown, stage};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// What the toolbar's callback needs to find again on every step: the scene, its nodes, and the hash being walked.
struct Demo {
    scene: Scene,
    nodes: Nodes,
    /// The "Step" node that drives the selection toolbar.
    step: NodeId,
    trace: Rc<Trace>,
}

thread_local! {
    // Keeps the `Scene` alive for the page's lifetime. Every listener the toolbars install holds only a `Weak`
    // reference back to it, so without a strong handle kept here it would drop the moment [`build`] returns.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
}

/// The id of the `<svg>` this demo draws into, and of the `<div>` in the window that hosts it.
const DIAGRAM_ID: &str = "sha2-256-diagram";
const STAGE_ID: &str = "sha2-256-diagram-stage";
/// The id of the hidden `.demo-error` paragraph in the window that shows a failure in a step.
const ERROR_ID: &str = "sha2-256-window-error";

/// The first column's left edge, and the first row's top. The top leaves room for the scene's title and, below it, the
/// stepping toolbar, which sits at the top of the diagram because the diagram is tall and narrow.
const LEFT: f64 = 20.0;
const TOP: f64 = 100.0;
/// How far the stepping toolbar sits from the top edge: below the scene's own title.
const TOOLBAR_MARGIN: f64 = 52.0;
/// The space between one column and the next, and between one node and the next below it in a column.
const H_GAP: f64 = 70.0;
const V_GAP: f64 = 36.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's own content: `values` as `u32` words in binary, one to a row. Rotations and shifts are the heart of
/// SHA-256, and bits make them visible. A word is four bytes, shown big-endian, the way the standard writes them.
fn words(values: &[u32]) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values.to_vec()), DataFormat::Binary).with_layout(GridLayout::Columns(1))
}

/// `content` with its eight words labelled `a` to `h`, as the standard names the working variables.
fn lettered(content: DataNodeContent) -> DataNodeContent {
    content.with_labelling_style(LabellingStyle::Alphabetic)
}

/// A forced south-to-north connector, for a flow that runs straight down a column.
fn downward() -> ConnectorOptions {
    ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North))
}

/// A forced east-to-west connector, for a flow that runs across to the next column.
fn rightward() -> ConnectorOptions {
    ConnectorOptions::default()
        .with_from_side(Some(Side::East))
        .with_to_side(Some(Side::West))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector from the east side of `from` to the west side of `to`, which meets `to` at the height of `from`'s own
/// middle. So it runs straight across when `from` sits beside `to`, and not down to `to`'s own middle, which would be
/// far below when `to` is a tall column. `from` must lie within `to`'s own height.
fn level_with(from: &Rect, to: &Rect) -> ConnectorOptions {
    let middle = from.origin.y + from.size.height / 2.0;
    rightward().with_to_position(Some(((middle - to.origin.y) / to.size.height).clamp(0.0, 1.0)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a named data node at `(x, y)` and returns it with its measured rectangle, so the next node can be placed
/// relative to it.
fn add(scene: &Scene, x: f64, y: f64, name: &str, content: DataNodeContent) -> Result<(NodeId, Rect), String> {
    let id = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
    let rect = scene.node_rect(id).map_err(stringify)?;
    Ok((id, rect))
}

/// Adds `items` as one column at `x`, the first at `y`, each [`V_GAP`] below the one before. Every node is centred on
/// the column's own middle, the widest node's, so a connector between two of them runs straight down. Without that, a
/// node with a short name is narrower than one with a long name, and a connector between them has a short elbow.
/// Returns every node with its rectangle, in order.
fn column(scene: &Scene, x: f64, y: f64, items: Vec<(&str, DataNodeContent)>) -> Result<Vec<(NodeId, Rect)>, String> {
    let mut y = y;
    let mut nodes = Vec::with_capacity(items.len());
    for (name, content) in items {
        let (id, rect) = add(scene, x, y, name, content)?;
        y = rect.origin.y + rect.size.height + V_GAP;
        nodes.push((id, rect));
    }
    let widest = nodes.iter().map(|(_, r)| r.size.width).fold(0.0, f64::max);
    for (id, rect) in &mut nodes {
        let moved = Point::new(x + (widest - rect.size.width) / 2.0, rect.origin.y);
        scene.move_node(*id, moved).map_err(stringify)?;
        *rect = scene.node_rect(*id).map_err(stringify)?;
    }
    Ok(nodes)
}

/// The right edge of the rightmost of `rects`, and the lowest edge of any of them.
fn extent(rects: &[Rect]) -> (f64, f64) {
    rects.iter().fold((0.0, 0.0), |(right, bottom), r| {
        (
            f64::max(right, r.origin.x + r.size.width),
            f64::max(bottom, r.origin.y + r.size.height),
        )
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes both halves draw: the message, the block it becomes, and the schedule built from it.
struct Common {
    message: NodeId,
    block: NodeId,
    schedule: NodeId,
    schedule_content: DataNodeContent,
}

/// What the first half adds below the block: the four schedule words word `n` is built from, the two sigma results, and
/// their sum, in the order they are drawn: `ms[n-16]`, `ms[n-15]`, `sigma0`, `ms[n-7]`, `ms[n-2]`, `sigma1`, `ms[n]`.
struct Expansion {
    terms: [NodeId; 7],
}

/// What the second half adds, in columns three and four.
struct Compression {
    working: NodeId,
    /// The five terms of `temp1`, before it: `bigSigma1(e)`, `choice(e,f,g)`, `h`, `K[i]` and `W[i]`.
    temp1_terms: [NodeId; 5],
    temp1: NodeId,
    /// The two terms of `temp2`, before it: `bigSigma0(a)` and `majority(a,b,c)`.
    temp2_terms: [NodeId; 2],
    temp2: NodeId,
    next: NodeId,
    digest: NodeId,
    /// The digest as a hexadecimal hash value, below it.
    hash: NodeId,
    k: NodeId,
    k_content: DataNodeContent,
}

/// The nodes of whichever half is drawn.
enum Half {
    Expansion(Expansion),
    Compression(Compression),
}

/// Every node the walk writes to after the diagram is drawn.
struct Nodes {
    common: Common,
    half: Half,
}

impl Nodes {
    /// Which half these nodes are.
    fn stage(&self) -> Stage {
        match self.half {
            Half::Expansion(_) => Stage::Expansion,
            Half::Compression(_) => Stage::Compression,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes everything the walk shows at one position into the nodes already drawn: each node's values, then the marks.
///
/// Every write is absolute. Nothing is read back, and nothing depends on the position the walk came from, so a step
/// back and a step forward are the same operation. A node the walk has not reached shows zeros, and a mark it has not
/// reached is cleared.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn apply(scene: &Scene, nodes: &Nodes, shown: &walk::Shown) -> Result<(), String> {
    let set =
        |node: NodeId, values: &[u32]| scene.set_data_values(node, NodeValues::U32(values.to_vec())).map_err(stringify);
    set(nodes.common.block, &shown.block)?;
    set(nodes.common.schedule, &shown.schedule)?;

    match &nodes.half {
        Half::Expansion(expansion) => {
            let values = shown
                .expansion
                .map_or([0; 7], |e| [e.w16, e.w15, e.sigma0, e.w7, e.w2, e.sigma1, e.sum]);
            for (node, value) in expansion.terms.iter().zip(values) {
                set(*node, &[value])?;
            }
        },
        Half::Compression(c) => {
            let round = shown.round.as_ref();
            let from_round = |f: fn(&algorithm::Round) -> u32| round.map_or(0, f);
            let index = shown.round_index;
            let temp1_terms: [u32; 5] = [
                from_round(|r| r.big_sigma1),
                from_round(|r| r.choice),
                round.map_or(0, |r| r.before[7]),
                index.map_or(0, |i| ROUND_CONSTANTS[i]),
                index.map_or(0, |i| shown.schedule[i]),
            ];
            set(c.working, &shown.working)?;
            for (node, value) in c.temp1_terms.iter().zip(temp1_terms) {
                set(*node, &[value])?;
            }
            set(c.temp1, &[from_round(|r| r.temp1)])?;
            set(c.temp2_terms[0], &[from_round(|r| r.big_sigma0)])?;
            set(c.temp2_terms[1], &[from_round(|r| r.majority)])?;
            set(c.temp2, &[from_round(|r| r.temp2)])?;
            set(c.next, &round.map_or([0; 8], |r| r.after))?;
            set(c.digest, &shown.digest)?;
            scene
                .set_data_values(c.hash, NodeValues::U8(shown.hash.clone().into_bytes()))
                .map_err(stringify)?;
        },
    }
    apply_marks(scene, nodes, shown)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rings or marks the cells the walk is on, per `shown`: the whole node for a step that names one, the schedule word
/// being built or used, the four words it reads, and the round constant in use. Whatever `shown` does not name is
/// cleared. The digest and the round constants exist only in the second half. Then the cells with no value yet are
/// marked, by [`apply_unreached`].
fn apply_marks(scene: &Scene, nodes: &Nodes, shown: &walk::Shown) -> Result<(), String> {
    let (digest, hash) = match &nodes.half {
        Half::Compression(c) => (Some(c.digest), Some(c.hash)),
        Half::Expansion(_) => (None, None),
    };
    for (ring, node) in [
        (Ring::Message, Some(nodes.common.message)),
        (Ring::Block, Some(nodes.common.block)),
        (Ring::Digest, digest),
        (Ring::Hash, hash),
    ] {
        if let Some(node) = node {
            scene.set_focus(node, shown.ring == Some(ring)).map_err(stringify)?;
        }
    }
    let cell = |content: &DataNodeContent, focus: Option<usize>| {
        focus.and_then(|i| content.natural_selection(i)).unwrap_or(Selection::None)
    };
    scene
        .set_selection(
            nodes.common.schedule,
            cell(&nodes.common.schedule_content, shown.schedule_focus),
        )
        .map_err(stringify)?;
    scene
        .set_secondary_selection(nodes.common.schedule, &shown.schedule_secondary)
        .map_err(stringify)?;
    if let Half::Compression(c) = &nodes.half {
        scene.set_selection(c.k, cell(&c.k_content, shown.k_focus)).map_err(stringify)?;
    }
    apply_unreached(scene, nodes, shown)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks the cells the walk has not reached yet, so a placeholder zero cannot be taken for a computed one. The message
/// block's own padding words, and a schedule word that legitimately comes out as zero, are real values. They are not
/// marked.
fn apply_unreached(scene: &Scene, nodes: &Nodes, shown: &walk::Shown) -> Result<(), String> {
    let mark = |node: NodeId, cells: &[usize]| scene.set_unreached_cells(node, cells).map_err(stringify);
    // `count` cells from the first, if `unreached`, and none otherwise.
    let first =
        |count: usize, unreached: bool| -> Vec<usize> { if unreached { (0..count).collect() } else { Vec::new() } };

    mark(nodes.common.block, &first(algorithm::BLOCK_WORDS, shown.block_unreached))?;
    mark(nodes.common.schedule, &shown.schedule_unreached)?;
    match &nodes.half {
        Half::Expansion(expansion) => {
            for term in &expansion.terms {
                mark(*term, &first(1, shown.terms_unreached))?;
            }
        },
        Half::Compression(c) => {
            mark(c.digest, &first(8, shown.digest_unreached))?;
            mark(c.hash, &first(1, shown.hash_unreached))?;
        },
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the diagram for one half of the hash, with nothing reached yet, and returns its nodes and the "Step" node that
/// drives its toolbar.
///
/// # What this draws
///
/// The diagram is a row of columns, read left to right and then down each one.
///
/// **Column 1** is the input. "Plain Text Message" is the text being hashed. Below it, "Message Block" is that text as
/// 16 big-endian words, followed by the `0x80` terminator byte, with the message's length in bits in the last word.
///
/// **Column 2** is "Message Schedule", 64 words in one column. The first 16 are the block, and each of the other 48 is
/// `ms[n-16] + sigma0(ms[n-15]) + ms[n-7] + sigma1(ms[n-2])`, ignoring overflow.
///
/// While the schedule is being built, column 1 continues below the block with the pieces of that sum: the four words it
/// reads, the two sigma results, and the total, which is the new word. The four words are marked in the schedule as
/// secondary, and the new one is marked as the word in focus.
///
/// Once the schedule is complete, column 1 stays, and two more columns appear. **Column 3** is one round of
/// compression, read downward: "Initial hash H" and "Working variables a..h", the five terms of `temp1 = h +
/// bigSigma1(e) + choice(e, f, g) + K[i] + W[i]`, the two of `temp2 = bigSigma0(a) + majority(a, b, c)`, and the next
/// working variables, `h=g, g=f, f=e, e=d+temp1, d=c, c=b, b=a, a=temp1+temp2`. After 64 rounds, "Digest" is each
/// working variable added to the initial hash word it began as. Without that last addition the result would not be a
/// hash of the block. A last step writes the digest out below it as the hexadecimal hash value. **Column 4** is "Round
/// Constants K", 64 words in one column.
///
/// "Round Constants K" starts at the same height as the schedule, and both use one row per word. So round `i` marks row
/// `i` in both, and the two rows sit on one horizontal line either side of the round between them.
///
/// # Stepping through it
///
/// "Step" is a small array of one cell per walk position, placed far off-canvas, purely to drive a selection toolbar.
/// It sits at the top of the diagram, since the diagram is tall. See [`walk`] for what each position shows, and
/// [`apply`] for how a position is written into the diagram. Anything the walk has not reached yet is zeros.
///
/// # Errors
///
/// Returns `Err` if the window's own stage is missing, or if any library call fails.
fn build_scene(stage: Stage) -> Result<(Scene, Nodes, NodeId), String> {
    let document = crate::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1900.0, 1700.0))?;
    crate::util::required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("SHA-256", SceneTitleOptions::default())
        .map_err(stringify)?;

    // Column 1: the message, and the block below it.
    let (message, message_rect) = add(
        &scene,
        LEFT,
        TOP,
        "Plain Text Message",
        DataNodeContent::new(NodeValues::U8(MESSAGE.to_vec()), DataFormat::PlainText),
    )?;
    let (block, block_rect) = add(
        &scene,
        LEFT,
        message_rect.origin.y + message_rect.size.height + V_GAP,
        "Message Block",
        words(&[0; algorithm::BLOCK_WORDS]),
    )?;
    scene.add_edge_with(message, block, downward()).map_err(stringify)?;

    // Column 2: the message schedule, from the block's own height.
    let column_2 = message_rect.origin.x + message_rect.size.width.max(block_rect.size.width) + H_GAP;
    let row_2 = block_rect.origin.y;
    let schedule_content = words(&[0; algorithm::SCHEDULE_WORDS]);
    let (schedule, schedule_rect) = add(&scene, column_2, row_2, "Message Schedule", schedule_content.clone())?;
    scene
        .add_edge_with(block, schedule, level_with(&block_rect, &schedule_rect))
        .map_err(stringify)?;

    let common = Common {
        message,
        block,
        schedule,
        schedule_content,
    };
    let mut rects = vec![message_rect, block_rect, schedule_rect];

    let half = match stage {
        Stage::Expansion => {
            // Column 1, continued: the pieces of the sum that builds the next schedule word.
            let items = [
                "ms[n-16]",
                "ms[n-15]",
                "sigma0(ms[n-15])",
                "ms[n-7]",
                "ms[n-2]",
                "sigma1(ms[n-2])",
                "ms[n] = sum",
            ]
            .map(|name| (name, words(&[0])))
            .to_vec();
            let terms = column(&scene, LEFT, block_rect.origin.y + block_rect.size.height + 2.0 * V_GAP, items)?;
            // Each sigma is applied to the word just above it, and the sum is the new schedule word.
            scene.add_edge_with(terms[1].0, terms[2].0, downward()).map_err(stringify)?;
            scene.add_edge_with(terms[4].0, terms[5].0, downward()).map_err(stringify)?;
            scene
                .add_edge_with(terms[6].0, schedule, level_with(&terms[6].1, &schedule_rect))
                .map_err(stringify)?;
            rects.extend(terms.iter().map(|&(_, r)| r));
            Half::Expansion(Expansion {
                terms: std::array::from_fn(|i| terms[i].0),
            })
        },
        Stage::Compression => {
            // Column 3: one round, read downward.
            let column_3 = schedule_rect.origin.x + schedule_rect.size.width + H_GAP;
            let items = vec![
                ("Initial hash H", words(&INITIAL_HASH)),
                ("Working variables a..h", lettered(words(&[0; 8]))),
                ("bigSigma1(e)", words(&[0])),
                ("choice(e,f,g)", words(&[0])),
                ("h", words(&[0])),
                ("K[i]", words(&[0])),
                ("W[i]", words(&[0])),
                ("temp1", words(&[0])),
                ("bigSigma0(a)", words(&[0])),
                ("majority(a,b,c)", words(&[0])),
                ("temp2", words(&[0])),
                ("Next working variables a..h", lettered(words(&[0; 8]))),
                ("Digest = H + working", words(&[0; 8])),
            ];
            let c = column(&scene, column_3, row_2, items)?;
            let link = |from: usize, to: usize| scene.add_edge_with(c[from].0, c[to].0, downward()).map_err(stringify);
            // The initial hash starts the working variables, which lead into the five terms that sum to `temp1`. The
            // two terms of `temp2` follow. `temp1` and `temp2` both feed the next working variables, and those the
            // digest. The working variables also feed the terms of `temp2`, and `temp1` feeds the next working
            // variables, but a connector over the nodes between would cross them. Those relationships are named in
            // the nodes instead.
            for (from, to) in [
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 7),
                (8, 9),
                (9, 10),
                (10, 11),
                (11, 12),
            ] {
                link(from, to)?;
            }
            // `temp1` reaches the next working variables around the left of everything between, in the gap after column
            // 2, and enters from outside their west edge.
            scene
                .add_edge_with(
                    c[7].0,
                    c[11].0,
                    ConnectorOptions::default()
                        .with_from_side(Some(Side::West))
                        .with_to_side(Some(Side::West)),
                )
                .map_err(stringify)?;
            rects.extend(c.iter().map(|&(_, r)| r));

            // Below the digest: the digest as a hexadecimal hash value. It is wider than the column, so it hangs below
            // the columns either side, which end above it, rather than widening every node in the column.
            let digest_rect = c[12].1;
            let (hash, hash_rect) = add(
                &scene,
                0.0,
                digest_rect.origin.y + digest_rect.size.height + V_GAP,
                "SHA-256 hash",
                DataNodeContent::new(NodeValues::U8(vec![b'0'; 64]), DataFormat::PlainText),
            )?;
            let centre = digest_rect.origin.x + digest_rect.size.width / 2.0;
            scene
                .move_node(hash, Point::new(centre - hash_rect.size.width / 2.0, hash_rect.origin.y))
                .map_err(stringify)?;
            scene.add_edge_with(c[12].0, hash, downward()).map_err(stringify)?;
            rects.push(scene.node_rect(hash).map_err(stringify)?);

            // Column 4: the round constants, from the schedule's own height, so that row `i` of each is on one line.
            let (column_3_right, _) = extent(&c.iter().map(|&(_, r)| r).collect::<Vec<_>>());
            let k_content = words(&ROUND_CONSTANTS);
            let (k, k_rect) = add(&scene, column_3_right + H_GAP, row_2, "Round Constants K", k_content.clone())?;
            rects.push(k_rect);

            Half::Compression(Compression {
                working: c[1].0,
                temp1_terms: std::array::from_fn(|i| c[2 + i].0),
                temp1: c[7].0,
                temp2_terms: [c[8].0, c[9].0],
                temp2: c[10].0,
                next: c[11].0,
                digest: c[12].0,
                hash,
                k,
                k_content,
            })
        },
    };

    // "Step": see this function's own doc comment, "Stepping through it".
    let step_values: Vec<u8> = (1..=STEPS as u8).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U8(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    // Fit this diagram's own `<svg>` to what it draws. The stepping toolbar is at the top, inside `TOP`, so no room is
    // left below for it.
    let (right, bottom) = extent(&rects);
    fit_nested_size(&scene, DIAGRAM_ID, right, bottom, false)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    Ok((scene, Nodes { common, half }, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the diagram for `stage`, shows `position` in it, and wires its stepping toolbar. It replaces whatever diagram
/// was there, and keeps the zoom and pan and the keyboard focus the old one had.
///
/// # Errors
///
/// Returns `Err` if the window's own stage is missing, or if any library call fails.
fn build(stage: Stage, position: usize, trace: Rc<Trace>) -> Result<(), String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let focused = focused_button();

    let (scene, nodes, step) = build_scene(stage)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }
    apply(&scene, &nodes, &shown(&trace, position))?;

    let mut options = SelectionToolbarOptions::new(Side::North).with_stride(SelectionStride::new(8, "8"));
    options.margin = TOOLBAR_MARGIN;
    scene
        .show_selection_toolbar(step, options, |_scene, _node, transition| go(transition.to.unwrap_or(0)))
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    scene.set_selection(step, Selection::Cell(position)).map_err(stringify)?;

    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo { scene, nodes, step, trace });
    });
    if let Some(label) = focused {
        focus_button(&label);
    }
    crate::util::fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`. Within a half this only writes values and marks. Crossing into the
/// other half draws the diagram again for it. A failure is shown in the window's own error banner, and logged, since
/// the toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(position: usize) {
    match step_to(position) {
        Ok(()) => crate::util::clear_error(ERROR_ID),
        Err(e) => crate::util::report_error(ERROR_ID, &format!("This step failed (position {position}): {e}")),
    }
}

/// The work of [`go`].
fn step_to(position: usize) -> Result<(), String> {
    let wanted = stage(position);
    let (current, trace) = DEMO
        .with_borrow(|slot| slot.as_ref().map(|d| (d.nodes.stage(), d.trace.clone())))
        .ok_or("the diagram is not built")?;
    if current != wanted {
        return build(wanted, position, trace);
    }
    DEMO.with_borrow(|slot| {
        let demo = slot.as_ref().ok_or("the diagram is not built")?;
        apply(&demo.scene, &demo.nodes, &shown(&demo.trace, position))?;
        // A step back from the first position, or a restart, comes back as unstarted. Put the toolbar back on the
        // first position, so its next click advances from there and not from nowhere.
        if position == 0 {
            demo.scene.set_selection(demo.step, Selection::Cell(0)).map_err(stringify)?;
        }
        Ok(())
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible name of the toolbar button that has keyboard focus, if one in the diagram does.
fn focused_button() -> Option<String> {
    let document = crate::util::document().ok()?;
    let active = document.active_element()?;
    let diagram = document.get_element_by_id(DIAGRAM_ID)?;
    if diagram.contains(Some(&active)) {
        active.get_attribute("aria-label")
    } else {
        None
    }
}

/// Gives keyboard focus to the button in the diagram named `label`, if there is one and it can take focus. A button
/// that is disabled now is left alone.
fn focus_button(label: &str) {
    use wasm_bindgen::JsCast;
    let Ok(document) = crate::util::document() else { return };
    let selector = format!("#{DIAGRAM_ID} g[aria-label=\"{label}\"]");
    if let Ok(Some(button)) = document.query_selector(&selector) {
        if let Ok(button) = button.dyn_into::<web_sys::SvgElement>() {
            let _ = button.focus();
        }
    }
}

/// Builds the SHA-256 scene at its first step, with the message in focus, wires its stepping toolbar, and resizes the
/// window it runs in to fit the scene. Runs in `sha2-256-window.html`, not in the gallery.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#sha2-256-diagram-stage`, if the message does not fit one block, or if any
/// library call fails.
pub(crate) fn build_sha2_256_window() -> Result<(), String> {
    let trace = Rc::new(algorithm::trace(MESSAGE).ok_or("the message does not fit one block")?);
    build(stage(0), 0, trace)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The SHA-256 scene is larger than most demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `sha2-256-window.html`, which runs [`build_sha2_256_window`] there. A named
/// window is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha2-256-open`, or if a listener could not be attached to it.
pub(crate) fn build_sha2_256_demo() -> Result<(), String> {
    crate::util::wire_open_window(&crate::util::document()?, "sha2-256-open", WINDOW_PAGE, WINDOW_NAME)
}

/// The page [`build_sha2_256_demo`]'s button opens.
const WINDOW_PAGE: &str = "sha2-256-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-sha2-256";
