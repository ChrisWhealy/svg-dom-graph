//! `panel-sha1` / `#sha1-diagram`: SHA-1 of one short message, drawn as the standard describes it. See
//! [`build_scene`]'s own doc comment for what the diagram shows. The scene is large, so it runs in a window of its own.
//! The gallery's panel keeps the description and opens that window: see [`build_sha1_demo`].
//!
//! # Layout
//!
//! Data flows along one axis where it can. The compression phase runs left to right across the top row. A row that
//! would make the diagram too wide turns back on itself, so the flow snakes: the next row runs right to left, directly
//! beneath the end of the last. The schedule phase is a single column of nodes beside the schedule, itself a single
//! column. Every connector is a straight line between neighbouring nodes, and none crosses another or passes over a
//! node.
//!
//! # Phases
//!
//! The SHA1 algorithm performs its calculation in two distinct phases. The diagram shows only the current phase. While
//! the message schedule is being built, only the message, its block and the schedule are on screen, with the steps that
//! make each new schedule word.
//!
//! There then follows 80 rounds of compression, in four sections of 20. Every round in a section uses the same round
//! function and the same round constant, so the diagram names them, and crossing from one section to the next draws it
//! again. A node cannot be removed once drawn, so crossing from one stage to another draws the diagram again. Every
//! step inside a stage only writes new values and marks into the nodes already there, via [`apply`]. Which values and
//! marks a step shows comes from [`walk::shown`], a pure function of the position alone, so stepping backwards is the
//! same write as stepping forwards. Zoom and pan are kept across the redraw, and so is the keyboard focus on the button
//! that was pressed.
//!
//! # What is real
//!
//! All of it. [`algorithm::trace`] hashes [`walk::MESSAGE`] in plain Rust, and every value on screen is a piece of that
//! one calculation: the padded block, the 80-word message schedule, the 80 rounds of compression, and the digest. The
//! SHA-1 constants are fixed tables. `algorithm`'s unit tests check them, and check the digests of several messages
//! against the published values.
//!
//! # What is not covered
//!
//! A message of more than one block, which needs the blocks chained one into the next, and SHA-1's relatives.

mod algorithm;
mod walk;

use crate::source_file;
use algorithm::{INITIAL_HASH, ROUND_CONSTANTS, RoundFunction, Trace};
use demo_common::util::{ensure_svg_in, fit_nested_size, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionStride, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use walk::{MESSAGE, Ring, STEPS, Stage, shown, stage};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function, which only opens a window. The calculation comes first, then the walk
/// through it, then the scene that draws both.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("sha1/algorithm/mod.rs", "algorithm/mod.rs"),
    source_file!("sha1/algorithm/round.rs", "algorithm/round.rs"),
    source_file!("sha1/algorithm/trace.rs", "algorithm/trace.rs"),
    source_file!("sha1/walk/mod.rs", "walk/mod.rs"),
    source_file!("sha1/walk/phase.rs", "walk/phase.rs"),
    source_file!("sha1/walk/ring.rs", "walk/ring.rs"),
    source_file!("sha1/walk/shown.rs", "walk/shown.rs"),
    source_file!("sha1/mod.rs", "mod.rs"),
];

/// The id of the `<svg>` this demo draws into, and of the `<div>` in the window that hosts it.
const DIAGRAM_ID: &str = "sha1-diagram";
const STAGE_ID: &str = "sha1-diagram-stage";
/// The id of the hidden `.demo-error` paragraph in the window that shows a failure in a step.
const ERROR_ID: &str = "sha1-window-error";

/// The page [`build_sha1_demo`]'s button opens.
const WINDOW_PAGE: &str = "sha1-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-sha1";

/// The first column's left edge, and the first row's top. The top leaves room for the scene's title and, below it, the
/// stepping toolbar, which sits at the top of the diagram.
const LEFT: f64 = 20.0;
const TOP: f64 = 100.0;
/// How far the stepping toolbar sits from the top edge: below the scene's own title.
const TOOLBAR_MARGIN: f64 = 52.0;
/// The space between one node and the next along a row, and between one row and the next.
const H_GAP: f64 = 70.0;
const V_GAP: f64 = 70.0;
/// Words in the working variables, and in the terms of the round's sum.
const WORKING: usize = 5;
/// Words in the XOR node: the four schedule words, and their XOR.
const XOR_WORDS: usize = 5;
/// The labels of the XOR node's own rows: the four schedule words it reads, then their XOR.
const XOR_LABELS: &[&str] = &["ms[n-3]", "ms[n-8]", "ms[n-14]", "ms[n-16]", "XOR"];
/// The labels of the rows of the terms of `temp`, for each section of rounds. Only `f` differs from one to the next.
/// `W` is the schedule word for the round, the one marked in the schedule.
const CHOICE_LABELS: &[&str] = &["ROTL(a,5)", "Ch(b,c,d)", "e", "K", "W = ms[i]"];
const PARITY_LABELS: &[&str] = &["ROTL(a,5)", "Parity(b,c,d)", "e", "K", "W = ms[i]"];
const MAJORITY_LABELS: &[&str] = &["ROTL(a,5)", "Maj(b,c,d)", "e", "K", "W = ms[i]"];

/// The labels of the rows of the terms of `temp` in `section`.
fn terms_labels(section: usize) -> &'static [&'static str] {
    match RoundFunction::of_section(section) {
        RoundFunction::Choice => CHOICE_LABELS,
        RoundFunction::Parity => PARITY_LABELS,
        RoundFunction::Majority => MAJORITY_LABELS,
    }
}

/// Words to a row of the message schedule in the first stage.
const EXPANSION_COLUMNS: usize = 4;
/// How far inside the schedule's top and bottom edges the block and the new word are level with it.
const SCHEDULE_INSET: f64 = 20.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the toolbar's callback needs to find again on every step: the scene, its nodes, and the hash being walked.
struct Demo {
    scene: Scene,
    nodes: Nodes,
    /// The "Step" node that drives the selection toolbar.
    step: NodeId,
    /// The stage the nodes were drawn for.
    drawn_stage: Stage,
    trace: Rc<Trace>,
    /// The position the nodes' values show now, or `None` if the last write to them failed partway, so nothing is known
    /// about what they hold. [`apply`] skips what this position already shows.
    shown_at: Option<usize>,
}

thread_local! {
    // Keeps the `Scene` alive for the page's lifetime. Every listener the toolbars install holds only a `Weak`
    // reference back to it, so without a strong handle kept here it would drop the moment [`build`] returns.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's own content: `values` as `u32` words in binary, one to a row. Rotations are the heart of SHA-1, and bits
/// make them visible. A word is four bytes, shown big-endian, the way the standard writes them.
fn words(values: &[u32]) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values.to_vec()), DataFormat::Binary).with_layout(GridLayout::Columns(1))
}

/// A node's own content: `values` as `u32` words in hexadecimal, `columns` to a row. The message block and the message
/// schedule have so many words that binary would make them far too tall or wide.
fn grid(values: &[u32], columns: usize) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values.to_vec()), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(columns))
}

/// `content` with its words labelled `a` to `e`, as the standard names the working variables.
fn lettered(content: DataNodeContent) -> DataNodeContent {
    content.with_labelling_style(LabellingStyle::Alphabetic)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A straight connector between two sides, with each end fixed to that side's middle.
fn between(from: Side, to: Side) -> ConnectorOptions {
    ConnectorOptions::default()
        .with_connector_type(ConnectorType::Straight)
        .with_from_side(Some(from))
        .with_to_side(Some(to))
}

/// A connector for a flow to the right, along a row.
fn rightward() -> ConnectorOptions {
    between(Side::East, Side::West)
}

/// A connector for a flow to the left, along a row that has turned back on itself.
fn leftward() -> ConnectorOptions {
    between(Side::West, Side::East)
}

/// A connector for a flow straight down, from one row to the next.
fn downward() -> ConnectorOptions {
    between(Side::South, Side::North)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node drawn on the diagram, with the rectangle it occupies, so the next one can be placed relative to it.
#[derive(Clone, Copy)]
struct Placed {
    id: NodeId,
    rect: Rect,
}

impl Placed {
    fn right(&self) -> f64 {
        self.rect.origin.x + self.rect.size.width
    }

    fn bottom(&self) -> f64 {
        self.rect.origin.y + self.rect.size.height
    }

    fn middle_x(&self) -> f64 {
        self.rect.origin.x + self.rect.size.width / 2.0
    }
}

/// The size a named data node will have, so it can be placed before it is drawn.
fn measure(scene: &Scene, name: &str, content: &DataNodeContent) -> Result<Size, String> {
    scene.measure_named_data_node(name, content).map_err(stringify)
}

/// Adds a named data node whose left edge is at `left` and whose middle is at height `middle_y`.
fn add_at_left(
    scene: &Scene,
    left: f64,
    middle_y: f64,
    name: &str,
    content: DataNodeContent,
) -> Result<Placed, String> {
    let size = measure(scene, name, &content)?;
    add(scene, Point::new(left, middle_y - size.height / 2.0), name, content)
}

/// Adds a named data node whose middle is at `(middle_x, middle_y)`.
fn add_centred(
    scene: &Scene,
    middle_x: f64,
    middle_y: f64,
    name: &str,
    content: DataNodeContent,
) -> Result<Placed, String> {
    let size = measure(scene, name, &content)?;
    add(
        scene,
        Point::new(middle_x - size.width / 2.0, middle_y - size.height / 2.0),
        name,
        content,
    )
}

/// Adds a named data node with its top left at `top_left`, and returns it with its measured rectangle.
fn add(scene: &Scene, top_left: Point, name: &str, content: DataNodeContent) -> Result<Placed, String> {
    let id = scene.add_named_data_node(top_left, name, content).map_err(stringify)?;
    Ok(Placed {
        id,
        rect: scene.node_rect(id).map_err(stringify)?,
    })
}

/// The right edge of the rightmost of `placed`, and the lowest edge of any of them.
fn extent(placed: &[Placed]) -> (f64, f64) {
    placed.iter().fold((0.0, 0.0), |(right, bottom), p| {
        (f64::max(right, p.right()), f64::max(bottom, p.bottom()))
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes both stages draw: the message schedule, and the walk's focus on it.
struct Common {
    schedule: NodeId,
    schedule_content: DataNodeContent,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the first stage adds: the message and its block, and the two nodes that build one new schedule word.
struct Expansion {
    message: NodeId,
    block: NodeId,
    /// The four schedule words word `n` is built from, `ms[n-3]`, `ms[n-8]`, `ms[n-14]` and `ms[n-16]`, then their XOR.
    xor: NodeId,
    /// The XOR rotated left by one bit, which is schedule word `n`.
    sum: NodeId,
}

/// What the second stage adds, for one section of 20 rounds.
struct Compression {
    working: NodeId,
    /// The five terms of `temp`: `ROTL(a, 5)`, `f(b, c, d)`, `e`, `K` and `W`.
    terms: NodeId,
    temp: NodeId,
    next: NodeId,
    digest: NodeId,
    /// The digest as a hexadecimal hash value, below it.
    hash: NodeId,
}

/// The nodes of whichever stage is drawn.
enum Half {
    Expansion(Expansion),
    Compression(Compression),
}

/// Every node the walk writes to after the diagram is drawn.
struct Nodes {
    common: Common,
    half: Half,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes everything the walk shows at one position into the nodes already drawn: each node's values, then the marks.
///
/// Every write is absolute, so a step back and a step forward are the same operation. A node the walk has not reached
/// shows zeros, and a mark it has not reached is cleared.
///
/// `previous` is what the nodes show now, if that is known. The message block and the message schedule are the two
/// biggest nodes, and they are final for most of the walk. They are only written when `previous` shows something
/// different, so a step during compression leaves their cells alone. Without a `previous`, as after drawing the diagram
/// afresh, everything is written.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn apply(scene: &Scene, nodes: &Nodes, shown: &walk::Shown, previous: Option<&walk::Shown>) -> Result<(), String> {
    let set =
        |node: NodeId, values: &[u32]| scene.set_data_values(node, NodeValues::U32(values.to_vec())).map_err(stringify);
    if previous.is_none_or(|p| p.schedule != shown.schedule) {
        set(nodes.common.schedule, &shown.schedule)?;
    }

    match &nodes.half {
        Half::Expansion(e) => {
            if previous.is_none_or(|p| p.block != shown.block) {
                set(e.block, &shown.block)?;
            }
            let [w3, w8, w14, w16] = shown.expansion.map_or([0; 4], |x| x.inputs);
            set(e.xor, &[w3, w8, w14, w16, shown.expansion.map_or(0, |x| x.xor)])?;
            set(e.sum, &[shown.expansion.map_or(0, |x| x.sum)])?;
        },
        Half::Compression(c) => {
            let round = shown.round.as_ref();
            set(c.working, &shown.working)?;
            set(c.terms, &round.map_or([0; WORKING], |r| r.terms))?;
            set(c.temp, &[round.map_or(0, |r| r.temp)])?;
            set(c.next, &round.map_or([0; WORKING], |r| r.after))?;
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
/// being built or used, and the four words it reads. Whatever `shown` does not name is cleared. The message and block
/// exist only in the first stage, and the digest only in the second. Then the cells with no value yet are marked, by
/// [`apply_unreached`].
fn apply_marks(scene: &Scene, nodes: &Nodes, shown: &walk::Shown) -> Result<(), String> {
    let rings: Vec<(Ring, NodeId)> = match &nodes.half {
        Half::Expansion(e) => vec![(Ring::Message, e.message), (Ring::Block, e.block)],
        Half::Compression(c) => vec![(Ring::Digest, c.digest), (Ring::Hash, c.hash)],
    };
    for (ring, node) in rings {
        scene.set_focus(node, shown.ring == Some(ring)).map_err(stringify)?;
    }
    let focus = shown
        .schedule_focus
        .and_then(|i| nodes.common.schedule_content.natural_selection(i))
        .unwrap_or(Selection::None);
    scene.set_selection(nodes.common.schedule, focus).map_err(stringify)?;
    scene
        .set_secondary_selection(nodes.common.schedule, &shown.schedule_secondary)
        .map_err(stringify)?;
    apply_unreached(scene, nodes, shown)
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

    mark(nodes.common.schedule, &shown.schedule_unreached)?;
    match &nodes.half {
        Half::Expansion(e) => {
            mark(e.block, &first(algorithm::BLOCK_WORDS, shown.block_unreached))?;
            mark(e.xor, &first(XOR_WORDS, shown.terms_unreached))?;
            mark(e.sum, &first(1, shown.terms_unreached))?;
        },
        Half::Compression(c) => {
            mark(c.working, &first(WORKING, shown.terms_unreached))?;
            mark(c.terms, &first(WORKING, shown.terms_unreached))?;
            mark(c.temp, &first(1, shown.terms_unreached))?;
            mark(c.next, &first(WORKING, shown.terms_unreached))?;
            mark(c.digest, &first(WORKING, shown.digest_unreached))?;
            mark(c.hash, &first(1, shown.hash_unreached))?;
        },
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the first stage, with nothing reached yet, and returns its nodes and every rectangle it occupies.
///
/// # What this draws
///
/// **The column on the left** runs from top to bottom. "Plain Text Message" is the text being hashed. "Message Block"
/// is that text as 16 big-endian words, followed by the `0x80` terminator byte, with the message's length in bits in
/// the last word. Below it is one new schedule word being built. The "XOR" node holds the four words it reads, labelled
/// `ms[n-3]`, `ms[n-8]`, `ms[n-14]` and `ms[n-16]`, which are also marked in the schedule as secondary, then their XOR
/// in a fifth row. The node below it is that XOR rotated left by one bit, which is the new word.
///
/// **"Message Schedule"** is on the right: 80 words, [`EXPANSION_COLUMNS`] to a row. The first 16 are the block, and
/// each of the other 64 is `ROTL(ms[n-3] XOR ms[n-8] XOR ms[n-14] XOR ms[n-16], 1)`. The new word is marked in it as
/// the word in focus. The three connectors that reach the schedule each run straight across. The schedule is placed
/// beside the column so that the block is level with its top, and the new word is level with its bottom. So the gap
/// between the nodes of the column is set by the schedule's own height, unless that would be less than [`V_GAP`].
fn build_expansion(scene: &Scene) -> Result<(Common, Half, Vec<Placed>), String> {
    let message_name = "Plain Text Message";
    let block_name = "Message Block";
    let xor_name = "XOR";
    let sum_name = "ms[n] = ROTL(XOR, 1)";
    let message_content = DataNodeContent::new(NodeValues::U8(MESSAGE.to_vec()), DataFormat::PlainText);
    let block_content = grid(&[0; algorithm::BLOCK_WORDS], 4);
    let schedule_content = grid(&[0; algorithm::SCHEDULE_WORDS], EXPANSION_COLUMNS);
    let xor_content = words(&[0; XOR_WORDS]).with_labelling_style(LabellingStyle::Text(XOR_LABELS));
    let sum_content = words(&[0]);

    let message_size = measure(scene, message_name, &message_content)?;
    let block_size = measure(scene, block_name, &block_content)?;
    let xor_size = measure(scene, xor_name, &xor_content)?;
    let sum_size = measure(scene, sum_name, &sum_content)?;
    let schedule_size = measure(scene, "Message Schedule", &schedule_content)?;

    // Every node of the column is centred on the widest one's own middle, so each connector between two runs straight
    // down.
    let column_width = [message_size, block_size, xor_size, sum_size]
        .iter()
        .fold(0.0, |widest, size| f64::max(widest, size.width));
    let column_x = LEFT + column_width / 2.0;

    // The block is level with the top of the schedule, and the new word with its bottom, each a little way in. If the
    // schedule is too short for that to leave a gap of `V_GAP` between the nodes, the gap is `V_GAP`, and the
    // connectors meet the schedule's edge instead.
    let block_top = TOP + message_size.height + V_GAP;
    let reach = schedule_size.height
        - 2.0 * SCHEDULE_INSET
        - (block_size.height / 2.0 + xor_size.height + sum_size.height / 2.0);
    let gap = f64::max(V_GAP, reach / 2.0);

    let message = add_centred(scene, column_x, TOP + message_size.height / 2.0, message_name, message_content)?;
    let block = add_centred(scene, column_x, block_top + block_size.height / 2.0, block_name, block_content)?;
    let xor_top = block.bottom() + gap;
    let xor = add_centred(scene, column_x, xor_top + xor_size.height / 2.0, xor_name, xor_content)?;
    let sum_top = xor.bottom() + gap;
    let sum = add_centred(scene, column_x, sum_top + sum_size.height / 2.0, sum_name, sum_content)?;

    let schedule_top = block_top + block_size.height / 2.0 - SCHEDULE_INSET;
    let schedule = add_at_left(
        scene,
        LEFT + column_width + 2.0 * H_GAP,
        schedule_top + schedule_size.height / 2.0,
        "Message Schedule",
        schedule_content.clone(),
    )?;

    // Where along the schedule's left side a node's own middle is, so that a connector to it runs level.
    let level = |node: &Placed| {
        let middle = node.rect.origin.y + node.rect.size.height / 2.0;
        ((middle - schedule.rect.origin.y) / schedule.rect.size.height).clamp(0.0, 1.0)
    };
    scene.add_edge_with(message.id, block.id, downward()).map_err(stringify)?;
    scene
        .add_edge_with(block.id, schedule.id, rightward().with_to_position(Some(level(&block))))
        .map_err(stringify)?;
    scene
        .add_edge_with(schedule.id, xor.id, leftward().with_from_position(Some(level(&xor))))
        .map_err(stringify)?;
    scene.add_edge_with(xor.id, sum.id, downward()).map_err(stringify)?;
    scene
        .add_edge_with(sum.id, schedule.id, rightward().with_to_position(Some(level(&sum))))
        .map_err(stringify)?;

    let common = Common {
        schedule: schedule.id,
        schedule_content,
    };
    let half = Half::Expansion(Expansion {
        message: message.id,
        block: block.id,
        xor: xor.id,
        sum: sum.id,
    });
    Ok((common, half, vec![message, block, xor, sum, schedule]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the second stage for `section` (20 rounds), with nothing reached yet, and returns its nodes and every
/// rectangle it occupies.
///
/// # What this draws
///
/// **Row 1**, left to right, is one round. "Initial hash H" starts "Working variables a..e", which lead into the five
/// terms of `temp = ROTL(a, 5) + f(b, c, d) + e + K + W`, each in a row labelled with its name. `f` and `K` are the
/// same for every round of a section, so the name of `f` changes from one section to the next. `W` is `ms[i]`, the
/// schedule word for round `i`, which sits above the row. It is the word marked in the schedule, and a connector runs
/// down from the schedule to the terms. "Initial hash H" is named as fixed by the standard, since it is not worked out
/// from the message.
///
/// **Row 2** turns back on itself, directly below, and runs right to left. "temp" is the sum of the terms, and sits
/// straight below them, centred on them. It is `a` of the next round, so it leads into "Next working variables", which
/// are `e=d`, `d=c`, `c=ROTL(b, 30)`, `b=a` and `a=temp`. They sit below the current working variables, which they are
/// shifted along from. After 80 rounds, "Digest" is each working variable added to the initial hash word it began as.
/// It sits below the initial hash, which it is added to. Without that last addition the result would not be a hash of
/// the block. A last step writes the digest out below it as the hexadecimal hash value.
fn build_compression(scene: &Scene, section: usize) -> Result<(Common, Half, Vec<Placed>), String> {
    let terms_name = "Terms of temp, added together";
    let terms_content = words(&[0; WORKING]).with_labelling_style(LabellingStyle::Text(terms_labels(section)));
    let h_name = "Initial hash H: fixed by the SHA-1 standard";
    let schedule_name = "Message Schedule: round i uses the marked word ms[i] as W";
    let working_name = "Working variables a..e";
    let next_name = "Next: e=d, d=c, c=ROTL(b,30), b=a, a=temp";
    let temp_name = "temp = sum of the terms";
    let digest_name = "Digest = H + working variables";
    let hash_name = "SHA-1 hash";

    let schedule_content = grid(&[0; algorithm::SCHEDULE_WORDS], 8);
    let schedule_size = measure(scene, schedule_name, &schedule_content)?;
    let row_size = measure(scene, terms_name, &terms_content)?;
    let hash_content = DataNodeContent::new(NodeValues::U8(vec![b'0'; 40]), DataFormat::PlainText);
    let hash_size = measure(scene, hash_name, &hash_content)?;

    // The first column holds the initial hash and the digest, and the hash below that is wider than either. So the
    // column starts far enough from the left edge for the hash to be centred on it.
    let widest_h = measure(scene, h_name, &words(&INITIAL_HASH))?
        .width
        .max(measure(scene, digest_name, &words(&[0; WORKING]))?.width);
    let left = LEFT + ((hash_size.width - widest_h) / 2.0).max(0.0);

    let middle_1 = TOP + schedule_size.height + V_GAP + row_size.height / 2.0;
    let h = add_at_left(scene, left, middle_1, h_name, words(&INITIAL_HASH))?;
    let working = add_at_left(scene, h.right() + H_GAP, middle_1, working_name, lettered(words(&[0; WORKING])))?;
    let terms = add_at_left(scene, working.right() + H_GAP, middle_1, terms_name, terms_content)?;

    // The schedule is centred above the terms, so its word for this round comes down straight to them.
    let schedule = add_centred(
        scene,
        terms.middle_x(),
        TOP + schedule_size.height / 2.0,
        schedule_name,
        schedule_content.clone(),
    )?;

    scene.add_edge_with(h.id, working.id, rightward()).map_err(stringify)?;
    scene.add_edge_with(working.id, terms.id, rightward()).map_err(stringify)?;
    scene.add_edge_with(schedule.id, terms.id, downward()).map_err(stringify)?;

    // Row 2: each node sits straight below the one it is shifted from, or added to.
    let middle_2 = middle_1 + row_size.height + V_GAP;
    let next = add_centred(scene, working.middle_x(), middle_2, next_name, lettered(words(&[0; WORKING])))?;
    let temp = add_centred(scene, terms.middle_x(), middle_2, temp_name, words(&[0]))?;
    let digest = add_centred(scene, h.middle_x(), middle_2, digest_name, words(&[0; WORKING]))?;
    scene.add_edge_with(working.id, next.id, downward()).map_err(stringify)?;
    scene.add_edge_with(next.id, digest.id, leftward()).map_err(stringify)?;
    scene.add_edge_with(h.id, digest.id, downward()).map_err(stringify)?;
    scene.add_edge_with(terms.id, temp.id, downward()).map_err(stringify)?;
    // `temp` is `a` of the next round, so it joins the next working variables, which are next along the row.
    scene.add_edge_with(temp.id, next.id, leftward()).map_err(stringify)?;
    let hash = add_centred(
        scene,
        digest.middle_x(),
        middle_2 + row_size.height / 2.0 + V_GAP + hash_size.height / 2.0,
        hash_name,
        hash_content,
    )?;
    scene.add_edge_with(digest.id, hash.id, downward()).map_err(stringify)?;

    let common = Common {
        schedule: schedule.id,
        schedule_content,
    };
    let half = Half::Compression(Compression {
        working: working.id,
        terms: terms.id,
        temp: temp.id,
        next: next.id,
        digest: digest.id,
        hash: hash.id,
    });
    Ok((common, half, vec![h, working, terms, temp, schedule, next, digest, hash]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the diagram for `stage`, with nothing reached yet, and returns its nodes and the "Step" node that drives its
/// toolbar. See [`build_expansion`] and [`build_compression`] for what each stage draws.
///
/// # Stepping through it
///
/// "Step" is a small array of one cell per walk position, placed far off-canvas, purely to drive a selection toolbar.
/// It sits at the top of the diagram. See [`walk`] for what each position shows, and [`apply`] for how a position is
/// written into the diagram. Anything the walk has not reached yet is zeros.
///
/// # Errors
///
/// Returns `Err` if the window's own stage is missing, or if any library call fails.
fn build_scene(stage: Stage) -> Result<(Scene, Nodes, NodeId), String> {
    let document = demo_common::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1900.0, 1200.0))?;
    demo_common::util::required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let title = match stage {
        Stage::Expansion => "SHA-1: Build the Message Schedule".to_owned(),
        Stage::Compression(section) => {
            let first = section * algorithm::ROUNDS_PER_SECTION;
            let function = match RoundFunction::of_section(section) {
                RoundFunction::Choice => "choice",
                RoundFunction::Parity => "parity",
                RoundFunction::Majority => "majority",
            };
            let k = ROUND_CONSTANTS[section];
            format!(
                "SHA-1: Rounds {first} to {}, using {function} and K = 0x{k:08x}",
                first + algorithm::ROUNDS_PER_SECTION - 1
            )
        },
    };
    scene
        .show_scene_title(&title, SceneTitleOptions::default())
        .map_err(stringify)?;

    let (common, half, placed) = match stage {
        Stage::Expansion => build_expansion(&scene)?,
        Stage::Compression(section) => build_compression(&scene, section)?,
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
    demo_common::util::hide_step_driver(DIAGRAM_ID)?;

    // Fit this diagram's own `<svg>` to what it draws. The stepping toolbar is at the top, inside `TOP`, so no room is
    // left below for it.
    let (right, bottom) = extent(&placed);
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
    apply(&scene, &nodes, &shown(&trace, position), None)?;

    let mut options = SelectionToolbarOptions::new(Side::North).with_stride(SelectionStride::new(8, "8"));
    options.margin = TOOLBAR_MARGIN;
    scene
        .show_selection_toolbar(step, options, |_scene, _node, transition| go(transition.to.unwrap_or(0)))
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    scene.set_selection(step, Selection::Cell(position)).map_err(stringify)?;

    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene,
            nodes,
            step,
            drawn_stage: stage,
            trace,
            shown_at: Some(position),
        });
    });
    if let Some(label) = focused {
        focus_button(&label);
    }
    demo_common::util::fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`. Within a stage this only writes values and marks. Crossing into
/// another stage draws the diagram again for it. A failure is shown in the window's own error banner, and logged, since
/// the toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(position: usize) {
    match step_to(position) {
        Ok(()) => demo_common::util::clear_error(ERROR_ID),
        Err(e) => demo_common::util::report_error(ERROR_ID, &format!("This step failed (position {position}): {e}")),
    }
}

/// The work of [`go`].
fn step_to(position: usize) -> Result<(), String> {
    let wanted = stage(position);
    let (current, trace) = DEMO
        .with_borrow(|slot| slot.as_ref().map(|d| (d.drawn_stage, d.trace.clone())))
        .ok_or("the diagram is not built")?;
    if current != wanted {
        return build(wanted, position, trace);
    }
    DEMO.with_borrow_mut(|slot| {
        let demo = slot.as_mut().ok_or("the diagram is not built")?;
        // Unknown until the write has finished, so a failure partway leaves the next step writing everything.
        let previous = demo.shown_at.take().map(|at| shown(&demo.trace, at));
        apply(&demo.scene, &demo.nodes, &shown(&demo.trace, position), previous.as_ref())?;
        demo.shown_at = Some(position);
        // A step back from the first position, or a restart, comes back as unstarted. Put the toolbar back on the first
        // position, so its next click advances from there and not from nowhere.
        if position == 0 {
            demo.scene.set_selection(demo.step, Selection::Cell(0)).map_err(stringify)?;
        }
        Ok(())
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible name of the toolbar button that has keyboard focus, if one in the diagram does.
fn focused_button() -> Option<String> {
    let document = demo_common::util::document().ok()?;
    let active = document.active_element()?;
    let diagram = document.get_element_by_id(DIAGRAM_ID)?;
    if diagram.contains(Some(&active)) {
        active.get_attribute("aria-label")
    } else {
        None
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives keyboard focus to the button in the diagram named `label`, if there is one and it can take focus. A button
/// that is disabled now is left alone.
fn focus_button(label: &str) {
    use wasm_bindgen::JsCast;
    let Ok(document) = demo_common::util::document() else { return };
    let selector = format!("#{DIAGRAM_ID} g[aria-label=\"{label}\"]");
    if let Ok(Some(button)) = document.query_selector(&selector) {
        if let Ok(button) = button.dyn_into::<web_sys::SvgElement>() {
            let _ = button.focus();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the SHA-1 scene at its first step, with the message in focus, wires its stepping toolbar, and resizes the
/// window it runs in to fit the scene. Runs in `sha1-window.html`, not in the gallery.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#sha1-diagram-stage`, if the message does not fit one block, or if any library
/// call fails.
pub(crate) fn build_sha1_window() -> Result<(), String> {
    let trace = Rc::new(algorithm::trace(MESSAGE).ok_or("the message does not fit one block")?);
    build(stage(0), 0, trace)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The SHA-1 scene is larger than most demos. So the gallery panel keeps only the description, plus a button that opens
/// the scene in a window of its own: `sha1-window.html`, which runs [`build_sha1_window`] there. A named window is
/// reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#sha1-open`, or if a listener could not be attached to it.
pub(crate) fn build_sha1_demo() -> Result<(), String> {
    demo_common::util::wire_open_window(&demo_common::util::document()?, "sha1-open", WINDOW_PAGE, WINDOW_NAME)
}
