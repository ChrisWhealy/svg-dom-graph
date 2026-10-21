//! Drawing the scene: the controls along the top, then the path of boxes laid out by [`snake`], each with its note.

use super::{
    DIAGRAM_ID, MARGIN, NOTE_WIDTH, STAGE_ID, TOOLBAR_ROOM, TOP,
    code::Transmission,
    controls, progress,
    snake::{self, Item, Plan},
    walk::{BOXES, POSITIONS},
    words,
};
use crate::util::{document, ensure_svg_in, hide_step_driver, required_element, resize_svg, stringify};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        BitValues, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Side,
        ToolbarOptions,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The room outside the block of cells for the connectors that loop round the ends of the rows.
const LOOP_ROOM: f64 = 50.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was drawn: the scene, and the "Step" node that drives the toolbar.
pub(super) struct Drawn {
    pub(super) scene: Scene,
    pub(super) step: NodeId,
    /// The position shown.
    pub(super) position: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Bits that are each one bit wide, written in binary and laid out in one row.
fn bit_row(bits: &[u8]) -> Result<DataNodeContent, String> {
    let values = BitValues::new(1, bits.to_vec()).map_err(stringify)?;
    Ok(DataNodeContent::new(NodeValues::Bits(values), DataFormat::Binary).with_layout(GridLayout::Columns(bits.len())))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The boxes of the path, in order, each with its name and its content.
fn boxes(t: &Transmission) -> Result<Vec<(&'static str, DataNodeContent)>, String> {
    let syndrome = BitValues::new(3, vec![t.syndrome]).map_err(stringify)?;
    Ok(vec![
        ("Data bits d1 d2 d3 d4", bit_row(&t.data)?),
        ("Parity bits p1 p2 p4", bit_row(&t.parity)?),
        ("Codeword p1 p2 d1 p4 d2 d3 d4", bit_row(&t.codeword)?),
        ("Received codeword", bit_row(&t.received)?),
        ("Check bits c1 c2 c4", bit_row(&t.checks)?),
        (
            "Syndrome c4 c2 c1",
            DataNodeContent::new(NodeValues::Bits(syndrome), DataFormat::Binary),
        ),
        (
            "Error position",
            DataNodeContent::new(NodeValues::U8(vec![t.syndrome]), DataFormat::Decimal),
        ),
        ("Corrected codeword", bit_row(&t.corrected)?),
        ("Recovered data d1 d2 d3 d4", bit_row(&t.recovered)?),
    ])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `t` at walk position `position`, in the stage, replacing whatever was there. `status` is what
/// the status line beside the buttons says. The caller keeps the zoom and pan, and wires the toolbar to the returned
/// "Step" node.
///
/// The path of boxes is as long as the calculation: from the data bits, through the parity bits, the codeword, the
/// received codeword, the checks, the syndrome and the error position, to the corrected codeword and the recovered
/// data. [`snake::best_columns`] chooses how many boxes go along a row so that the whole path takes as little room as
/// it can.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
pub(super) fn draw(t: &Transmission, position: usize, status: &str) -> Result<Drawn, String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    // Measure the path: every box, then every note at the width of a cell, which is the widest box or the usual width
    // of a note.
    let contents = boxes(t)?;
    let notes = words::notes(t);
    let mut sizes = Vec::with_capacity(BOXES);
    for (name, content) in &contents {
        sizes.push(scene.measure_named_data_node(name, content).map_err(stringify)?);
    }
    let width = sizes.iter().map(|s| s.width).fold(NOTE_WIDTH, f64::max);
    let mut items = Vec::with_capacity(BOXES);
    for (size, note) in sizes.iter().zip(&notes) {
        items.push(Item {
            node: *size,
            note_height: scene.measure_markdown_node(width, note).map_err(stringify)?.height,
        });
    }
    let columns = snake::best_columns(&items, width);

    // The controls take the first row, and the path starts under them.
    let left = MARGIN + LOOP_ROOM;
    let controls = controls::add_controls(&scene, left, TOP, status, t)?;
    let plan = snake::plan(&items, columns, width, left, controls.bottom + snake::GAP_Y);

    let mut nodes = Vec::with_capacity(BOXES);
    let mut note_nodes = Vec::with_capacity(BOXES);
    let mut shown = Vec::with_capacity(2 * BOXES);
    for (((name, content), note), placement) in contents.into_iter().zip(&notes).zip(&plan.placements) {
        let node = scene.add_named_data_node(placement.node, name, content).map_err(stringify)?;
        let words = scene.add_markdown_node(placement.note, width, note).map_err(stringify)?;
        nodes.push(node);
        note_nodes.push(words);
        shown.extend([node, words]);
    }
    shown.push(controls.status);
    connect(&scene, &nodes, &plan)?;
    progress::show(&scene, &nodes, &note_nodes, t, position)?;

    fit_canvas(&document, &scene, &plan, controls.right - left, &shown)?;
    let step = add_step_driver(&scene)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok(Drawn {
        scene,
        step,
        position: position.min(POSITIONS - 1),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Joins each box to the next. Along a row the connector is straight. At the end of a row it is an elbow that loops
/// round the outside.
fn connect(scene: &Scene, nodes: &[NodeId], plan: &Plan) -> Result<(), String> {
    for (pair, placements) in nodes.windows(2).zip(plan.placements.windows(2)) {
        let link = snake::link(&placements[0], &placements[1]);
        let mut options = ConnectorOptions::default()
            .with_from_side(Some(link.from))
            .with_to_side(Some(link.to));
        if link.turns {
            options = options.with_connector_type(ConnectorType::Elbow { corner_radius: 12.0 });
        }
        scene.add_edge_with(pair[0], pair[1], options).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The canvas is as wide and tall as the controls, the boxes and the notes need, with room beside the block for the
/// connectors that loop round it and east of it for the zoom toolbar.
fn fit_canvas(
    document: &web_sys::Document,
    scene: &Scene,
    plan: &Plan,
    controls_width: f64,
    nodes: &[NodeId],
) -> Result<(), String> {
    let block = MARGIN + LOOP_ROOM + plan.size.width.max(controls_width) + LOOP_ROOM;
    let (mut right, mut lowest) = (block, 0.0_f64);
    for &node in nodes {
        let rect = scene.node_rect(node).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
        lowest = lowest.max(rect.origin.y + rect.size.height);
    }
    resize_svg(document, DIAGRAM_ID, Size::new(right + MARGIN + TOOLBAR_ROOM, lowest + MARGIN))?;
    scene.refresh_layout().map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Step" is a small array of one cell for each walk position, placed far off-canvas, purely to drive the selection
/// toolbar. It is hidden from assistive technology, since the toolbar's own buttons announce the position.
fn add_step_driver(scene: &Scene) -> Result<NodeId, String> {
    let values: Vec<u16> = (1..=POSITIONS).map(|n| n as u16).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U16(values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    hide_step_driver(DIAGRAM_ID)?;
    Ok(step)
}
