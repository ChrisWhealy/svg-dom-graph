//! `panel-huffman` / `#huffman-diagram`: pass one of Huffman encoding, which counts the bytes of a text, builds the code
//! tree and gives every byte its canonical code. See [`build_scene`]'s own doc comment for what the diagram shows. The
//! scene is large, so it runs in a window of its own. The gallery's panel keeps the description and opens that window:
//! see [`build_huffman_demo`].
//!
//! Huffman encoding takes two passes over the text. The first, shown here, counts how often each byte occurs and builds
//! the tree. The second replaces each byte with its code and packs the odd-length bit sequences into whole bytes. This
//! file draws pass one only, and the second pass and the decoder come later.
//!
//! # One scene per step
//!
//! A node or edge cannot be removed once drawn, and every step of pass one changes which nodes exist. So each step
//! draws the whole scene again, from [`walk::shown`] alone, and keeps the zoom and pan and the keyboard focus the old
//! one had. Stepping backwards is the same work as stepping forwards.
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
//! calculation: the counts, the merges, the tree's paths and the canonical codes. Symbols are bytes, so a multi-byte
//! character is counted as its separate bytes, as real Huffman encoders do.

mod algorithm;
mod walk;

use crate::source_file;
use crate::util::{CONTENT_TOP, ensure_svg_in, fit_nested_size, required_element, stringify};
use algorithm::{MAX_INPUT_LEN, Trace};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        BoolFormat, ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeOptions, NodeValues, Scene,
        SceneTitleOptions, Selection, Side, ToolbarOptions,
    },
};
use walk::{Row, Shown, Spacing, leaf_label, steps, symbol_label};
use wasm_bindgen::JsCast;

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
/// The width of the text box and of the two Markdown nodes. It is wide enough for their longest line.
const PANEL_WIDTH: f64 = 1000.0;
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text that has been encoded, and what is worked out once from it and kept.
struct Encoding {
    trace: Trace,
    /// The height the explanation needs at its tallest step, once it has been measured. Every step reserves that much
    /// room, so nothing below the explanation moves as the walk steps.
    explanation_height: Cell<Option<f64>>,
}

/// What the status line says.
#[derive(Clone)]
enum Status {
    /// The text was encoded.
    Encoded,
    Empty,
    /// The text is this many bytes, which is more than [`MAX_INPUT_LEN`].
    TooLong(usize),
}

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

    /// The model after the text box changes to `text`. A text that can be encoded starts the walk again. One that cannot
    /// leaves the diagram as it was and only changes what the status line says.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A forced south-to-north connector, for a link that runs down from a parent to its child.
fn downward() -> ConnectorOptions {
    ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North))
}

/// A leaf's content: its byte and its count as one run of text.
fn leaf_content(symbol: u8, weight: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(leaf_label(symbol, weight).into_bytes()), DataFormat::PlainText)
}

/// A branch's content: its weight. The weight is at most [`MAX_INPUT_LEN`], so it fits a `u16`.
fn branch_content(weight: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U16(vec![weight as u16]), DataFormat::Decimal)
}

/// The content of the small box that labels a link: a bit, written `0` or `1`.
fn bit_content(bit: bool) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(vec![bit]), DataFormat::Bool(BoolFormat::Bit))
}

/// A table cell's text, or a dash for one the walk has not reached.
fn cell(text: Option<String>) -> String {
    text.unwrap_or_else(|| "–".to_owned())
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes of the forest, so that what the step highlights can be found again.
struct Forest {
    /// Each tree node's own box, by its index in the tree's arena. `None` for a node that is not in the forest yet.
    boxes: Vec<Option<NodeId>>,
    /// The box that labels the link above each tree node, by the index of the node below it. `None` for a root.
    bits: Vec<Option<NodeId>>,
}

/// Draws the forest `shown` names, with its leaves on the line `leaf_y`, and returns its nodes.
///
/// A leaf is a data node of its byte and count, and a branch a data node of its weight. A branch's two links are each
/// drawn through a small box that reads `0` for the left child and `1` for the right, so the paths in the code table can
/// be read off the tree. Each box is centred on the middle the layout gives it. The branch the last merge made, and the
/// two trees it joined, are ringed.
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

/// Marks the route from the root to the leaf of the byte that has just been given its canonical code, with the library's
/// secondary selection: every box on it, and every link's label. A node with no box in the forest, which is only
/// possible for a route that names a node not drawn yet, is left alone.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the code table with its top left at `(left, top)`, one row for each distinct byte, the commonest first, and returns the
/// lowest edge. The first row holds the headings, and has a fill of its own. The row the last step wrote to is ringed.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn draw_table(scene: &Scene, shown: &Shown, left: f64, top: f64) -> Result<f64, String> {
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
            if index.is_some() && index == shown.focus_row {
                scene.set_focus(id, true).map_err(stringify)?;
            }
            x += width;
        }
        y += ROW_HEIGHT;
    }
    Ok(y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The height the explanation needs at its tallest step, which every step reserves.
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
    for position in 0..steps(&encoding.trace) {
        let text = walk::shown(&encoding.trace, position).caption.join("\n\n");
        tallest = tallest.max(scene.measure_markdown_node(PANEL_WIDTH, &text).map_err(stringify)?.height);
    }
    encoding.explanation_height.set(Some(tallest));
    Ok(tallest)
}

/// The height the status line needs at its tallest, so that it can say any of its things without moving what is below.
fn status_height(scene: &Scene, model: &Model) -> Result<f64, String> {
    let mut tallest: f64 = 0.0;
    for status in [Status::Encoded, Status::Empty, Status::TooLong(MAX_INPUT_LEN + 1)] {
        let text = Model { status, ..model.clone() }.status_text();
        tallest = tallest.max(scene.measure_markdown_node(PANEL_WIDTH, &text).map_err(stringify)?.height);
    }
    Ok(tallest)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the whole scene for `model` and returns it with the text input node and the "Step" node that drives its
/// stepping toolbar.
///
/// # What this draws
///
/// From the top: the text input node holding the text to encode, a status line, an explanation of the current step, the
/// forest of trees, and the code table. The explanation and the status line are Markdown nodes, and the room for each
/// is kept at the height of its tallest text, so nothing below them moves as the walk steps.
///
/// **The forest** starts as one tree of one node for each distinct byte, lightest first. Each step merges the two
/// lightest into a new tree whose weight is their sum, and the merge is ringed. The smaller of the two goes on the left.
/// Every link is labelled `0` for left or `1` for right. Ties go to the tree made first, so the same text always gives
/// the same tree. A leaf is coloured differently from a branch, and both from a link's label, since each holds a
/// different kind of value.
///
/// **The code table** has a row for each distinct byte, the commonest first, under a heading row of its own colour. Its
/// "Tree path" column is the route from the root to the byte's leaf, and its "Length" is the length of that path.
/// "Code" is the byte's *canonical* code, which is assigned after all the lengths are known. The canonical codes are not
/// the paths: they only keep each length. They are handed out shortest first, and by byte value within a length, each
/// being the previous code plus 1, shifted left by however many bits longer it is. A decoder can rebuild them from the
/// lengths alone, so no tree ever has to be sent.
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
fn build_scene(model: &Model) -> Result<(Scene, NodeId, NodeId), String> {
    let trace = &model.encoding.trace;
    let shown = walk::shown(trace, model.position);

    let document = crate::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 800.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Huffman encoding: pass one", SceneTitleOptions::default())
        .map_err(stringify)?;

    // The text box, then the status line and the explanation each in the room kept for them.
    let input = scene
        .add_text_input_node(
            Point::new(LEFT, CONTENT_TOP),
            PANEL_WIDTH,
            "Text to encode",
            &model.text,
            |_, id, text| typed(id, text),
        )
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input).map_err(stringify)?;

    let status_top = input_rect.origin.y + input_rect.size.height + GAP;
    scene
        .add_markdown_node(Point::new(LEFT, status_top), PANEL_WIDTH, &model.status_text())
        .map_err(stringify)?;
    let explanation_top = status_top + status_height(&scene, model)? + GAP;
    scene
        .add_markdown_node(Point::new(LEFT, explanation_top), PANEL_WIDTH, &shown.caption.join("\n\n"))
        .map_err(stringify)?;

    // The forest is sized for the finished tree, whose root is the tallest, and the leaves sit on one line below it.
    // Everything after is placed from that, so nothing moves as the trees merge.
    let spacing = Spacing {
        box_height: scene.measure_data_node(&branch_content(1)).map_err(stringify)?.height,
        bit_height: scene.measure_data_node(&bit_content(false)).map_err(stringify)?.height,
    };
    let (finished, _) = walk::layout(&trace.tree.nodes, &[trace.tree.root()]);
    let levels = finished.iter().map(|p| p.height).max().unwrap_or(0);
    let forest_top = explanation_top + explanation_height(&scene, &model.encoding)? + V_GAP;
    let leaf_y = forest_top + levels as f64 * spacing.level();
    let leaves: Vec<usize> = (0..trace.frequencies.len()).collect();
    let (_, widest) = walk::layout(&trace.tree.nodes, &leaves);

    let forest = draw_forest(&scene, trace, &shown, leaf_y, spacing)?;
    highlight_route(&scene, &forest, &shown.route)?;
    // The table stands to the right of the forest, level with its top. That makes the scene wide rather than tall, so it
    // is scaled down less to fit a window.
    let table_left = LEFT + widest + TABLE_GAP;
    let table_bottom = draw_table(&scene, &shown, table_left, forest_top)?;
    let bottom = table_bottom.max(leaf_y + spacing.box_height);

    // "Step": see this function's own doc comment, "Stepping through it".
    let step_values: Vec<u8> = (1..=steps(trace).min(usize::from(u8::MAX)) as u8).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U8(step_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    crate::util::hide_step_driver(DIAGRAM_ID)?;

    // The widest the forest ever is is every byte in a row of its own, before any merge.
    let right = table_left + COLUMNS.iter().map(|&(_, w)| w).sum::<f64>();
    fit_nested_size(&scene, DIAGRAM_ID, right, bottom, false)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok((scene, input, step))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `model`, wires its stepping toolbar, and replaces whatever scene was there. It keeps the zoom and
/// pan and the keyboard focus on a toolbar button that the old one had. Returns the new text input node.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or if any library call fails.
fn build(model: Model) -> Result<NodeId, String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let focused = focused_button();

    let (scene, input, step) = build_scene(&model)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(step, crate::util::step_toolbar_options(), |_scene, _node, transition| {
            go(transition.to.unwrap_or(0));
        })
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets "Step" to unstarted as its first committed act. Reapply the position.
    scene.set_selection(step, Selection::Cell(model.position)).map_err(stringify)?;

    DEMO.with_borrow_mut(|slot| *slot = Some(Demo { scene, model }));
    if let Some(label) = focused {
        focus_button(&label);
    }
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`. A failure is shown in the window's own error banner, and logged, since
/// the toolbar's own callback has nowhere to return it to. The banner clears again at the next step that works.
fn go(position: usize) {
    let model = DEMO.with_borrow(|slot| slot.as_ref().map(|d| Model { position, ..d.model.clone() }));
    report(
        model
            .ok_or_else(|| "the diagram is not built".to_owned())
            .and_then(|m| build(m).map(|_| ())),
        &format!("This step failed (position {position})"),
    );
}

/// The text box now holds `text`. Draws the scene again for it, then gives the new text box the keyboard focus, with the
/// caret where it was, so that typing carries on in it.
fn typed(id: NodeId, text: &str) {
    let caret = DEMO.with_borrow(|slot| slot.as_ref().and_then(|d| d.scene.text_input_caret(id).ok()));
    let model = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.model.with_text(text)));
    let result = model.ok_or_else(|| "the diagram is not built".to_owned()).and_then(|model| {
        let input = build(model)?;
        DEMO.with_borrow(|slot| match slot {
            Some(demo) => demo.scene.focus_text_input(input, caret).map_err(stringify),
            None => Ok(()),
        })
    });
    report(result, "Could not draw the text");
}

/// Shows a failure in the window's own error banner, or clears the banner after a success.
fn report(result: Result<(), String>, what: &str) {
    match result {
        Ok(()) => crate::util::clear_error(ERROR_ID),
        Err(e) => crate::util::report_error(ERROR_ID, &format!("{what}: {e}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    let Ok(document) = crate::util::document() else { return };
    let selector = format!("#{DIAGRAM_ID} g[aria-label=\"{label}\"]");
    if let Ok(Some(button)) = document.query_selector(&selector) {
        if let Ok(button) = button.dyn_into::<web_sys::SvgElement>() {
            let _ = button.focus();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Huffman scene at its first step, wires its stepping toolbar, and resizes the window it runs in to fit the
/// scene. Runs in `huffman-window.html`, not in the gallery.
///
/// Demonstrates:
///
/// 1. Counting the bytes, then building the tree by repeatedly merging the two lightest trees. A merge is one step, and
///    every link is labelled `0` or `1`.
/// 2. Reading each byte's code length from the depth of its leaf. This is the last thing the tree is used for.
/// 3. Assigning canonical codes from those lengths, one byte at a time, with the route to the byte marked in the tree.
///    The table keeps the tree's own paths beside the codes, so it shows that the two differ, with the lengths unchanged.
/// 4. Editing the text in the scene's own text input node, which draws everything again from the first step.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#huffman-diagram-stage`, or if any library call fails.
pub(crate) fn build_huffman_window() -> Result<(), String> {
    let trace = algorithm::trace(DEFAULT_TEXT.as_bytes()).ok_or("the default text cannot be encoded")?;
    build(Model::new(DEFAULT_TEXT, trace))?;
    crate::util::fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Huffman scene is larger than most demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `huffman-window.html`, which runs [`build_huffman_window`] there. A named
/// window is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#huffman-open`, or if a listener could not be attached to it.
pub(crate) fn build_huffman_demo() -> Result<(), String> {
    crate::util::wire_open_window(&crate::util::document()?, "huffman-open", WINDOW_PAGE, WINDOW_NAME)
}

/// The page [`build_huffman_demo`]'s button opens.
const WINDOW_PAGE: &str = "huffman-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-huffman";
