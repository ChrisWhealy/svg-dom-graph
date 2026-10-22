//! `panel-chacha20` / `#chacha20-diagram`: all 20 rounds of ChaCha20, one quarter round at a time, drawn as the four
//! columns `a`, `b`, `c` and `d` that the RFC's own figures use. See [`build_chacha20_window`]'s own doc comment for
//! what the diagram shows. The scene is large, so it runs in a window of its own. The gallery's panel keeps the
//! description and opens that window: see [`build_chacha20_demo`].
//!
//! # Layout
//!
//! The state before the quarter round is at the top. Its four words feed the four columns, and the four words that come
//! out feed the state below. Each of the four words runs down a column of its own. A step that updates a word puts its
//! operator node in that word's column. The word's previous value is one operand. A step that adds or XORs another word
//! draws that word's latest value as its second operand, so the connector crosses to the other column. A rotation has
//! only the one operand.
//!
//! # The walk
//!
//! Every quarter round has the same layout, so one diagram serves all 80 of them. Stepping only writes new values and
//! marks into the nodes already drawn, via [`apply`]. What a step shows comes from [`walk::shown`], a pure function of
//! the position alone. Stepping backwards is therefore the same write as stepping forwards, and so is a restart.
//!
//! The keystream block, which the 20 rounds lead to, has a layout of its own. Crossing to or from it draws the diagram
//! again, and keeps the zoom and pan and the keyboard focus the old one had.
//!
//! # What is real
//!
//! All of it. [`algorithm::rounds`] runs the 20 rounds in plain Rust, and every value on screen is one of its results.
//! The first state is the initial state of RFC 8439, section 2.3.2. The tests of [`algorithm`] check the quarter round,
//! the block function and the encryption against the values the RFC publishes.
//!
//! # What is not covered
//!
//! The XOR of the keystream with a message. The tests of [`algorithm`] cover it.

mod algorithm;
mod walk;

use algorithm::{Operation, Rounds, STATE_WORDS, STEPS, Word};
use demo_common::util::{ensure_svg_in, stringify, view_box_rect};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        ArithmeticOperator, BinaryOperator, ConnectorOptions, DataFormat, DataNodeContent, DragOptions, EdgeAnchors,
        GridLayout, LabellingStyle, NodeOptions, NodeValues, Scene, SceneTitleOptions, Selection, SelectionStride,
        Side, ToolbarOptions, UnaryOperator,
    },
};
use walk::{POSITIONS, STEPS_PER_QUARTER_ROUND, Shown, Stage};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind [`build_chacha20_demo`]: the rounds, the walk through them and the tests of both.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("chacha20/algorithm/mod.rs", "algorithm/mod.rs"),
    crate::source_file!("chacha20/algorithm/unit_tests.rs", "algorithm/unit_tests.rs"),
    crate::source_file!("chacha20/walk/mod.rs", "walk/mod.rs"),
    crate::source_file!("chacha20/walk/unit_tests.rs", "walk/unit_tests.rs"),
    crate::source_file!("chacha20/mod.rs", "mod.rs"),
];

/// The page the gallery's button opens, and the name of the window it opens in. A named window is reused.
const WINDOW_PAGE: &str = "chacha20-window.html";
const WINDOW_NAME: &str = "svg-dom-graph-chacha20";
/// The paragraph in that page which reports a failure with nowhere else to go.
const ERROR_BANNER: &str = "chacha20-window-error";

/// The `<svg>` the diagram is drawn in, and the stage that holds it.
const DIAGRAM_ID: &str = "chacha20-diagram";
const STAGE_ID: &str = "chacha20-diagram-stage";

/// The distance between the left-hand edge of one column and the next.
const COLUMN_PITCH: f64 = 250.0;
/// The distance between one step and the next, measured down a column.
const STEP_PITCH: f64 = 90.0;
const X_MARGIN: f64 = 40.0;
/// Below the scene's own title and its stepping toolbar, which sit at the top.
const Y_MARGIN: f64 = demo_common::util::CONTENT_TOP;
/// Between a state and the row of four words it feeds, or that feeds it.
const STATE_GAP: f64 = 60.0;
/// Between the four input words and the first step.
const INPUT_GAP: f64 = 40.0;
/// The first size of the `<svg>`, before it is fitted to what is drawn.
const FIRST_SIZE: (f64, f64) = (1900.0, 1400.0);

/// The row each of the 12 steps sits on. Steps that hand a value straight on share a row. The `XOR` after the first
/// `ADD` shares its row, and so do the `ADD` and `XOR` after each rotation of `d`. Every other step has a row of its
/// own.
const STEP_ROWS: [usize; 12] = [0, 0, 1, 1, 1, 2, 3, 3, 4, 4, 4, 5];
/// How many rows the steps take.
const ROWS: usize = 6;

/// The fixing points of an `ADD` that feeds an `XOR` on its own row, while its other operand is on an earlier row. Its
/// east side then takes two connectors: the operand coming from `b` and the sum going to the `XOR`. Three points keep
/// both clear of each other, and put the connector from the node above on the middle of its north side.
const SUM_ANCHORS: EdgeAnchors = EdgeAnchors(3);

/// The 12 steps as the RFC writes them, one label for each row of the list of steps.
const STEP_LABELS: &[&str] = &[
    "a += b", "d ^= a", "d <<<= 16", "c += d", "b ^= c", "b <<<= 12", "a += b", "d ^= a", "d <<<= 8", "c += d",
    "b ^= c", "b <<<= 7",
];

/// The four rows of a state, named for the words `a`, `b`, `c` and `d` that a column or diagonal of it holds.
const ROW_LABELS: &[&str] = &["a", "b", "c", "d"];

/// Where along the edge of a state each of the four columns' connector sits. Spread in order, so none crosses another.
const FAN: [f64; 4] = [0.125, 0.375, 0.625, 0.875];

/// What each toolbar stride moves by: one round is four quarter rounds, each of 12 steps.
const ROUND_STRIDE: usize = 4 * STEPS_PER_QUARTER_ROUND;

/// The key of RFC 8439's examples: the bytes `00 01 02 ... 1f`.
fn rfc_key() -> [u8; algorithm::KEY_LEN] {
    std::array::from_fn(|i| i as u8)
}

/// The nonce of the block function's example in RFC 8439, section 2.3.2.
const NONCE: [u8; algorithm::NONCE_LEN] = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];

/// The block counter of that example.
const COUNTER: u32 = 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn hex_word(value: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(vec![value]), DataFormat::Hexadecimal)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A state: 16 words in a 4 by 4 matrix, its rows labelled `a` to `d`.
fn state_content(words: &[u32; STATE_WORDS]) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(words.to_vec()), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(4))
        .with_labelling_style(LabellingStyle::Text(ROW_LABELS))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes of the quarter round diagram, which every quarter round writes its values into.
struct Nodes {
    /// The state before the quarter round.
    before: NodeId,
    /// The state after it.
    after: NodeId,
    /// The list of the 12 steps.
    list: NodeId,
    /// The four words going in, as `a`, `b`, `c` and `d`.
    inputs: Vec<NodeId>,
    /// The operator node of each of the 12 steps.
    steps: Vec<NodeId>,
    /// The four words coming out, as `a`, `b`, `c` and `d`.
    outputs: Vec<NodeId>,
    /// For each step, the nodes in use while the walk stands at it: the step's own node and the operands it reads.
    in_use: Vec<Vec<NodeId>>,
    /// Every node `in_use` can name.
    every_node: Vec<NodeId>,
}

impl Nodes {
    /// Every node the viewer may drag. The step list and the operators' operands are all among them.
    fn movable(&self) -> Vec<NodeId> {
        let mut all = vec![self.before, self.after, self.list];
        all.extend(&self.inputs);
        all.extend(&self.steps);
        all.extend(&self.outputs);
        all
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the walk needs to move the diagram on one step.
struct Demo {
    scene: Scene,
    /// The quarter round diagram's nodes. `None` while the keystream block is on show.
    nodes: Option<Nodes>,
    drawn: Stage,
    rounds: Rc<Rounds>,
    /// The quarter round the scene's title was last written for.
    titled: Option<usize>,
}

thread_local! {
    // Keeps this Scene's only strong handle alive for the page's lifetime. Every listener this crate installs holds
    // only a `Weak` reference to it, so the toolbars stop responding the moment this is dropped. See `tree::SCENE`.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the quarter round diagram, with nothing reached yet, and returns its nodes and the node that drives its
/// toolbar. See [`build_chacha20_window`] for what it shows.
///
/// Every operator node is drawn with a zero result, which [`apply`] replaces as the walk reaches it. The layout does
/// not depend on the values, since a hexadecimal word is always as wide as any other.
fn draw_quarter_round(scene: &Scene, rounds: &Rounds) -> Result<(Nodes, NodeId), String> {
    let column_x = |word: Word| X_MARGIN + COLUMN_PITCH * word.index() as f64;
    let first = &rounds.stages[0];
    let state = state_content(&first.before);

    // The state is centred over the four columns, and the four words it feeds sit below it.
    let state_size = scene.measure_named_data_node("state before", &state).map_err(stringify)?;
    let word_size = scene.measure_named_data_node("a", &hex_word(0)).map_err(stringify)?;
    let columns_centre = X_MARGIN + (COLUMN_PITCH * 3.0 + word_size.width) / 2.0;
    let state_x = columns_centre - state_size.width / 2.0;
    let inputs_top = Y_MARGIN + state_size.height + STATE_GAP;
    let steps_top = inputs_top + word_size.height + INPUT_GAP;
    let outputs_top = steps_top + STEP_PITCH * ROWS as f64;
    let after_top = outputs_top + word_size.height + STATE_GAP;

    let named = |name: &str, content: DataNodeContent, at: Point| -> Result<NodeId, String> {
        scene.add_named_data_node(at, name, content).map_err(stringify)
    };
    let fan = |from: NodeId, to: NodeId, index: usize, outward: bool| -> Result<(), String> {
        let options = ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North));
        let options = if outward {
            options.with_from_position(Some(FAN[index]))
        } else {
            options.with_to_position(Some(FAN[index]))
        };
        scene.add_edge_with(from, to, options).map_err(stringify)?;
        Ok(())
    };

    let before = named("state before", state.clone(), Point::new(state_x, Y_MARGIN))?;

    // The latest node in each column, indexed like the quarter round's own input and output. It starts as the input.
    let mut latest: Vec<NodeId> = Vec::with_capacity(Word::ALL.len());
    for word in Word::ALL {
        let node = named(word.name(), hex_word(0), Point::new(column_x(word), inputs_top))?;
        fan(before, node, word.index(), true)?;
        latest.push(node);
    }
    let inputs = latest.clone();

    // For each step, the nodes the walk shows as in use while it stands there: the step's own node and its operands.
    // The four input nodes are listed too, since a step can read one of them.
    let mut in_use: Vec<Vec<NodeId>> = Vec::with_capacity(STEPS.len());
    let mut every_node: Vec<NodeId> = latest.clone();
    let mut steps = Vec::with_capacity(STEPS.len());
    // The row each word was last updated on. An input has none.
    let mut latest_row: [Option<usize>; 4] = [None; 4];

    for (index, &(word, operation)) in STEPS.iter().enumerate() {
        let row = STEP_ROWS[index];
        let at = Point::new(column_x(word), steps_top + STEP_PITCH * row as f64);
        let own = latest[word.index()];
        let mut operands = vec![own];

        let node = match operation {
            Operation::Add(other) => {
                operands.push(latest[other.index()]);
                // A sum that feeds an `XOR` on its own row leaves by the same side as the operand from `b` arrives,
                // unless that operand is on the sum's own row too, and so arrives on the other side.
                let feeds_same_row = STEP_ROWS.get(index + 1) == Some(&row);
                let shares_row = latest_row[other.index()] == Some(row);
                let anchors = (feeds_same_row && !shares_row).then_some(SUM_ANCHORS);
                scene.add_arithmetic_operator_node_with(
                    at,
                    ArithmeticOperator::Add,
                    (own, latest[other.index()]),
                    hex_word(0),
                    NodeOptions::default().with_edge_anchors(anchors),
                )
            },
            Operation::Xor(other) => {
                operands.push(latest[other.index()]);
                scene.add_binary_operator_node(at, BinaryOperator::Xor, (own, latest[other.index()]), hex_word(0))
            },
            Operation::RotateLeft(bits) => {
                scene.add_unary_operator_node(at, UnaryOperator::RotateLeft(bits), own, hex_word(0))
            },
        }
        .map_err(stringify)?;
        latest[word.index()] = node;
        latest_row[word.index()] = Some(row);

        operands.push(node);
        in_use.push(operands);
        every_node.push(node);
        steps.push(node);
    }

    // The state after: what the four output words go into.
    let after = named("state after", state_content(&first.after), Point::new(state_x, after_top))?;
    let mut outputs = Vec::with_capacity(Word::ALL.len());
    for word in Word::ALL {
        let output = named(word.name(), hex_word(0), Point::new(column_x(word), outputs_top))?;
        scene.add_edge(latest[word.index()], output).map_err(stringify)?;
        fan(output, after, word.index(), false)?;
        outputs.push(output);
    }

    // The list of steps, one row for each, showing the value the step produced.
    let list_content = DataNodeContent::new(NodeValues::U32(vec![0; STEPS_PER_QUARTER_ROUND]), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(STEP_LABELS));
    let list = named(
        "steps",
        list_content,
        Point::new(X_MARGIN + COLUMN_PITCH * Word::ALL.len() as f64, steps_top),
    )?;

    let after_rect = scene.node_rect(after).map_err(stringify)?;
    let list_rect = scene.node_rect(list).map_err(stringify)?;
    demo_common::util::fit_nested_size(
        scene,
        DIAGRAM_ID,
        list_rect.origin.x + list_rect.size.width,
        after_rect.origin.y + after_rect.size.height,
        false,
    )?;

    let nodes = Nodes {
        before,
        after,
        list,
        inputs,
        steps,
        outputs,
        in_use,
        every_node,
    };
    Ok((nodes, driver(scene)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the keystream block diagram: the state after 20 rounds and the initial state, which are added word by word to
/// make the block. Returns its three data nodes, which the caller makes draggable, and the node that drives its
/// toolbar.
fn draw_keystream(scene: &Scene, rounds: &Rounds) -> Result<(Vec<NodeId>, NodeId), String> {
    let named = |name: &str, content: DataNodeContent, at: Point| -> Result<NodeId, String> {
        scene.add_named_data_node(at, name, content).map_err(stringify)
    };
    let mixed = state_content(&rounds.mixed);
    let size = scene
        .measure_named_data_node("state after 20 rounds", &mixed)
        .map_err(stringify)?;
    let centre = X_MARGIN + (COLUMN_PITCH * 3.0 + size.width / 2.0) / 2.0 + size.width / 2.0;
    let gap = 60.0;

    let initial = named(
        "initial state",
        state_content(&rounds.initial),
        Point::new(centre - size.width - gap / 2.0, Y_MARGIN),
    )?;
    let after = named("state after 20 rounds", mixed, Point::new(centre + gap / 2.0, Y_MARGIN))?;
    let block = named(
        "keystream block: the two states above, added word by word",
        state_content(&rounds.keystream),
        Point::new(centre - size.width / 2.0, Y_MARGIN + size.height + 2.0 * STATE_GAP),
    )?;
    let edge = ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North));
    scene
        .add_edge_with(initial, block, edge.with_to_position(Some(0.25)))
        .map_err(stringify)?;
    scene
        .add_edge_with(after, block, edge.with_to_position(Some(0.75)))
        .map_err(stringify)?;

    let block_rect = scene.node_rect(block).map_err(stringify)?;
    let after_rect = scene.node_rect(after).map_err(stringify)?;
    demo_common::util::fit_nested_size(
        scene,
        DIAGRAM_ID,
        // Clear of the zoom toolbar, which sits at the right-hand edge.
        after_rect.origin.x + after_rect.size.width + 40.0,
        block_rect.origin.y + block_rect.size.height,
        false,
    )?;
    Ok((vec![initial, after, block], driver(scene)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The node whose cells the stepping toolbar walks along, one cell for each position. It is far off-canvas, since it
/// only drives the toolbar, and it is hidden from assistive technology. See [`demo_common::util::hide_step_driver`].
fn driver(scene: &Scene) -> Result<NodeId, String> {
    let cells: Vec<u16> = (1..=POSITIONS as u16).collect();
    let node = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U16(cells), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    demo_common::util::hide_step_driver(DIAGRAM_ID)?;
    Ok(node)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `shown` into the quarter round diagram: every value, each mark for a value not yet reached, and the highlight
/// on the step in use and the words it reads.
///
/// It writes everything each time and leaves the library to skip what has not changed, so it needs no record of what
/// was written before.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
fn apply(scene: &Scene, nodes: &Nodes, shown: &Shown) -> Result<(), String> {
    let words = |values: &[u32]| NodeValues::U32(values.to_vec());
    let unreached = |reached: bool| if reached { Vec::new() } else { vec![0] };
    let group = {
        let mut cells = shown.group.to_vec();
        cells.sort_unstable();
        cells
    };

    scene.set_data_values(nodes.before, words(&shown.before)).map_err(stringify)?;
    scene.set_secondary_selection(nodes.before, &group).map_err(stringify)?;
    scene.set_data_values(nodes.after, words(&shown.after)).map_err(stringify)?;
    scene.set_secondary_selection(nodes.after, &group).map_err(stringify)?;
    scene
        .set_unreached_cells(nodes.after, &shown.unreached_after())
        .map_err(stringify)?;

    for (node, value) in nodes.inputs.iter().zip(shown.inputs) {
        scene.set_data_values(*node, words(&[value])).map_err(stringify)?;
        scene
            .set_unreached_cells(*node, &unreached(shown.inputs_reached()))
            .map_err(stringify)?;
    }
    for (index, (node, value)) in nodes.steps.iter().zip(shown.steps).enumerate() {
        scene.set_data_values(*node, words(&[value])).map_err(stringify)?;
        scene
            .set_unreached_cells(*node, &unreached(index < shown.reached))
            .map_err(stringify)?;
    }
    for (node, value) in nodes.outputs.iter().zip(shown.outputs) {
        scene.set_data_values(*node, words(&[value])).map_err(stringify)?;
        scene
            .set_unreached_cells(*node, &unreached(shown.outputs_reached()))
            .map_err(stringify)?;
    }

    scene.set_data_values(nodes.list, words(&shown.steps)).map_err(stringify)?;
    scene
        .set_unreached_cells(nodes.list, &shown.unreached_steps())
        .map_err(stringify)?;
    scene
        .set_selection(nodes.list, shown.current.map_or(Selection::None, Selection::Cell))
        .map_err(stringify)?;

    // The step the walk stands at, and the values it works on, share the secondary highlight.
    let current = shown.current.and_then(|step| nodes.in_use.get(step));
    for &candidate in &nodes.every_node {
        let lit = current.is_some_and(|used| used.contains(&candidate));
        scene
            .set_secondary_selection(candidate, if lit { &[0] } else { &[] })
            .map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the diagram `stage` calls for, shows `position` in it, and wires its stepping toolbar. It replaces whatever
/// diagram was there, and keeps the zoom and pan and the keyboard focus the old one had.
///
/// # Errors
///
/// Returns `Err` if the window's own stage is missing, or if any library call fails.
fn build(stage: Stage, position: Option<usize>, rounds: Rc<Rounds>) -> Result<(), String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|demo| demo.scene.view()));
    let focused = demo_common::focus::focused_selector(DIAGRAM_ID);

    let document = demo_common::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(FIRST_SIZE.0, FIRST_SIZE.1))?;
    demo_common::util::required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let (nodes, driver, movable) = match stage {
        Stage::QuarterRound => {
            let (nodes, driver) = draw_quarter_round(&scene, &rounds)?;
            let movable = nodes.movable();
            (Some(nodes), driver, movable)
        },
        Stage::Keystream => {
            let (movable, driver) = draw_keystream(&scene, &rounds)?;
            (None, driver, movable)
        },
    };
    // Drawing resized the `<svg>` to fit its content, so the bounds are read only now. A drag handler keeps the bounds
    // it was given, and cannot be installed twice on the same node.
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let drag = DragOptions::default().with_bounds(Some(view_box_rect(&svg)?));
    for node in movable {
        scene.make_draggable_with(node, drag).map_err(stringify)?;
    }
    scene
        .show_scene_title(walk::title(&rounds, position), SceneTitleOptions::default())
        .map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }
    let shown = walk::shown(&rounds, position);
    if let Some(nodes) = &nodes {
        apply(&scene, nodes, &shown)?;
    }

    let options = demo_common::util::step_toolbar_options().with_stride(SelectionStride::new(ROUND_STRIDE, "Round"));
    scene
        .show_selection_toolbar(driver, options, |_scene, _node, transition| go(transition.to))
        .map_err(stringify)?;
    // `show_selection_toolbar` always resets the driver to unstarted as its first committed act. Reapply the position.
    if let Some(position) = position {
        scene.set_selection(driver, Selection::Cell(position)).map_err(stringify)?;
    }

    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene,
            nodes,
            drawn: stage,
            rounds,
            titled: Some(shown.quarter_round),
        });
    });
    if let Some(selector) = focused {
        demo_common::focus::focus(&selector);
    }
    demo_common::util::fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the diagram to walk position `position`, where `None` is the walk before its first step. Within a diagram this
/// only writes values and marks. Crossing to the keystream block, or back, draws the diagram again.
///
/// A failure is shown in the window's own error banner, and logged, since the toolbar's own callback has nowhere to
/// return it to. The banner clears again at the next step that works.
fn go(position: Option<usize>) {
    match step_to(position) {
        Ok(()) => demo_common::util::clear_error(ERROR_BANNER),
        Err(e) => {
            demo_common::util::report_error(ERROR_BANNER, &format!("This step failed (position {position:?}): {e}"))
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The work of [`go`].
fn step_to(position: Option<usize>) -> Result<(), String> {
    let wanted = walk::stage(position);
    let (drawn, rounds) = DEMO
        .with_borrow(|slot| slot.as_ref().map(|demo| (demo.drawn, demo.rounds.clone())))
        .ok_or("the diagram is not built")?;
    if drawn != wanted {
        return build(wanted, position, rounds);
    }
    DEMO.with_borrow_mut(|slot| {
        let demo = slot.as_mut().ok_or("the diagram is not built")?;
        let Some(nodes) = &demo.nodes else { return Ok(()) };
        let shown = walk::shown(&demo.rounds, position);
        apply(&demo.scene, nodes, &shown)?;
        if demo.titled != Some(shown.quarter_round) {
            demo.scene
                .show_scene_title(walk::title(&demo.rounds, position), SceneTitleOptions::default())
                .map_err(stringify)?;
            demo.titled = Some(shown.quarter_round);
        }
        Ok(())
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the ChaCha20 scene before its first step, with nothing worked out yet, wires its stepping toolbar, and
/// resizes the window it runs in to fit the scene. Runs in `chacha20-window.html`, not in the gallery.
///
/// The initial state of RFC 8439, section 2.3.2, is at the top. Below it are the four columns of the first quarter
/// round: `a`, `b`, `c` and `d` going in, the 12 steps that mix them, and `a`, `b`, `c` and `d` coming out. Below those
/// is the state after the quarter round, which is the initial state with those four words replaced. The four words
/// involved are marked in both states.
///
/// Each of the 12 steps is an operator node, `ADD`, `XOR` or `ROTL`, in the column of the word it updates. An `ADD` or
/// `XOR` takes the latest value of a second word, so its connector crosses from that word's column. At the right is a
/// list of the steps, each showing the value it produced.
///
/// The selection toolbar steps through the walk. "Next" and "Prev" move one step, and when a quarter round ends they
/// move on to the next, which is drawn in the same diagram with its own state. A quarter round mixes a column of the
/// state in an odd round and a diagonal in an even round, and the title says which. The "Round" buttons move on a whole
/// round. Once the last step of round 20 is done, one more step shows the keystream block that the rounds lead to.
/// "Restart" returns the walk to the beginning. Every node the walk has not reached then shows zero, and is drawn
/// faint.
///
/// Every value shown is worked out by [`algorithm::rounds`]. `svg_dom_graph` itself never evaluates an operator. See
/// [`Scene::add_arithmetic_operator_node`]'s own doc comment for why. So this function does what a real caller would:
/// it takes each real value and hands it to the library alongside the operator that produced it, and later replaces it
/// with [`Scene::set_data_values`].
///
/// # Errors
///
/// Returns `Err` if the page is missing `#chacha20-diagram-stage`, or if any library call fails.
pub(crate) fn build_chacha20_window() -> Result<(), String> {
    let key = rfc_key();
    let rounds = Rc::new(algorithm::rounds(&algorithm::initial_state(&key, COUNTER, &NONCE)));
    build(Stage::QuarterRound, None, rounds)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ChaCha20 scene is larger than most demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `chacha20-window.html`, which runs [`build_chacha20_window`] there. A named
/// window is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#chacha20-open`, or if a listener could not be attached to it.
pub(crate) fn build_chacha20_demo() -> Result<(), String> {
    demo_common::util::wire_open_window(&demo_common::util::document()?, "chacha20-open", WINDOW_PAGE, WINDOW_NAME)
}
