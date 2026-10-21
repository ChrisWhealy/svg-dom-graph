//! Drawing the scene: the sender's column going down, the receiver's going up, and the controls along the top.

use super::{
    DIAGRAM_ID, GAP, INDEX_COLUMNS, INPUT_LEN, INPUT_WIDTH, NOTE_WIDTH, RECEIVER_EXPLANATION_WIDTH, ROW_GAP, SCALE,
    SENDER_EXPLANATION_WIDTH, STAGE_ID, TOOLBAR_ROOM, TOP, TYPICAL_INPUT_HEIGHT,
    algorithm::{Computation, FINAL_MASK, Finding, INITIAL, compute},
    column::{Column, Spec, array_of_words, hexdump, plain, word},
    controls,
    flips::Flips,
    progress::{Nodes, ReceiverNodes},
    transmission::Transmission,
    walk::Walk,
    words::{Words, words},
};
use crate::{
    base64::encode::{length_note, utf8_prefix},
    util::{MARGIN, document, ensure_svg_in, required_element, resize_svg, stringify},
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, NodeOptions,
        NodeValues, Scene, Side, ToolbarOptions,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fill of the notes that say something about the text or about the error. The boxes of the explanations have the
/// library's usual light blue, as the buttons would, so these are white to look different from both.
const NOTE_FILL: &str = "#ffffff";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was drawn: the scene, the text input node, the "Step" node that drives the toolbar, and the position shown.
pub(super) struct Drawn {
    pub(super) scene: Scene,
    pub(super) input: NodeId,
    pub(super) step: NodeId,
    /// The position the scene shows, which is the one asked for unless the text is too short to have it.
    pub(super) position: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The widths and sizes that the layout is worked out from.
struct Measures {
    /// How wide the text box is, and the widest of the boxes that run down the middle of a column.
    input_width: f64,
    middle_width: f64,
    mask_width: f64,
    xor: Size,
    /// The gap between the columns.
    channel_width: f64,
    /// How wide the box that sits to the left of the sender's column is.
    aside_width: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the rows share while they are being drawn.
struct Drawing<'a> {
    scene: &'a Scene,
    bytes: &'a [u8],
    computed: &'a Computation,
    transmission: &'a Transmission,
    words: &'a Words,
    measures: Measures,
    sender: Column<'a>,
    receiver: Column<'a>,
    /// The sender goes down and the receiver goes up.
    downward: ConnectorOptions,
    upward: ConnectorOptions,
    /// The x of the left edge of the XOR that makes the CRC, in the gap between the columns.
    xor_left: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the sender's rows leave behind for the rows that follow.
struct SenderRows {
    /// The bytes as a hex dump, and the register after each byte or the initial register if there are none.
    dump: Option<NodeId>,
    register: NodeId,
    per_byte: Vec<NodeId>,
    /// The top of the next row.
    y: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sender's last row: the register after the last byte, and the XOR that makes the CRC from it.
struct LastRow {
    last: NodeId,
    sent: NodeId,
    mask: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `text` as it is at walk position `position`, in the stage, replacing whatever was there. The
/// caller keeps the zoom and pan, and wires the toolbar to the returned "Step" node.
///
/// The library cannot change how many values a node holds, so the scene is drawn again for each change, with exactly as
/// many values as the text has bytes. An empty text has no bytes to shift through the register, so it has only the
/// sender's initial register and the final steps, and nothing to send.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
pub(super) fn draw(text: &str, position: usize, flips: &Flips) -> Result<Drawn, String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let bytes = utf8_prefix(text, INPUT_LEN);
    let computed = compute(bytes);
    let transmission = Transmission::new(bytes, computed.crc, flips);
    let words = words(bytes, &computed, &transmission);
    let measures = measure(&scene, bytes, computed.crc)?;

    let sender_axis =
        MARGIN + SENDER_EXPLANATION_WIDTH + GAP + measures.aside_width + GAP + measures.middle_width / 2.0;
    let receiver_axis = sender_axis + measures.middle_width + measures.channel_width;
    let sender = Column {
        scene: &scene,
        axis: sender_axis,
        explain_left: MARGIN,
        explanation_width: SENDER_EXPLANATION_WIDTH,
        side: MARGIN + SENDER_EXPLANATION_WIDTH + GAP + measures.aside_width / 2.0,
        nodes: Vec::new(),
        notes: Vec::new(),
    };
    let receiver = Column {
        scene: &scene,
        axis: receiver_axis,
        explain_left: receiver_axis + measures.middle_width / 2.0 + GAP,
        explanation_width: RECEIVER_EXPLANATION_WIDTH,
        side: receiver_axis,
        nodes: Vec::new(),
        notes: Vec::new(),
    };
    let xor_left = sender_axis + measures.middle_width / 2.0 + GAP;
    let mut drawing = Drawing {
        scene: &scene,
        bytes,
        computed: &computed,
        transmission: &transmission,
        words: &words,
        measures,
        sender,
        receiver,
        downward: ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North)),
        upward: ConnectorOptions::default()
            .with_from_side(Some(Side::North))
            .with_to_side(Some(Side::South)),
        xor_left,
    };

    let (input, y) = drawing.add_first_row(text, &flips.describe())?;
    let rows = drawing.add_sender_rows(input, y)?;
    let last_row = drawing.add_last_row(&rows)?;
    let receiver_nodes = drawing.add_receiver(&rows, &last_row)?;

    let mut nodes = drawing.sender.nodes;
    nodes.extend(drawing.receiver.nodes);
    nodes.extend([last_row.mask, last_row.sent]);
    fit_canvas(&document, &scene, &nodes)?;

    let walk = Walk::new(bytes.len());
    let position = position.min(walk.positions() - 1);
    let shown = Nodes {
        bytes: bytes.len(),
        dump: rows.dump,
        per_byte: rows.per_byte,
        notes: drawing.sender.notes.iter().chain(&drawing.receiver.notes).copied().collect(),
        last_register: last_row.last,
        sent_crc: last_row.sent,
        receiver: receiver_nodes,
    };
    super::progress::show(&scene, &shown, &walk.progress(position))?;

    let step = add_step_driver(&scene, walk.positions())?;
    // Nothing is made draggable: the boxes are placed to line up, and the explanations sit beside them.
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok(Drawn { scene, input, step, position })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Measures what the layout is worked out from. Both columns are as wide as their widest box.
fn measure(scene: &Scene, bytes: &[u8], crc: u32) -> Result<Measures, String> {
    let mut middle_width = INPUT_WIDTH;
    let mut input_width = INPUT_WIDTH;
    if !bytes.is_empty() {
        let dump = scene
            .measure_named_data_node("Input bytes", &hexdump(bytes))
            .map_err(stringify)?;
        input_width = dump.width.min(INPUT_WIDTH);
        middle_width = dump.width;
        let index = array_of_words(vec![0; bytes.len()]);
        let register = array_of_words(vec![0; bytes.len()]);
        for (name, content) in [
            ("Table index", &index),
            ("Table entry", &register),
            ("Received register after each byte", &register),
        ] {
            middle_width = middle_width.max(scene.measure_named_data_node(name, content).map_err(stringify)?.width);
        }
    }
    let initial = scene
        .measure_named_data_node("Initial register", &word(INITIAL))
        .map_err(stringify)?;
    let mask = scene
        .measure_named_data_node("Final mask", &word(FINAL_MASK))
        .map_err(stringify)?;
    let xor = scene.measure_operator_box("XOR", &word(crc)).map_err(stringify)?;
    // The gap between the columns holds the XOR that makes the CRC, with a clear lane at its middle for the bytes to
    // cross, so it is twice as wide as the XOR and the space that it needs on both sides.
    let channel_width = 2.0 * (xor.width.max(mask.width) + 2.0 * GAP);
    Ok(Measures {
        input_width,
        middle_width,
        mask_width: mask.width,
        xor,
        channel_width,
        aside_width: initial.width,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The canvas is as wide and tall as its nodes need, with room east of them for the zoom toolbar. The view box stays as
/// large as the scene, and the element is made smaller than it, so the browser draws all of it, text included, scaled
/// down to fit.
fn fit_canvas(document: &web_sys::Document, scene: &Scene, nodes: &[NodeId]) -> Result<(), String> {
    let (mut right, mut lowest) = (0.0_f64, 0.0_f64);
    for &node in nodes {
        let rect = scene.node_rect(node).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
        lowest = lowest.max(rect.origin.y + rect.size.height);
    }
    let size = Size::new(right + MARGIN + TOOLBAR_ROOM, lowest + MARGIN);
    resize_svg(document, DIAGRAM_ID, size)?;
    let diagram = required_element(document, DIAGRAM_ID)?;
    for (name, value) in [("width", size.width * SCALE), ("height", size.height * SCALE)] {
        diagram
            .set_attribute(name, &value.to_string())
            .map_err(|e| format!("could not scale #{DIAGRAM_ID}: {e:?}"))?;
    }
    scene.refresh_layout().map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Step" is a small array of one cell for each walk position, placed far off-canvas, purely to drive the selection
/// toolbar. It is hidden from assistive technology, since the toolbar's own buttons announce the position.
fn add_step_driver(scene: &Scene, positions: usize) -> Result<NodeId, String> {
    let values: Vec<u16> = (1..=positions).map(|n| n as u16).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U16(values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    crate::util::hide_step_driver(DIAGRAM_ID)?;
    Ok(step)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Drawing<'_> {
    /// The first row: the text box, with what it holds that the checksum leaves out on its right, and what it is on the
    /// left. After them come the controls for introducing an error. Returns the text input node and the top of the next
    /// row.
    ///
    /// The text box is a library node whose height is not known until it is drawn, so the row is started low enough for
    /// the taller note, and the notes are centred on the box once it is there.
    fn add_first_row(&mut self, text: &str, status: &str) -> Result<(NodeId, f64), String> {
        let scene = self.scene;
        let note_text = length_note(text.len(), self.bytes.len(), INPUT_LEN);
        let note_size = scene.measure_markdown_node(NOTE_WIDTH, &note_text).map_err(stringify)?;
        let words_size = scene
            .measure_markdown_node(SENDER_EXPLANATION_WIDTH, &self.words.input)
            .map_err(stringify)?;
        let tallest = note_size.height.max(words_size.height);
        let top = TOP + (tallest - TYPICAL_INPUT_HEIGHT).max(0.0) / 2.0;
        let input = scene
            .add_text_input_node(
                Point::new(self.sender.axis - self.measures.input_width / 2.0, top),
                self.measures.input_width,
                "Text to checksum",
                text,
                |_, id, text| super::typed(id, text),
            )
            .map_err(stringify)?;
        let input_rect = scene.node_rect(input).map_err(stringify)?;
        let middle = input_rect.origin.y + input_rect.size.height / 2.0;
        let note = scene
            .add_markdown_node_with(
                Point::new(
                    input_rect.origin.x + input_rect.size.width + GAP / 3.0,
                    middle - note_size.height / 2.0,
                ),
                NOTE_WIDTH,
                &note_text,
                NodeOptions::default().with_fill(Some(NOTE_FILL)),
            )
            .map_err(stringify)?;
        self.sender.nodes.extend([input, note]);
        self.sender.explain(middle, &self.words.input)?;

        // The controls are to the right of the note, level with the text box.
        let note_rect = scene.node_rect(note).map_err(stringify)?;
        let status_node = controls::add_controls(
            scene,
            note_rect.origin.x + note_rect.size.width + controls::CONTROL_GAP,
            middle,
            status,
            NodeOptions::default().with_fill(Some(NOTE_FILL)),
        )?;
        self.sender.nodes.push(status_node);
        let y = (input_rect.origin.y + input_rect.size.height)
            .max(middle + tallest / 2.0)
            .max(top + TYPICAL_INPUT_HEIGHT)
            + ROW_GAP;
        Ok((input, y))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The sender's rows down the page from `y`, each connected to the one before it: the bytes, the table index and
    /// entry for each, and the register after each. Without text there is only the initial register.
    fn add_sender_rows(&mut self, input: NodeId, mut y: f64) -> Result<SenderRows, String> {
        let (scene, words) = (self.scene, self.words);
        if self.bytes.is_empty() {
            // Nothing is shifted through the register, so the initial register is the register after the last byte.
            let spec = Spec {
                name: "Initial register",
                content: word(INITIAL),
                explanation: words.no_bytes.clone(),
                aside: None,
            };
            let height = self.sender.height(&spec)?;
            let (initial, _) = self.sender.place(y, height, spec)?;
            scene.add_edge_with(input, initial, self.downward).map_err(stringify)?;
            return Ok(SenderRows {
                dump: None,
                register: initial,
                per_byte: Vec::new(),
                y: y + height + ROW_GAP,
            });
        }

        let (mut previous, mut dump, mut per_byte) = (input, None, Vec::new());
        let steps = &self.computed.steps;
        for (name, content, explanation, aside) in [
            ("Input bytes", hexdump(self.bytes), &words.bytes, None),
            (
                "Table index",
                DataNodeContent::new(NodeValues::U8(steps.iter().map(|s| s.index).collect()), DataFormat::Hexadecimal)
                    .with_layout(GridLayout::Columns(INDEX_COLUMNS)),
                &words.index,
                None,
            ),
            (
                "Table entry",
                array_of_words(steps.iter().map(|s| s.entry).collect()),
                &words.entry,
                None,
            ),
            (
                "Register after each byte",
                array_of_words(steps.iter().map(|s| s.register).collect()),
                &words.register,
                Some(("Initial register", word(INITIAL))),
            ),
        ] {
            let spec = Spec {
                name,
                content,
                explanation: explanation.clone(),
                aside,
            };
            let height = self.sender.height(&spec)?;
            let (id, aside) = self.sender.place(y, height, spec)?;
            y += height + ROW_GAP;
            scene.add_edge_with(previous, id, self.downward).map_err(stringify)?;
            if let Some(initial) = aside {
                // The initial register is on the left of the register, which is on the left of the box it feeds.
                scene
                    .add_edge_with(
                        initial,
                        id,
                        ConnectorOptions::default()
                            .with_from_side(Some(Side::East))
                            .with_to_side(Some(Side::West)),
                    )
                    .map_err(stringify)?;
            }
            if name == "Input bytes" {
                dump = Some(id);
            } else {
                per_byte.push(id);
            }
            previous = id;
        }
        Ok(SenderRows {
            dump,
            register: previous,
            per_byte,
            y,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The last row of the sender: the register after the last byte, with the XOR that flips it on its right, level
    /// with it, in the gap between the columns. The mask is under the XOR.
    fn add_last_row(&mut self, rows: &SenderRows) -> Result<LastRow, String> {
        let (scene, words) = (self.scene, self.words);
        let xor_size = self.measures.xor;
        let spec = Spec {
            name: "Register after the last byte",
            content: word(self.computed.register),
            explanation: format!("{}\n\n{}", words.last, words.result),
            aside: None,
        };
        let height = self.sender.height(&spec)?.max(xor_size.height);
        let middle = rows.y + height / 2.0;
        let (last, _) = self.sender.place(rows.y, height, spec)?;
        if rows.register != last {
            scene.add_edge_with(rows.register, last, self.downward).map_err(stringify)?;
        }
        let mask_top = (rows.y + height).max(middle + xor_size.height / 2.0) + ROW_GAP;
        let mask = scene
            .add_named_data_node(
                Point::new(self.xor_left + (xor_size.width - self.measures.mask_width) / 2.0, mask_top),
                "Final mask",
                word(FINAL_MASK),
            )
            .map_err(stringify)?;
        let sent = scene
            .add_binary_operator_node(
                Point::new(self.xor_left, middle - xor_size.height / 2.0),
                BinaryOperator::Xor,
                (last, mask),
                word(self.computed.crc),
            )
            .map_err(stringify)?;
        Ok(LastRow { last, sent, mask })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The receiver's column, from the bottom so that the data flows up it. Its top row, the corrected bytes, is level
    /// with the sender's input bytes, so that the two can be compared at a glance. There is nothing to receive without
    /// text, so then nothing is drawn.
    fn add_receiver(&mut self, rows: &SenderRows, last_row: &LastRow) -> Result<Option<ReceiverNodes>, String> {
        let Some(dump) = rows.dump else { return Ok(None) };
        let (scene, t, words) = (self.scene, self.transmission, self.words);
        let up = self.upward;
        let elbow = ConnectorType::Elbow { corner_radius: 12.0 };
        let received_dump = Spec {
            name: "Received bytes",
            content: hexdump(&t.received),
            explanation: words.received.clone(),
            aside: None,
        };
        let register_row = Spec {
            name: "Received register after each byte",
            content: array_of_words(t.received_computed.steps.iter().map(|s| s.register).collect()),
            explanation: words.received_register.clone(),
            aside: None,
        };
        let crc_row = Spec {
            name: "CRC of the received bytes",
            content: word(t.received_computed.crc),
            explanation: words.received_crc.clone(),
            aside: None,
        };
        let position_row = Spec {
            name: "Error position",
            content: plain(&t.position()),
            explanation: words.position.clone(),
            aside: None,
        };
        let corrected_row = Spec {
            name: "Corrected bytes",
            content: hexdump(&t.corrected),
            explanation: words.corrected.clone(),
            aside: None,
        };
        let syndrome_content = word(t.syndrome);
        // From the top: corrected, position, syndrome, CRC, register, received.
        let heights = [
            self.receiver.height(&corrected_row)?,
            self.receiver.height(&position_row)?,
            self.receiver.operator_height(&syndrome_content, &words.syndrome)?,
            self.receiver.height(&crc_row)?,
            self.receiver.height(&register_row)?,
            self.receiver.height(&received_dump)?,
        ];
        let dump_rect = scene.node_rect(dump).map_err(stringify)?;
        let dump_middle = dump_rect.origin.y + dump_rect.size.height / 2.0;
        let mut tops = [0.0; 6];
        tops[0] = dump_middle - heights[0] / 2.0;
        for i in 1..6 {
            tops[i] = tops[i - 1] + heights[i - 1] + ROW_GAP;
        }

        let (received_id, _) = self.receiver.place(tops[5], heights[5], received_dump)?;
        let (register_id, _) = self.receiver.place(tops[4], heights[4], register_row)?;
        let (crc_id, _) = self.receiver.place(tops[3], heights[3], crc_row)?;
        // The CRC travels with the data, so the receiver has its own copy of it. It is level with the syndrome, on its
        // left, so that it enters the syndrome at its west edge, and the receiver's own CRC enters at its south edge.
        // The CRC reaches the copy from the sender's XOR, straight up the gap between the columns.
        let xor_size = self.measures.xor;
        let copy_content = word(t.arrived_crc);
        let copy_size = scene
            .measure_named_data_node("CRC that arrived", &copy_content)
            .map_err(stringify)?;
        let syndrome_middle = tops[2] + heights[2] / 2.0;
        let copy = scene
            .add_named_data_node(
                Point::new(
                    self.xor_left + (xor_size.width - copy_size.width) / 2.0,
                    syndrome_middle - copy_size.height / 2.0,
                ),
                "CRC that arrived",
                copy_content,
            )
            .map_err(stringify)?;
        self.receiver.nodes.push(copy);
        let syndrome_id =
            self.receiver
                .place_operator(tops[2], heights[2], (copy, crc_id), syndrome_content, &words.syndrome)?;
        scene.add_edge_with(last_row.sent, copy, up).map_err(stringify)?;
        let (position_id, _) = self.receiver.place(tops[1], heights[1], position_row)?;
        let (corrected_id, _) = self.receiver.place(tops[0], heights[0], corrected_row)?;

        // The bytes cross from the sender to the receiver, and the error is introduced on the way across. The edge is
        // an elbow, and its upright part is in the middle of the gap, clear of the XOR.
        scene
            .add_edge_with(
                dump,
                received_id,
                ConnectorOptions::default()
                    .with_from_side(Some(Side::East))
                    .with_to_side(Some(Side::West))
                    .with_connector_type(elbow),
            )
            .map_err(stringify)?;
        scene.add_edge_with(received_id, register_id, up).map_err(stringify)?;
        scene.add_edge_with(register_id, crc_id, up).map_err(stringify)?;
        scene.add_edge_with(syndrome_id, position_id, up).map_err(stringify)?;
        scene.add_edge_with(position_id, corrected_id, up).map_err(stringify)?;
        // The received bytes are what is corrected. Their edge goes round the right-hand side of the column.
        scene
            .add_edge_with(
                received_id,
                corrected_id,
                ConnectorOptions::default()
                    .with_from_side(Some(Side::East))
                    .with_to_side(Some(Side::East))
                    .with_connector_type(elbow),
            )
            .map_err(stringify)?;
        self.mark_differences(received_id, register_id, copy, corrected_id)?;

        Ok(Some(ReceiverNodes {
            bytes: received_id,
            registers: register_id,
            crc: crc_id,
            arrived_crc: copy,
            syndrome: syndrome_id,
            position: position_id,
            corrected: corrected_id,
        }))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks the bytes that changed, and the registers that differ because of them. So is the CRC if it changed, and
    /// the byte that is corrected.
    fn mark_differences(
        &self,
        received: NodeId,
        registers: NodeId,
        arrived_crc: NodeId,
        corrected: NodeId,
    ) -> Result<(), String> {
        let (scene, t) = (self.scene, self.transmission);
        if !t.flips.crc.is_empty() {
            scene.set_secondary_selection(arrived_crc, &[0]).map_err(stringify)?;
        }
        if !t.flips.data.is_empty() {
            scene.set_secondary_selection(received, &t.flips.bytes()).map_err(stringify)?;
            let differ: Vec<usize> = t
                .received_computed
                .steps
                .iter()
                .zip(&self.computed.steps)
                .enumerate()
                .filter(|(_, (received, sent))| received.register != sent.register)
                .map(|(i, _)| i)
                .collect();
            scene.set_secondary_selection(registers, &differ).map_err(stringify)?;
        }
        if let Finding::Data { byte, .. } = t.finding {
            scene.set_secondary_selection(corrected, &[byte]).map_err(stringify)?;
        }
        Ok(())
    }
}
