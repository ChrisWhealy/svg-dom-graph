//! `panel-huffman` / `#huffman-diagram`: Huffman encoding, which counts the bytes of a text, builds the code tree,
//! gives every byte its canonical code and then packs the codes into bytes. See [`build_scene`]'s own doc comment for
//! what the diagram shows. The scene is large, so it runs in a window of its own. The gallery's panel keeps the
//! description and opens that window: see [`build_huffman_demo`].
//!
//! Huffman encoding takes two passes over the text. The first counts how often each byte occurs and builds the tree.
//! The second replaces each byte with its code and packs the odd-length bit sequences into whole bytes. This file draws
//! both as one walk, and the decoder comes later.
//!
//! # One scene per step
//!
//! A node or edge cannot be removed once drawn, and every step changes which nodes exist. So each step draws the whole
//! scene again, from [`walk::shown`] alone, and keeps the zoom and pan and the keyboard focus the old one had. Stepping
//! backwards is the same work as stepping forwards.
//!
//! # The text to encode
//!
//! The text is typed into a text input node in the scene itself. Each change draws the scene again, at its first step,
//! and puts the keyboard focus and the caret back in the new text box, so typing carries on. A text that cannot be
//! encoded leaves the diagram showing the last one that could, and says why in the status line.
//!
//! # What is real
//!
//! All of it. [`algorithm::trace`] encodes the text in plain Rust, and every value on screen is a piece of that one
//! calculation: the counts, the merges, the tree's paths, the canonical codes, the pending bits and the output. Symbols
//! are bytes, so a multi-byte character is counted as its separate bytes, as real Huffman encoders do.

mod algorithm;
mod walk;

use crate::source_file;
use algorithm::{MAX_INPUT_LEN, Trace};
use demo_common::util::{CONTENT_TOP, ensure_svg_in, fit_nested_size, required_element, stringify};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        BoolFormat, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, HexDumpFormat,
        NodeOptions, NodeValues, Scene, SceneTitleOptions, Selection, SelectionStride, Side, ToolbarOptions,
    },
};
use walk::{Packing, Row, Shown, Spacing, leaf_label, pack_steps, packing, steps, symbol_label};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function, which only opens a window. The calculation comes first, then the walk
/// through it, then the scene that draws both.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("huffman/algorithm/mod.rs", "algorithm/mod.rs"),
    source_file!("huffman/algorithm/tree.rs", "algorithm/tree.rs"),
    source_file!("huffman/algorithm/codes.rs", "algorithm/codes.rs"),
    source_file!("huffman/algorithm/pack.rs", "algorithm/pack.rs"),
    source_file!("huffman/walk/mod.rs", "walk/mod.rs"),
    source_file!("huffman/walk/shown.rs", "walk/shown.rs"),
    source_file!("huffman/walk/packing.rs", "walk/packing.rs"),
    source_file!("huffman/walk/layout.rs", "walk/layout.rs"),
    source_file!("huffman/mod.rs", "mod.rs"),
];

/// The text the demo opens on: the first sentence of Jane Austen's *Pride and Prejudice*.
const DEFAULT_TEXT: &str = "It is a truth universally acknowledged, that a single man in possession of a good fortune must be in want of a wife.";

/// What the toolbar's callback and the text box's callback need to find again: the scene, and what it shows.
struct Demo {
    scene: Scene,
    model: Model,
}

thread_local! {
    // Keeps the `Scene` alive for the page's lifetime. Every listener the toolbars install holds only a `Weak`
    // reference back to it, so without a strong handle kept here it would drop the moment [`build`] returns.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
}

/// The ids of the `<svg>` this demo draws into, and of the `<div>` in the window that hosts it.
const DIAGRAM_ID: &str = "huffman-diagram";
const STAGE_ID: &str = "huffman-diagram-stage";
/// The id of the hidden `.demo-error` paragraph in the window that shows a failure in a step.
const ERROR_ID: &str = "huffman-window-error";

/// The first column's left edge.
const LEFT: f64 = 20.0;
/// The space between one block of the scene and the next below it, and between the closely stacked nodes at the top.
const V_GAP: f64 = 36.0;
const GAP: f64 = 12.0;
/// The width of the text box and of the two Markdown nodes. Longer lines wrap.
const PANEL_WIDTH: f64 = 560.0;
/// The fill of the code table's heading row. The rows below it keep the usual light blue.
const HEADER_FILL: &str = "#bdd3f5";
/// The code table's columns: symbol, count, tree path, length, canonical code. Each is a box wide enough for its widest
/// text, and the headings are the first row.
const COLUMNS: [(&str, f64); 5] = [
    ("Byte", 80.0),
    ("Count", 60.0),
    ("Tree path", 100.0),
    ("Length", 60.0),
    ("Code", 100.0),
];
/// The space between the forest, at its widest, and the code table to its right.
const TABLE_GAP: f64 = 40.0;
/// The height of a row of the code table.
const ROW_HEIGHT: f64 = 26.0;
/// How many positions the toolbar's `Prev 8` and `Next 8` buttons move.
const STRIDE: usize = 8;
/// What a named data node adds to the width of its own grid: the frame and the room for its name. The measured width of
/// a grid does not include it.
const NAMED_FRAME: f64 = 40.0;
/// The width of the plain box that stands in for a row of bits or bytes that is empty, since a data node cannot be.
const EMPTY_WIDTH: f64 = 240.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text that has been encoded, and what is worked out once from it and kept.
struct Encoding {
    trace: Trace,
    /// The height the explanation needs at its tallest step, once it has been measured. Every step reserves that much
    /// room, so nothing below the explanation moves as the walk steps.
    explanation_height: Cell<Option<f64>>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the status line says.
#[derive(Clone)]
enum Status {
    /// The text was encoded.
    Encoded,
    Empty,
    /// The text is this many bytes, which is more than [`MAX_INPUT_LEN`].
    TooLong(usize),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything a scene is drawn from, so that stepping and typing each only have to change one part of it.
#[derive(Clone)]
struct Model {
    /// What the text box holds. It can be a text that cannot be encoded.
    text: String,
    /// The last text that could be encoded, which is the one the diagram shows.
    encoding: Rc<Encoding>,
    position: usize,
    status: Status,
}

impl Model {
    fn new(text: &str, trace: Trace) -> Self {
        Self {
            text: text.to_owned(),
            encoding: Rc::new(Encoding {
                trace,
                explanation_height: Cell::new(None),
            }),
            position: 0,
            status: Status::Encoded,
        }
    }

    /// The model after the text box changes to `text`. A text that can be encoded starts the walk again. One that
    /// cannot leaves the diagram as it was and only changes what the status line says.
    fn with_text(&self, text: &str) -> Self {
        match algorithm::trace(text.as_bytes()) {
            Some(trace) => Self::new(text, trace),
            None => Self {
                text: text.to_owned(),
                status: if text.is_empty() { Status::Empty } else { Status::TooLong(text.len()) },
                ..self.clone()
            },
        }
    }

    /// The status line, as Markdown.
    fn status_text(&self) -> String {
        let trace = &self.encoding.trace;
        match self.status {
            Status::Encoded => format!(
                "**{} bytes**, {} distinct, {} merges.",
                trace.frequencies.iter().map(|f| f.count).sum::<u32>(),
                trace.frequencies.len(),
                trace.tree.merges.len()
            ),
            Status::Empty => {
                "**Nothing to encode.** Enter some text. The diagram shows the last text that could be encoded."
                    .to_owned()
            },
            Status::TooLong(bytes) => format!(
                "**{bytes} bytes is too long.** The limit is {MAX_INPUT_LEN}. The diagram shows the last text that could be encoded."
            ),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A straight, forced south-to-north connector, for a link that runs down from a parent to its child.
fn downward() -> ConnectorOptions {
    ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North))
        .with_connector_type(ConnectorType::Straight)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A leaf's content: its byte and its count as one run of text.
fn leaf_content(symbol: u8, weight: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(leaf_label(symbol, weight).into_bytes()), DataFormat::PlainText)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A branch's content: its weight. The weight is at most [`MAX_INPUT_LEN`], so it fits a `u16`.
fn branch_content(weight: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U16(vec![weight as u16]), DataFormat::Decimal)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content of the small box that labels a link: a bit, written `0` or `1`.
fn bit_content(bit: bool) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(vec![bit]), DataFormat::Bool(BoolFormat::Bit))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A table cell's text, or a dash for one the walk has not reached.
fn cell(text: Option<String>) -> String {
    text.unwrap_or_else(|| "–".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text of each column of `row`, in the order of [`COLUMNS`].
fn row_cells(row: &Row) -> [String; 5] {
    [
        symbol_label(row.symbol),
        row.count.to_string(),
        // A tree of a single leaf has an empty path.
        cell(
            row.path
                .as_ref()
                .map(|p| if p.is_empty() { "(root)".to_owned() } else { p.clone() }),
        ),
        cell(row.length.map(|l| l.to_string())),
        cell(row.canonical.clone()),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes of the forest, so that what the step highlights can be found again.
struct Forest {
    /// Each tree node's own box, by its index in the tree's arena. `None` for a node that is not in the forest yet.
    boxes: Vec<Option<NodeId>>,
    /// The box that labels the link above each tree node, by the index of the node below it. `None` for a root.
    bits: Vec<Option<NodeId>>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the forest `shown` names, with its leaves on the line `leaf_y`, and returns its nodes.
///
/// A leaf is a data node of its byte and count, and a branch a data node of its weight. A branch's two links are each
/// drawn through a small box that reads `0` for the left child and `1` for the right, so the paths in the code table
/// can be read off the tree. Each box is centred on the middle the layout gives it. The branch the last merge made, and
/// the two trees it joined, are ringed.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_forest(scene: &Scene, trace: &Trace, shown: &Shown, leaf_y: f64, spacing: Spacing) -> Result<Forest, String> {
    let nodes = &trace.tree.nodes;
    let (placed, _) = walk::layout(nodes, &shown.roots);
    let level = spacing.level();

    let mut forest = Forest {
        boxes: vec![None; nodes.len()],
        bits: vec![None; nodes.len()],
    };
    let mut rects: Vec<Option<Rect>> = vec![None; nodes.len()];
    for p in &placed {
        let content = match nodes[p.node] {
            algorithm::Node::Leaf { symbol, weight } => leaf_content(symbol, weight),
            algorithm::Node::Branch { weight, .. } => branch_content(weight),
        };
        // The box is drawn where the layout expects it, and then moved so that its own measured middle is on the middle
        // the layout gave it. The layout only has an estimate of how wide the text is.
        let id = scene
            .add_data_node(Point::new(LEFT + p.x - p.width / 2.0, p.top(leaf_y, level)), content)
            .map_err(stringify)?;
        let rect = scene.node_rect(id).map_err(stringify)?;
        scene
            .move_node(id, Point::new(LEFT + p.x - rect.size.width / 2.0, rect.origin.y))
            .map_err(stringify)?;
        forest.boxes[p.node] = Some(id);
        rects[p.node] = Some(scene.node_rect(id).map_err(stringify)?);
    }

    // The links come after every box, so a parent and both its children are drawn before anything joins them.
    for p in &placed {
        let algorithm::Node::Branch { left, right, .. } = nodes[p.node] else {
            continue;
        };
        let (Some(parent), Some(parent_rect)) = (forest.boxes[p.node], rects[p.node]) else {
            continue;
        };
        for (child, bit) in [(left, false), (right, true)] {
            let (Some(child_id), Some(child_rect)) = (forest.boxes[child], rects[child]) else {
                continue;
            };
            let bit_size = scene.measure_data_node(&bit_content(bit)).map_err(stringify)?;
            let middle = (parent_rect.origin.y + parent_rect.size.height + child_rect.origin.y) / 2.0;
            let label = scene
                .add_data_node(
                    Point::new(
                        child_rect.origin.x + child_rect.size.width / 2.0 - bit_size.width / 2.0,
                        middle - bit_size.height / 2.0,
                    ),
                    bit_content(bit),
                )
                .map_err(stringify)?;
            forest.bits[child] = Some(label);
            scene.add_edge_with(parent, label, downward()).map_err(stringify)?;
            scene.add_edge_with(label, child_id, downward()).map_err(stringify)?;
        }
    }

    // The merge just made is ringed: the new branch and the two trees it joined.
    if let Some(merge) = shown.merged {
        for node in [merge.parent, merge.left, merge.right] {
            if let Some(id) = forest.boxes[node] {
                scene.set_focus(id, true).map_err(stringify)?;
            }
        }
    }
    Ok(forest)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks the route from the root to the leaf of the byte that has just been given its canonical code, with the
/// library's secondary selection: every box on it, and every link's label. A node with no box in the forest, which is
/// only possible for a route that names a node not drawn yet, is left alone.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn highlight_route(scene: &Scene, forest: &Forest, route: &[usize]) -> Result<(), String> {
    for (i, &node) in route.iter().enumerate() {
        let mut marked = vec![forest.boxes[node]];
        // Every node but the root has a link above it, and the route goes through that link.
        if i > 0 {
            marked.push(forest.bits[node]);
        }
        for id in marked.into_iter().flatten() {
            scene.set_secondary_selection(id, &[0]).map_err(stringify)?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the code table with its top left at `(left, top)`, one row for each distinct byte, the commonest first, and
/// returns the lowest edge. The first row holds the headings, and has a fill of its own. The row `focus_row` is ringed.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_table(scene: &Scene, shown: &Shown, focus_row: Option<usize>, left: f64, top: f64) -> Result<f64, String> {
    let headings = COLUMNS.map(|(heading, _)| heading.to_owned());
    let rows =
        std::iter::once((headings, None)).chain(shown.rows.iter().enumerate().map(|(i, r)| (row_cells(r), Some(i))));

    let mut y = top;
    for (cells, index) in rows {
        let options = if index.is_none() {
            NodeOptions::default().with_fill(Some(HEADER_FILL))
        } else {
            NodeOptions::default()
        };
        let mut x = left;
        for (text, (_, width)) in cells.iter().zip(COLUMNS) {
            let id = scene
                .add_node_with(Point::new(x, y), Size::new(width, ROW_HEIGHT), text, options)
                .map_err(stringify)?;
            if index.is_some() && index == focus_row {
                scene.set_focus(id, true).map_err(stringify)?;
            }
            x += width;
        }
        y += ROW_HEIGHT;
    }
    Ok(y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws one named row of data at `(left, top)` and returns its lowest edge. `marked` cells get the secondary
/// selection. `content` is `None` for a row with nothing in it, which is drawn as a plain box saying so.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_row(
    scene: &Scene,
    name: &str,
    content: Option<DataNodeContent>,
    marked: &[usize],
    left: f64,
    top: f64,
) -> Result<Extent, String> {
    let Some(content) = content else {
        scene
            .add_node(
                Point::new(left, top),
                Size::new(EMPTY_WIDTH, ROW_HEIGHT),
                format!("{name}: none"),
            )
            .map_err(stringify)?;
        return Ok((left + EMPTY_WIDTH, top + ROW_HEIGHT));
    };
    let id = scene
        .add_named_data_node(Point::new(left, top), name, content)
        .map_err(stringify)?;
    if !marked.is_empty() {
        scene.set_secondary_selection(id, marked).map_err(stringify)?;
    }
    let rect = scene.node_rect(id).map_err(stringify)?;
    Ok((rect.origin.x + rect.size.width, rect.origin.y + rect.size.height))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the input of pass two as one hex dump with one cell for each line of sixteen bytes, with its top left at
/// `(left, top)`, and returns the lowest edge. The dump shows each byte in hexadecimal and as a character, since a byte
/// that is not printable ASCII, such as one of a multi-byte character, is only a dot in the character column. The byte
/// whose code is being packed at step `index` is marked, and no byte is at the flush.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_input_array(scene: &Scene, trace: &Trace, index: usize, left: f64, top: f64) -> Result<f64, String> {
    let bytes: Vec<u8> = trace.packed.steps.iter().map(|s| s.symbol).collect();
    let marked: &[usize] = if index < bytes.len() { &[index] } else { &[] };
    Ok(draw_row(scene, "Input bytes", Some(dump_content(bytes)), marked, left, top)?.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The right and bottom edges of something drawn.
type Extent = (f64, f64);

/// The bytes in one hexadecimal row of the output.
const RESULT_COLUMNS: usize = 16;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A row of data of `bytes`, `columns` wide.
fn bytes_content(bytes: Vec<u8>, format: DataFormat, columns: usize) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(bytes), format).with_layout(GridLayout::Columns(columns))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bytes` as a hex dump in the format of `hexdump -C`, with one cell for each line of sixteen bytes.
fn dump_content(bytes: Vec<u8>) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(bytes), DataFormat::HexDump(HexDumpFormat::new()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A row of `count` bits, which is drawn on one line.
fn bits_content(bits: Vec<bool>) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(bits), DataFormat::Bool(BoolFormat::Bit)).with_layout(GridLayout::Rows(1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How wide the bits to pack can get at any step of pass two, so that the pending bits beside them do not move as the
/// walk steps.
///
/// # Errors
///
/// Returns `Err` if measuring fails.
fn bits_to_pack_width(scene: &Scene, trace: &Trace) -> Result<f64, String> {
    let longest = (0..pack_steps(trace))
        .map(|i| packing(trace, i).appended.len())
        .max()
        .unwrap_or(0);
    Ok(scene
        .measure_data_node(&bits_content(vec![false; longest.max(1)]))
        .map_err(stringify)?
        .width
        + NAMED_FRAME)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How wide pass two's stack of rows can get, so that what stands beside it does not move as the walk steps. It is the
/// widest of a full row of the output in hexadecimal and of the bits to pack beside the pending bits, which are at most
/// seven.
///
/// # Errors
///
/// Returns `Err` if measuring fails.
fn packing_width(scene: &Scene, bits_width: f64) -> Result<f64, String> {
    let hex = bytes_content(vec![0; RESULT_COLUMNS], DataFormat::Hexadecimal, RESULT_COLUMNS);
    let pending = bits_content(vec![false; 7]);
    let [hex, pending] = [hex, pending].map(|c| scene.measure_data_node(&c).map(|s| s.width));
    let pair = bits_width + GAP + pending.map_err(stringify)? + NAMED_FRAME;
    Ok(hex.map_err(stringify)?.max(pair))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws what pass two shows at one step, with its top left at `(left, top)` and `width` wide, and returns the lowest
/// edge: the bits being appended with the bits left pending beside them, and under those the bytes written so far in
/// hexadecimal. What the step added to the bits being appended is marked. At the flush it adds its size against the
/// input's.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_packing(
    scene: &Scene,
    packing: &Packing,
    left: f64,
    top: f64,
    width: f64,
    bits_width: f64,
) -> Result<f64, String> {
    let bits = |bits: &[bool]| (!bits.is_empty()).then(|| bits_content(bits.to_vec()));
    let bytes = (!packing.output.is_empty())
        .then(|| bytes_content(packing.output.clone(), DataFormat::Hexadecimal, RESULT_COLUMNS));

    // The bits to pack are a whole byte or more, and the pending bits are what did not fit, so they stand beside them.
    let pack = draw_row(scene, "Bits to pack", bits(&packing.appended), &packing.marked, left, top)?;
    let pending = draw_row(scene, "Pending bits", bits(&packing.pending), &[], left + bits_width + GAP, top)?;
    let mut y = pack.1.max(pending.1) + V_GAP;
    y = draw_row(scene, "Output bytes", bytes, &packing.new_bytes, left, y)?.1;

    if let Some(summary) = &packing.summary {
        y += V_GAP;
        let id = scene
            .add_markdown_node(Point::new(left, y), width, summary)
            .map_err(stringify)?;
        let rect = scene.node_rect(id).map_err(stringify)?;
        y = rect.origin.y + rect.size.height;
    }
    Ok(y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many positions the whole walk has: pass one, then pass two.
fn total_steps(trace: &Trace) -> usize {
    steps(trace) + pack_steps(trace)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The height the explanation needs at its tallest step of either pass, which every step reserves.
///
/// Measured once for each text, since it only depends on the walk, and kept in `encoding`.
///
/// # Errors
///
/// Returns `Err` if measuring fails.
fn explanation_height(scene: &Scene, encoding: &Encoding) -> Result<f64, String> {
    if let Some(height) = encoding.explanation_height.get() {
        return Ok(height);
    }
    let mut tallest: f64 = 0.0;
    let captions = (0..steps(&encoding.trace))
        .map(|position| walk::shown(&encoding.trace, position).caption)
        .chain((0..pack_steps(&encoding.trace)).map(|index| packing(&encoding.trace, index).caption));
    for caption in captions {
        let text = caption.join("\n\n");
        tallest = tallest.max(scene.measure_markdown_node(PANEL_WIDTH, &text).map_err(stringify)?.height);
    }
    encoding.explanation_height.set(Some(tallest));
    Ok(tallest)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The height the status line needs at its tallest, so that it can say any of its things without moving what is below.
fn status_height(scene: &Scene, model: &Model) -> Result<f64, String> {
    let mut tallest: f64 = 0.0;
    for status in [Status::Encoded, Status::Empty, Status::TooLong(MAX_INPUT_LEN + 1)] {
        let text = Model { status, ..model.clone() }.status_text();
        tallest = tallest.max(scene.measure_markdown_node(PANEL_WIDTH, &text).map_err(stringify)?.height);
    }
    Ok(tallest)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the whole scene for `model` and returns it with the text input node and the "Step" node that drives its
/// stepping toolbar.
///
/// # What this draws
///
/// **Down the right-hand column**: the text input node holding the text to encode, a status line, an explanation of the
/// current step, and the code table. The explanation and the status line are Markdown nodes, and the room for each is
/// kept at the height of its tallest text, so nothing below them moves as the walk steps. The forest of trees fills the
/// room to the left, from the top. In pass two the text input is replaced by two arrays of the input's bytes, with the
/// byte being packed marked. The text, the code table and the packing then stand in a row, so that the data reads from
/// left to right and the scene is wide rather than tall.
///
/// **The forest** starts as one tree of one node for each distinct byte, lightest first. Each step merges the two
/// lightest into a new tree whose weight is their sum, and the merge is ringed. The smaller of the two goes on the
/// left. Every link is labelled `0` for left or `1` for right. Ties go to the tree made first, so the same text always
/// gives the same tree. A leaf is coloured differently from a branch, and both from a link's label, since each holds a
/// different kind of value.
///
/// **The code table** has a row for each distinct byte, the commonest first, under a heading row of its own colour. Its
/// "Tree path" column is the route from the root to the byte's leaf, and its "Length" is the length of that path.
/// "Code" is the byte's *canonical* code, which is assigned after all the lengths are known. The canonical codes are
/// not the paths: they only keep each length. They are handed out shortest first, and by byte value within a length,
/// each being the previous code plus 1, shifted left by however many bits longer it is. A decoder can rebuild them from
/// the lengths alone, so no tree ever has to be sent.
///
/// As each canonical code is assigned, the route from the root to that byte's leaf is marked with the library's
/// secondary selection, so the tree path in the table can be followed through the tree.
///
/// The scene is sized for the finished tree, so it does not change size as it is stepped.
///
/// # Stepping through it
///
/// "Step" is a small array of one cell per walk position, placed far off-canvas, purely to drive a selection toolbar.
/// See [`walk`] for what each position shows.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or if any library call fails.
fn build_scene(model: &Model) -> Result<(Scene, Option<NodeId>, NodeId), String> {
    let trace = &model.encoding.trace;
    // Pass two follows the last position of pass one. Its table is the finished one, so `shown` stays at pass one's
    // end.
    let pass_two = model.position.checked_sub(steps(trace)).map(|index| packing(trace, index));
    let shown = walk::shown(trace, model.position);

    let document = demo_common::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 800.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title(
            if pass_two.is_some() {
                "Huffman Encoding: Phase 2 - Encode input text"
            } else {
                "Huffman Encoding: Phase 1 - Count byte frequency & build encoding tree"
            },
            SceneTitleOptions::default(),
        )
        .map_err(stringify)?;

    // The forest is sized for the finished tree, whose root is the tallest, and the leaves sit on one line below it.
    // Everything after is placed from that, so nothing moves as the trees merge.
    let spacing = Spacing {
        box_height: scene.measure_data_node(&branch_content(1)).map_err(stringify)?.height,
        bit_height: scene.measure_data_node(&bit_content(false)).map_err(stringify)?.height,
    };
    let (finished, _) = walk::layout(&trace.tree.nodes, &[trace.tree.root()]);
    let levels = finished.iter().map(|p| p.height).max().unwrap_or(0);
    let forest_top = CONTENT_TOP;
    let leaf_y = forest_top + levels as f64 * spacing.level();
    let leaves: Vec<usize> = (0..trace.frequencies.len()).collect();
    let (_, widest) = walk::layout(&trace.tree.nodes, &leaves);
    let table_width: f64 = COLUMNS.iter().map(|&(_, w)| w).sum();

    // The right-hand column, which is the same in both passes: the text, the status line, the explanation, and under
    // them the code table. The forest, or pass two's packing, takes the room to its left.
    let bits_width = bits_to_pack_width(&scene, trace)?;
    let packing_width = packing_width(&scene, bits_width)?;
    let column = if pass_two.is_some() { LEFT } else { LEFT + widest + TABLE_GAP };
    let table_left = column + PANEL_WIDTH + TABLE_GAP;
    let packing_left = table_left + table_width + TABLE_GAP;
    let (input, input_bottom) = match model.position.checked_sub(steps(trace)) {
        None => {
            let id = scene
                .add_text_input_node(
                    Point::new(column, CONTENT_TOP),
                    PANEL_WIDTH,
                    "Text to encode",
                    &model.text,
                    |_, id, text| typed(id, text),
                )
                .map_err(stringify)?;
            let rect = scene.node_rect(id).map_err(stringify)?;
            (Some(id), rect.origin.y + rect.size.height)
        },
        Some(index) => (None, draw_input_array(&scene, trace, index, column, CONTENT_TOP)?),
    };

    // The status line and the explanation each get the room kept for them.
    let status_top = input_bottom + GAP;
    scene
        .add_markdown_node(Point::new(column, status_top), PANEL_WIDTH, &model.status_text())
        .map_err(stringify)?;
    let explanation_top = status_top + status_height(&scene, model)? + GAP;
    scene
        .add_markdown_node(
            Point::new(column, explanation_top),
            PANEL_WIDTH,
            &pass_two.as_ref().map_or(&shown.caption, |p| &p.caption).join("\n\n"),
        )
        .map_err(stringify)?;
    let table_top = explanation_top + explanation_height(&scene, &model.encoding)? + V_GAP;

    // Pass two has no forest, and puts its packing in the forest's place instead.
    let (table_bottom, left_bottom) = match &pass_two {
        None => {
            let forest = draw_forest(&scene, trace, &shown, leaf_y, spacing)?;
            highlight_route(&scene, &forest, &shown.route)?;
            (
                draw_table(&scene, &shown, shown.focus_row, column, table_top)?,
                leaf_y + spacing.box_height,
            )
        },
        Some(packing) => {
            let focus_row = packing.symbol.and_then(|s| shown.rows.iter().position(|r| r.symbol == s));
            (
                draw_table(&scene, &shown, focus_row, table_left, CONTENT_TOP)?,
                draw_packing(&scene, packing, packing_left, forest_top, packing_width, bits_width)?,
            )
        },
    };
    let bottom = table_bottom.max(left_bottom).max(table_top - V_GAP);

    // "Step": see this function's own doc comment, "Stepping through it".
    let step_values: Vec<u16> = (1..=total_steps(trace)).map(|n| n as u16).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U16(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    demo_common::util::hide_step_driver(DIAGRAM_ID)?;

    // The widest the forest ever is is every byte in a row of its own, before any merge. Pass two reads from left to
    // right: the text, then the table, then the packing.
    let right = if pass_two.is_some() {
        packing_left + packing_width + NAMED_FRAME
    } else {
        column + table_width.max(PANEL_WIDTH)
    };
    fit_nested_size(&scene, DIAGRAM_ID, right, bottom, false)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok((scene, input, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `model`, wires its stepping toolbar, and replaces whatever scene was there. It keeps the zoom
/// and pan and the keyboard focus on a toolbar button that the old one had. Returns the new text input node, which pass
/// two does not have.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or if any library call fails.
fn build(model: Model) -> Result<Option<NodeId>, String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let was_pass_two =
        DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.model.position >= steps(&d.model.encoding.trace)));
    let focused = demo_common::focus::focused_selector(DIAGRAM_ID);

    let (model_position, pass_one_steps) = (model.position, steps(&model.encoding.trace));
    let (scene, input, step) = build_scene(&model)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(
            step,
            demo_common::util::step_toolbar_options().with_stride(SelectionStride::new(STRIDE, "8")),
            |_scene, _node, transition| {
                go(transition.to.unwrap_or(0));
            },
        )
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    scene.set_selection(step, Selection::Cell(model.position)).map_err(stringify)?;

    DEMO.with_borrow_mut(|slot| *slot = Some(Demo { scene, model }));
    if let Some(selector) = focused {
        demo_common::focus::focus(&selector);
    }
    // The two passes are different sizes, so the window is fitted again when the walk crosses from one to the other.
    // Not at every step, since a window that kept changing size would be hard to use.
    if was_pass_two.is_some_and(|was| was != (model_position >= pass_one_steps)) {
        demo_common::util::fit_window_to_stage(DIAGRAM_ID)?;
    }
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`. A failure is shown in the window's own error banner, and logged,
/// since the toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(position: usize) {
    let model = DEMO.with_borrow(|slot| slot.as_ref().map(|d| Model { position, ..d.model.clone() }));
    report(
        model
            .ok_or_else(|| "the diagram is not built".to_owned())
            .and_then(|m| build(m).map(|_| ())),
        &format!("This step failed (position {position})"),
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box now holds `text`. Draws the scene again for it, then gives the new text box the keyboard focus, with
/// the caret where it was, so that typing carries on in it.
fn typed(id: NodeId, text: &str) {
    let caret = DEMO.with_borrow(|slot| slot.as_ref().and_then(|d| d.scene.text_input_caret(id).ok()));
    let model = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.model.with_text(text)));
    let result = model.ok_or_else(|| "the diagram is not built".to_owned()).and_then(|model| {
        let input = build(model)?;
        DEMO.with_borrow(|slot| match (slot, input) {
            (Some(demo), Some(input)) => demo.scene.focus_text_input(input, caret).map_err(stringify),
            _ => Ok(()),
        })
    });
    report(result, "Could not draw the text");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a failure in the window's own error banner, or clears the banner after a success.
fn report(result: Result<(), String>, what: &str) {
    match result {
        Ok(()) => demo_common::util::clear_error(ERROR_ID),
        Err(e) => demo_common::util::report_error(ERROR_ID, &format!("{what}: {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Huffman scene at its first step, wires its stepping toolbar, and resizes the window it runs in to fit the
/// scene. Runs in `huffman-window.html`, not in the gallery.
///
/// The walk has two passes. Pass one demonstrates:
///
/// 1. Counting the bytes, then building the tree by repeatedly merging the two lightest trees. A merge is one step, and
///    every link is labelled `0` or `1`.
/// 2. Reading each byte's code length from the depth of its leaf. This is the last thing the tree is used for.
/// 3. Assigning canonical codes from those lengths, one byte at a time, with the route to the byte marked in the tree.
///    The table keeps the tree's own paths beside the codes, so it shows that the two differ, with the lengths
///    unchanged.
/// 4. Editing the text in the scene's own text input node, which draws everything again from the first step.
///
/// Pass two follows the last canonical code, and demonstrates:
///
/// 5. Packing the codes into bytes, one input byte at a time. The text box is replaced by a hex dump of the input's
///    bytes, with the byte being packed marked. Beside it are the pending bits with the new code appended, the bits
///    left pending, and the bytes written so far in hexadecimal. A byte is written as soon as eight bits are ready, and
///    the step marks the bytes it wrote.
/// 6. The flush, which pads the last byte with zero bits. It shows the result's size against the input's, with the
///    compression ratio.
///
/// Crossing from one pass to the other fits the window to the scene again, since the two are different sizes.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#huffman-diagram-stage`, or if any library call fails.
pub(crate) fn build_huffman_window() -> Result<(), String> {
    let trace = algorithm::trace(DEFAULT_TEXT.as_bytes()).ok_or("the default text cannot be encoded")?;
    build(Model::new(DEFAULT_TEXT, trace))?;
    demo_common::util::fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Huffman scene is larger than most demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `huffman-window.html`, which runs [`build_huffman_window`] there. A named
/// window is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#huffman-open`, or if a listener could not be attached to it.
pub(crate) fn build_huffman_demo() -> Result<(), String> {
    demo_common::util::wire_open_window(&demo_common::util::document()?, "huffman-open", WINDOW_PAGE, WINDOW_NAME)
}

/// The page [`build_huffman_demo`]'s button opens.
const WINDOW_PAGE: &str = "huffman-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-huffman";
