//! The CRC-32 panel:
//!
//! - `panel-crc32` / `#crc32-diagram` — [`build_crc32_demo`]: the CRC-32 of typed text, a byte at a time, with a button
//!   that opens the scene in a window of its own: [`build_crc32_window`].

mod algorithm;
mod fields;

use crate::{
    base64::encode::{length_note, utf8_prefix},
    source_file,
    util::{
        MARGIN, document, ensure_svg_in, fit_window_to_stage, required_element, resize_svg, stringify, wire_open_window,
    },
};
use algorithm::{Computation, FINAL_MASK, Finding, INITIAL, POLYNOMIAL, compute, locate};
use fields::{Complaint, Problem, bit_and_byte};
use std::cell::{Cell, RefCell};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, HexDumpFormat,
        NodeValues, Scene, Side, ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the calculation, then the scene that draws it.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("crc32/algorithm.rs", "algorithm.rs"),
    source_file!("crc32/fields.rs", "fields.rs"),
    source_file!("crc32/mod.rs", "mod.rs"),
];

/// What the callbacks need to find again: the scene on the page, and what it shows.
struct Demo {
    scene: Scene,
    /// The text the scene was drawn for.
    text: String,
    /// The scene's text input node.
    input: NodeId,
}

thread_local! {
    // Keeps the demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does. Every change draws a new one.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
    // Whether the user is part way through composing text with an input method editor.
    static COMPOSING: Cell<bool> = const { Cell::new(false) };
    // The bit the user has flipped in transmission, if any: its byte, and its bit in the byte, from 0 at the least
    // significant. It stays across redraws.
    static ERROR: Cell<Option<(usize, u8)>> = const { Cell::new(None) };
    // What the two fields for choosing the bit to flip hold, so that they keep it across redraws.
    static FLIP_BYTE: RefCell<String> = RefCell::new("0".to_owned());
    static FLIP_BIT: RefCell<String> = RefCell::new("0".to_owned());
    // What is wrong with each of those two fields, from the last time a button was pressed, or `None` if nothing is. It
    // stays until the field is edited or a button is pressed again.
    static COMPLAINTS: RefCell<[Option<Complaint>; 2]> = const { RefCell::new([None, None]) };
}

const STAGE_ID: &str = "crc32-diagram-stage";
const DIAGRAM_ID: &str = "crc32-diagram";

/// The text the demo opens on.
const INITIAL_TEXT: &str = "Has this text been corrupted during transmission?";

/// How many bytes of the typed text the checksum covers. Each byte is a step in the diagram, so the limit keeps the
/// diagram to a manageable size. Text past it is not covered, and the cut falls between characters.
const INPUT_LEN: usize = 64;

/// How wide the text box is when there is no hex dump to be as wide as, and the note on its left.
const INPUT_WIDTH: f64 = 580.0;
const NOTE_WIDTH: f64 = 260.0;

/// How wide the explanation beside each box is: on the left of the sender's column, which has the most to say, and on the
/// right of the receiver's.
const SENDER_EXPLANATION_WIDTH: f64 = 570.0;
const RECEIVER_EXPLANATION_WIDTH: f64 = 380.0;

/// About how tall the text box is. The library draws it at a height of its own.
const TYPICAL_INPUT_HEIGHT: f64 = 70.0;

/// The gap between two columns, wide enough for a connector, and the room east of the last for the zoom toolbar.
const GAP: f64 = 60.0;
const TOOLBAR_ROOM: f64 = 80.0;

/// How much the diagram is scaled down, text and all. It is the largest of the demos, and it has a window to itself.
const SCALE: f64 = 0.85;

/// The names of the two fields that choose the bit to flip, in the order of [`COMPLAINTS`].
const FIELD_LABELS: [&str; 2] = ["Bit (0 to 7)", "Byte"];

/// The fill that marks a field that is not a number at all, and one that is a number but out of range. Neither is the
/// only sign: the status line says which field and what is wrong with it.
const NOT_A_NUMBER_FILL: &str = "#f5b7b1";
const OUT_OF_RANGE_FILL: &str = "#fad7a0";

/// The size of the fields and buttons for introducing an error, and the gap between one and the next.
const FIELD_WIDTH: f64 = 120.0;
const BUTTON_WIDTH: f64 = 180.0;
const BUTTON_HEIGHT: f64 = 36.0;
const STATUS_WIDTH: f64 = 300.0;
const CONTROL_GAP: f64 = 20.0;

/// The id of the live region in the window page that announces a change of the error.
const ANNOUNCEMENT_ID: &str = "crc32-announcement";

/// Finds the text box that holds the text to checksum, and not the fields for choosing a bit.
const TEXT_BOX: &str = "#crc32-diagram g[aria-label^=\"Text to checksum\"] input";

/// The XML namespace SVG elements are created in.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The space between one row of boxes and the next, wide enough for a connector.
const ROW_GAP: f64 = 50.0;

/// How many values the table index, the table entry and the register lay out on a row.
const INDEX_COLUMNS: usize = 8;
const WORD_COLUMNS: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The eight shifts that make the table's entry for `index`, as a numbered list. Each shifts the register right by one
/// place, and XORs in the polynomial if the bit that fell off the end was a one.
fn shifts(index: u8) -> String {
    let mut register = u32::from(index);
    let mut lines = Vec::new();
    for n in 1..=8 {
        let before = register;
        let fell = before & 1;
        let shifted = before >> 1;
        register = if fell == 1 { shifted ^ POLYNOMIAL } else { shifted };
        lines.push(if fell == 1 {
            format!(
                "{n}. `{before:08X}` loses a **1**. Shifted: `{shifted:08X}`. XOR `{POLYNOMIAL:08X}` = `{register:08X}`"
            )
        } else {
            format!("{n}. `{before:08X}` loses a **0**. Shifted: `{shifted:08X}`")
        });
    }
    lines.join("\n")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happens to the data between the sender and the receiver: the bytes that were sent, the bytes that arrived, and
/// what the receiver works out from them.
struct Transmission {
    sent: Vec<u8>,
    sent_crc: u32,
    /// The bit that was flipped in transmission, if the user introduced an error: its byte, and its bit in the byte.
    error: Option<(usize, u8)>,
    received: Vec<u8>,
    received_computed: Computation,
    syndrome: u32,
    finding: Finding,
    corrected: Vec<u8>,
    corrected_crc: u32,
}

impl Transmission {
    /// What arrives when `sent` goes through a channel that flips `error`, and what the receiver makes of it.
    fn new(sent: &[u8], sent_crc: u32, error: Option<(usize, u8)>) -> Self {
        let mut received = sent.to_vec();
        if let Some((byte, bit)) = error {
            received[byte] ^= 1 << bit;
        }
        let received_computed = compute(&received);
        let syndrome = sent_crc ^ received_computed.crc;
        let finding = locate(sent.len(), syndrome);
        let mut corrected = received.clone();
        if let Finding::Data { byte, bit } = finding {
            corrected[byte] ^= 1 << bit;
        }
        let corrected_crc = compute(&corrected).crc;
        Self {
            sent: sent.to_vec(),
            sent_crc,
            error,
            received,
            received_computed,
            syndrome,
            finding,
            corrected,
            corrected_crc,
        }
    }

    /// What the error-position box says.
    fn position(&self) -> String {
        match self.finding {
            Finding::NoError => "none".to_owned(),
            Finding::Data { byte, bit } => format!("byte {byte}, bit {bit}"),
            Finding::Crc { bit } => format!("bit {bit} of the CRC"),
            Finding::Uncorrectable => "more than one bit".to_owned(),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The words of the diagram: what each box is, and how its values are worked out. Every number in them is a real one.
/// The sender's are worked out for the first byte of the text, so that a reader can follow one byte all the way through
/// by hand.
struct Words {
    input: String,
    bytes: String,
    index: String,
    entry: String,
    register: String,
    last: String,
    result: String,
    no_bytes: String,
    received: String,
    received_register: String,
    received_crc: String,
    syndrome: String,
    position: String,
    corrected: String,
}

/// The words for `bytes`, what was worked out for them, and what became of them in transmission.
fn words(bytes: &[u8], computed: &Computation, transmission: &Transmission) -> Words {
    let crc = computed.crc;
    let register = computed.register;
    let first = bytes.first().zip(computed.steps.first());
    let (index, entry, register_step) = match first {
        Some((&byte, step)) => (
            format!(
                "For the first byte, `{byte:02X}`, the register is `{INITIAL:08X}`, so its lowest byte is `{low:02X}`. \
                 `{byte:02X}` XOR `{low:02X}` = `{index:02X}`.\n\nFor each later byte, the register is the one that the \
                 byte before it made, in the box below.",
                low = INITIAL & 0xFF,
                index = step.index
            ),
            format!(
                "The first index is `{index:02X}`. Its entry starts as the number `{index:08X}`, and is shifted eight \
                 times:\n\n{shifts}\n\nThe last line, `{entry:08X}`, is the table's entry for `{index:02X}`. A program \
                 makes all 256 entries once, then looks them up.",
                index = step.index,
                shifts = shifts(step.index),
                entry = step.entry
            ),
            format!(
                "For the first byte, `{INITIAL:08X}` shifted right by eight places is `{shifted:08X}`. XOR the entry, \
                 `{entry:08X}` = `{new:08X}`.\n\nThe new register goes back to the top to make the next byte's index: \
                 its lowest byte is XORed with the next input byte.",
                shifted = INITIAL >> 8,
                entry = step.entry,
                new = step.register
            ),
        ),
        None => (String::new(), String::new(), String::new()),
    };

    let t = transmission;
    let n = bytes.len();
    let what_happened = match t.error {
        Some((byte, bit)) => format!(
            "You flipped bit {bit} of byte {byte} on the way, so that byte is marked. `{:02X}` became `{:02X}`.",
            t.sent[byte], t.received[byte]
        ),
        None => {
            "No error has been introduced, so these are the bytes that were sent. Use the fields and buttons at the top \
                 of the diagram to flip one bit."
                .to_owned()
        },
    };
    let differing = match t.error {
        Some((byte, _)) => format!(
            "The registers from byte {byte} on are marked, because they differ from the sender's. An error in one byte \
             changes the register for every byte after it."
        ),
        None => "They are the same as the sender's, because the bytes are the same.".to_owned(),
    };
    let found = match t.finding {
        Finding::NoError => "The syndrome is zero, so there is nothing to find.".to_owned(),
        Finding::Data { byte, bit } => format!(
            "Bit {bit} of byte {byte} gives `{:08X}`. That is the bit that was flipped.",
            t.syndrome
        ),
        Finding::Crc { bit } => format!(
            "A syndrome with a single 1, here bit {bit}, means that a bit of the CRC itself changed. The data is right."
        ),
        Finding::Uncorrectable => format!(
            "No single bit gives `{:08X}`, so more than one bit changed. CRC-32 **detects** that, but it cannot say which \
             bits to correct. Two flipped bits are never mistaken for one with data this short.",
            t.syndrome
        ),
    };
    let corrected = match t.finding {
        Finding::NoError => "There was nothing to correct, so these are the bytes that arrived.".to_owned(),
        Finding::Data { byte, bit } => format!(
            "Flipping bit {bit} of byte {byte} back makes `{:02X}` again, and gives the bytes that were sent. The CRC of \
             the corrected bytes is `{:08X}`, the CRC that was sent.",
            t.corrected[byte], t.corrected_crc
        ),
        Finding::Crc { .. } => "The data arrived right, so there is nothing to correct in it.".to_owned(),
        Finding::Uncorrectable => "The receiver cannot tell which bits to flip, so these are the bytes that arrived. \
                                   The error is detected but not corrected. A real system would ask for the data again."
            .to_owned(),
    };

    Words {
        input: format!(
            "### Text to checksum\nType any text. CRC-32 works on its **bytes**. It covers the first {INPUT_LEN} bytes, \
             and the note on the left says how many it has."
        ),
        bytes: "### Input bytes\nThe bytes of the text, as a hex dump. A UTF-8 character such as `\u{20AC}` occupies \
                more than one byte, but CRC-32 ignores this and looks at byte separately.\n\nThis is the **sender**."
            .to_owned(),
        index: format!(
            "### Table index\nThe calculation takes one byte at a time. It **XORs** the byte with the **lowest byte \
             of the register**. The result is a number from 0 to 255 that picks an entry in a table of 256.\n\n{index}"
        ),
        entry: format!(
            "### Table entry\nEach index has a ready-made entry. Shifting the index through a 32-bit register **one bit \
             at a time** makes it. Shift right by one place. If the bit that fell off the end was a **1**, XOR the \
             register with the polynomial `{POLYNOMIAL:08X}`. Do this eight times, once for each bit of the byte.\n\n{entry}"
        ),
        register: format!(
            "### Register after each byte\nThe new register is the old register **shifted right by a whole byte**, \
             eight places, XORed with the table's entry.\n\n{register_step}\n\nThe box on the right is the register \
             before the first byte. It is all ones, so that leading zero bytes still change the checksum."
        ),
        last: "### Register after the last byte\nWhen every byte has been shifted through, the register holds the \
               checksum **before** its last step. It is not the answer yet."
            .to_owned(),
        result: format!(
            "### CRC-32\nThe last step flips every bit of the register, by XORing it with the mask `{FINAL_MASK:08X}`. \
             `{register:08X}` XOR `{FINAL_MASK:08X}` = `{crc:08X}`.\n\nThat is the **CRC-32**. The sender sends it along \
             with the data, and the receiver checks it."
        ),
        no_bytes: format!(
            "### Initial register\nThere are no bytes, so nothing is shifted through the register. It stays `{INITIAL:08X}`, \
             all ones, and the final XOR turns it to `{crc:08X}`. Type some text to see the steps, and to send them."
        ),
        received: format!(
            "### Received bytes\nThe bytes that arrived. The channel can flip a bit on the way.\n\n{what_happened}"
        ),
        received_register: format!(
            "### Received register after each byte\nThe receiver does not trust the data. It works out the CRC of what \
             arrived, by the same calculation as the sender, one byte at a time.\n\n{differing}"
        ),
        received_crc: format!(
            "### CRC of the received bytes\nThe last register XOR `{FINAL_MASK:08X}` is `{:08X}`. The CRC that the \
             sender sent was `{:08X}`.",
            t.received_computed.crc, t.sent_crc
        ),
        syndrome: format!(
            "### Syndrome\nThe CRC travels with the data, so the receiver has its own copy of the CRC that was sent, on the \
             left. It XORs that with the CRC it worked out: `{:08X}` XOR `{:08X}` = \
             `{:08X}`.\n\nIf the data arrived unchanged, this is **zero**. Anything else means the data changed, and the \
             error is **detected**. The syndrome is more than a warning. It depends only on **where** the flipped bits \
             are, and not on what the data holds, so it can say **which** bit.",
            t.sent_crc, t.received_computed.crc, t.syndrome
        ),
        position: format!(
            "### Error position\nA polynomial this good gives each of the {bits} bits of {n} bytes a different syndrome, \
             and none is a single 1. So the receiver works out the syndrome of flipping each bit and looks for the one \
             that matches `{:08X}`.\n\n{found}\n\nBits are counted from 0 at the least significant bit of a byte.",
            t.syndrome,
            bits = n * 8
        ),
        corrected: format!("### Corrected bytes\n{corrected}"),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One box of a column, with the note that explains it, and perhaps another box that only helps it.
struct Spec {
    name: &'static str,
    content: DataNodeContent,
    explanation: String,
    /// A box that feeds this one, beside it and level with it, and its name.
    aside: Option<(&'static str, DataNodeContent)>,
}

/// Where the nodes of one column go: the boxes on a single vertical axis, so that each is centred over the one it
/// connects to, and the explanations in a column of their own.
struct Column<'a> {
    scene: &'a Scene,
    /// The x of the axis the boxes are centred on.
    axis: f64,
    /// The x of the left edge of the explanations, and how wide they are.
    explain_left: f64,
    explanation_width: f64,
    /// The x of the middle of the boxes that only help.
    side: f64,
    nodes: Vec<NodeId>,
}

impl Column<'_> {
    /// How tall the row for `spec` is: the tallest of its box, its explanation and its aside.
    fn height(&self, spec: &Spec) -> Result<f64, String> {
        let size = self
            .scene
            .measure_named_data_node(spec.name, &spec.content)
            .map_err(stringify)?;
        let words = self
            .scene
            .measure_markdown_node(self.explanation_width, &spec.explanation)
            .map_err(stringify)?;
        let aside = match &spec.aside {
            Some((name, content)) => self.scene.measure_named_data_node(name, content).map_err(stringify)?.height,
            None => 0.0,
        };
        Ok(size.height.max(words.height).max(aside))
    }

    /// Adds the row for `spec` with its top at `top` and `height` tall, everything in it centred on the middle. Returns
    /// the ids of the box and of the aside.
    fn place(&mut self, top: f64, height: f64, spec: Spec) -> Result<(NodeId, Option<NodeId>), String> {
        let middle = top + height / 2.0;
        let size = self
            .scene
            .measure_named_data_node(spec.name, &spec.content)
            .map_err(stringify)?;
        let node = self
            .scene
            .add_named_data_node(
                Point::new(self.axis - size.width / 2.0, middle - size.height / 2.0),
                spec.name,
                spec.content,
            )
            .map_err(stringify)?;
        self.explain(middle, &spec.explanation)?;
        self.nodes.push(node);
        let aside = match spec.aside {
            Some((name, content)) => {
                let size = self.scene.measure_named_data_node(name, &content).map_err(stringify)?;
                let id = self
                    .scene
                    .add_named_data_node(
                        Point::new(self.side - size.width / 2.0, middle - size.height / 2.0),
                        name,
                        content,
                    )
                    .map_err(stringify)?;
                self.nodes.push(id);
                Some(id)
            },
            None => None,
        };
        Ok((node, aside))
    }

    /// Adds `explanation` in the column of explanations, centred on `middle`.
    fn explain(&mut self, middle: f64, explanation: &str) -> Result<(), String> {
        let size = self
            .scene
            .measure_markdown_node(self.explanation_width, explanation)
            .map_err(stringify)?;
        let id = self
            .scene
            .add_markdown_node(
                Point::new(self.explain_left, middle - size.height / 2.0),
                self.explanation_width,
                explanation,
            )
            .map_err(stringify)?;
        self.nodes.push(id);
        Ok(())
    }

    /// How tall an XOR operator node with `result` and its `explanation` is.
    fn operator_height(&self, result: &DataNodeContent, explanation: &str) -> Result<f64, String> {
        let size = self.scene.measure_operator_box("XOR", result).map_err(stringify)?;
        let words = self
            .scene
            .measure_markdown_node(self.explanation_width, explanation)
            .map_err(stringify)?;
        Ok(size.height.max(words.height))
    }

    /// Adds an XOR operator node of `inputs`, with its top at `top` and `height` tall, and its explanation.
    fn place_operator(
        &mut self,
        top: f64,
        height: f64,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
        explanation: &str,
    ) -> Result<NodeId, String> {
        let middle = top + height / 2.0;
        let size = self.scene.measure_operator_box("XOR", &result).map_err(stringify)?;
        let node = self
            .scene
            .add_binary_operator_node(
                Point::new(self.axis - size.width / 2.0, middle - size.height / 2.0),
                BinaryOperator::Xor,
                inputs,
                result,
            )
            .map_err(stringify)?;
        self.explain(middle, explanation)?;
        self.nodes.push(node);
        Ok(node)
    }
}

/// A single 32-bit value written in hexadecimal.
fn word(value: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(vec![value]), DataFormat::Hexadecimal)
}

/// Several 32-bit values written in hexadecimal, `WORD_COLUMNS` to a row.
fn array_of_words(values: Vec<u32>) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(WORD_COLUMNS))
}

/// `bytes` as a hex dump with offsets.
fn hexdump(bytes: &[u8]) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::U8(bytes.to_vec()),
        DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
    )
}

/// `text` as a single run of text in a box. Only printable ASCII can be drawn this way.
fn plain(text: &str) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(text.as_bytes().to_vec()), DataFormat::PlainText)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `text`, replacing whatever scene was there, and returns the new text input node. It keeps the zoom
/// and pan the old scene had.
///
/// The library cannot change how many values a node holds, so the scene is drawn again for each change, with exactly as
/// many values as the text has bytes. An empty text has no bytes to shift through the register, so it has only the
/// sender's initial register and the final steps, and nothing to send.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
fn build(text: &str) -> Result<NodeId, String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let bytes = utf8_prefix(text, INPUT_LEN);
    let computed = compute(bytes);

    // An error can only be in a byte that exists. One left over from longer text is dropped.
    let error = ERROR.get().filter(|&(byte, _)| byte < bytes.len());
    ERROR.set(error);
    let transmission = Transmission::new(bytes, computed.crc, error);
    let words = words(bytes, &computed, &transmission);
    let complaints = COMPLAINTS.with_borrow(Clone::clone);
    let status = status_text(error, &complaints);

    // Both columns are as wide as their widest box. The axis of each is half of that from its left edge.
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
    let result = word(computed.crc);
    let xor_size = scene.measure_operator_box("XOR", &result).map_err(stringify)?;
    // The gap between the columns holds the XOR that makes the CRC, with a clear lane at its middle for the bytes to
    // cross, so it is twice as wide as the XOR and the space that it needs on both sides.
    let channel_width = 2.0 * (xor_size.width.max(mask.width) + 2.0 * GAP);
    let aside_width = initial.width;
    let sender_axis = MARGIN + SENDER_EXPLANATION_WIDTH + GAP + aside_width + GAP + middle_width / 2.0;
    let mut sender = Column {
        scene: &scene,
        axis: sender_axis,
        explain_left: MARGIN,
        explanation_width: SENDER_EXPLANATION_WIDTH,
        side: MARGIN + SENDER_EXPLANATION_WIDTH + GAP + aside_width / 2.0,
        nodes: Vec::new(),
    };
    let receiver_axis = sender_axis + middle_width + channel_width;
    let mut receiver = Column {
        scene: &scene,
        axis: receiver_axis,
        explain_left: receiver_axis + middle_width / 2.0 + GAP,
        explanation_width: RECEIVER_EXPLANATION_WIDTH,
        side: receiver_axis,
        nodes: Vec::new(),
    };

    // The first row: the text box, with what it holds that the checksum leaves out on its right, and what it is on the
    // left. The text box is a library node whose height is not known until it is drawn, so the row is started low
    // enough for the taller note, and the notes are centred on the box once it is there.
    let note_text = length_note(text.len(), bytes.len(), INPUT_LEN);
    let note_size = scene.measure_markdown_node(NOTE_WIDTH, &note_text).map_err(stringify)?;
    let words_size = scene
        .measure_markdown_node(SENDER_EXPLANATION_WIDTH, &words.input)
        .map_err(stringify)?;
    let tallest = note_size.height.max(words_size.height);
    let top = MARGIN + (tallest - TYPICAL_INPUT_HEIGHT).max(0.0) / 2.0;
    let input = scene
        .add_text_input_node(
            Point::new(sender_axis - input_width / 2.0, top),
            input_width,
            "Text to checksum",
            text,
            |_, id, text| typed(id, text),
        )
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input).map_err(stringify)?;
    let middle = input_rect.origin.y + input_rect.size.height / 2.0;
    let note = scene
        .add_markdown_node(
            Point::new(
                input_rect.origin.x + input_rect.size.width + GAP / 3.0,
                middle - note_size.height / 2.0,
            ),
            NOTE_WIDTH,
            &note_text,
        )
        .map_err(stringify)?;
    sender.nodes.extend([input, note]);
    sender.explain(middle, &words.input)?;

    // The controls for introducing an error are to the right of the note, level with the text box: a field for the bit
    // and one for the byte, two buttons, and a line that says what has been done. The fields are nodes. The buttons are
    // drawn as SVG in the scene's own layer, so they zoom and pan with it.
    let note_rect = scene.node_rect(note).map_err(stringify)?;
    let mut control_left = note_rect.origin.x + note_rect.size.width + CONTROL_GAP;
    for (index, (label, store)) in FIELD_LABELS.into_iter().zip([&FLIP_BIT, &FLIP_BYTE]).enumerate() {
        let value = store.with_borrow(Clone::clone);
        let field = scene
            .add_text_input_node(
                Point::new(control_left, input_rect.origin.y),
                FIELD_WIDTH,
                label,
                &value,
                move |_, _, text| {
                    store.with_borrow_mut(|stored| *stored = text.to_owned());
                    // Editing a field that was marked wrong takes the mark off it, until the next button press.
                    clear_complaint(index);
                },
            )
            .map_err(stringify)?;
        sender.nodes.push(field);
        control_left += FIELD_WIDTH + CONTROL_GAP;
    }
    let layer = document
        .query_selector(&format!("#{DIAGRAM_ID} > g.svg-dom-graph-content"))
        .map_err(|e| format!("could not look for the scene's layer: {e:?}"))?
        .ok_or("the scene has no content layer")?;
    for (label, action) in [("Introduce the error", "introduce"), ("Remove the error", "remove")] {
        add_button(
            &document,
            &layer,
            Point::new(control_left, middle - BUTTON_HEIGHT / 2.0),
            label,
            action,
        )?;
        control_left += BUTTON_WIDTH + CONTROL_GAP;
    }
    let status_size = scene.measure_markdown_node(STATUS_WIDTH, &status).map_err(stringify)?;
    let status_node = scene
        .add_markdown_node(
            Point::new(control_left, middle - status_size.height / 2.0),
            STATUS_WIDTH,
            &status,
        )
        .map_err(stringify)?;
    sender.nodes.push(status_node);
    let mut y = (input_rect.origin.y + input_rect.size.height)
        .max(middle + tallest / 2.0)
        .max(top + TYPICAL_INPUT_HEIGHT)
        + ROW_GAP;

    // Connectors: the sender goes down, and the receiver goes up.
    let downward = ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North));
    let upward = ConnectorOptions::default()
        .with_from_side(Some(Side::North))
        .with_to_side(Some(Side::South));
    let elbow = ConnectorType::Elbow { corner_radius: 12.0 };

    let mut dump = None;
    let last_register = if bytes.is_empty() {
        // Nothing is shifted through the register, so the initial register is the register after the last byte.
        let spec = Spec {
            name: "Initial register",
            content: word(INITIAL),
            explanation: words.no_bytes.clone(),
            aside: None,
        };
        let height = sender.height(&spec)?;
        let (initial, _) = sender.place(y, height, spec)?;
        y += height + ROW_GAP;
        scene.add_edge_with(input, initial, downward).map_err(stringify)?;
        initial
    } else {
        let mut previous = input;
        let mut register = input;
        for (name, content, explanation, aside) in [
            ("Input bytes", hexdump(bytes), &words.bytes, None),
            (
                "Table index",
                DataNodeContent::new(
                    NodeValues::U8(computed.steps.iter().map(|s| s.index).collect()),
                    DataFormat::Hexadecimal,
                )
                .with_layout(GridLayout::Columns(INDEX_COLUMNS)),
                &words.index,
                None,
            ),
            (
                "Table entry",
                array_of_words(computed.steps.iter().map(|s| s.entry).collect()),
                &words.entry,
                None,
            ),
            (
                "Register after each byte",
                array_of_words(computed.steps.iter().map(|s| s.register).collect()),
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
            let height = sender.height(&spec)?;
            let (id, aside) = sender.place(y, height, spec)?;
            y += height + ROW_GAP;
            scene.add_edge_with(previous, id, downward).map_err(stringify)?;
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
            }
            previous = id;
            register = id;
        }
        register
    };

    // The receiver's rows, from the top. They are listed here because the sender's last row has to be low enough for the
    // two rows that go above the syndrome.
    let receiver_rows = if bytes.is_empty() {
        None
    } else {
        let t = &transmission;
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
        // corrected, position, syndrome, CRC, register, received
        let heights = [
            receiver.height(&corrected_row)?,
            receiver.height(&position_row)?,
            receiver.operator_height(&syndrome_content, &words.syndrome)?,
            receiver.height(&crc_row)?,
            receiver.height(&register_row)?,
            receiver.height(&received_dump)?,
        ];
        Some((
            heights,
            syndrome_content,
            [corrected_row, position_row, crc_row, register_row, received_dump],
        ))
    };

    // The last row of the sender: the register after the last byte, with the XOR that flips it on its right, level with
    // it, in the gap between the columns. The mask is under the XOR.
    let last_spec = Spec {
        name: "Register after the last byte",
        content: word(computed.register),
        explanation: format!("{}\n\n{}", words.last, words.result),
        aside: None,
    };
    let last_height = sender.height(&last_spec)?.max(xor_size.height);
    let row_top = y;
    let middle = row_top + last_height / 2.0;
    let (last, _) = sender.place(row_top, last_height, last_spec)?;
    if last_register != last {
        scene.add_edge_with(last_register, last, downward).map_err(stringify)?;
    }
    let xor_left = sender_axis + middle_width / 2.0 + GAP;
    let mask_top = (row_top + last_height).max(middle + xor_size.height / 2.0) + ROW_GAP;
    let mask = scene
        .add_named_data_node(
            Point::new(xor_left + (xor_size.width - mask.width) / 2.0, mask_top),
            "Final mask",
            word(FINAL_MASK),
        )
        .map_err(stringify)?;
    let sent = scene
        .add_binary_operator_node(
            Point::new(xor_left, middle - xor_size.height / 2.0),
            BinaryOperator::Xor,
            (last, mask),
            result,
        )
        .map_err(stringify)?;
    sender.nodes.extend([mask, sent]);

    let mut nodes = sender.nodes;
    if let Some((heights, syndrome_content, [corrected_row, position_row, crc_row, register_row, received_dump])) =
        receiver_rows
    {
        let t = &transmission;
        // The receiver's column starts level with the sender's input bytes, so that the corrected bytes are on the same row
        // as the bytes that were sent, and can be compared with them at a glance. Everything under them follows.
        let dump_rect = scene
            .node_rect(dump.ok_or("the input bytes were not drawn")?)
            .map_err(stringify)?;
        let dump_middle = dump_rect.origin.y + dump_rect.size.height / 2.0;
        let mut tops = [0.0; 6];
        tops[0] = dump_middle - heights[0] / 2.0;
        for i in 1..6 {
            tops[i] = tops[i - 1] + heights[i - 1] + ROW_GAP;
        }

        let (received_id, _) = receiver.place(tops[5], heights[5], received_dump)?;
        let (register_id, _) = receiver.place(tops[4], heights[4], register_row)?;
        let (crc_id, _) = receiver.place(tops[3], heights[3], crc_row)?;
        // The CRC travels with the data, so the receiver has its own copy of it. It is level with the syndrome, on its
        // left, so that it enters the syndrome at its west edge, and the receiver's own CRC enters at its south edge. The
        // CRC reaches the copy from the sender's XOR, straight up the gap between the columns.
        let copy_content = word(t.sent_crc);
        let copy_size = scene
            .measure_named_data_node("CRC that was sent", &copy_content)
            .map_err(stringify)?;
        let syndrome_middle = tops[2] + heights[2] / 2.0;
        let copy = scene
            .add_named_data_node(
                Point::new(
                    xor_left + (xor_size.width - copy_size.width) / 2.0,
                    syndrome_middle - copy_size.height / 2.0,
                ),
                "CRC that was sent",
                copy_content,
            )
            .map_err(stringify)?;
        nodes.push(copy);
        let syndrome_id =
            receiver.place_operator(tops[2], heights[2], (copy, crc_id), syndrome_content, &words.syndrome)?;
        scene.add_edge_with(sent, copy, upward).map_err(stringify)?;
        let (position_id, _) = receiver.place(tops[1], heights[1], position_row)?;
        let (corrected_id, _) = receiver.place(tops[0], heights[0], corrected_row)?;

        // The bytes cross from the sender to the receiver, and the error is introduced on the way across. The edge is an
        // elbow, and its upright part is in the middle of the gap, clear of the XOR.
        if let Some(dump) = dump {
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
        }
        scene.add_edge_with(received_id, register_id, upward).map_err(stringify)?;
        scene.add_edge_with(register_id, crc_id, upward).map_err(stringify)?;
        scene.add_edge_with(syndrome_id, position_id, upward).map_err(stringify)?;
        scene.add_edge_with(position_id, corrected_id, upward).map_err(stringify)?;
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

        // The bytes that changed, and the registers that differ because of them, are marked. So is the bit that is
        // corrected.
        if let Some((byte, _)) = t.error {
            scene.set_secondary_selection(received_id, &[byte]).map_err(stringify)?;
            let differ: Vec<usize> = t
                .received_computed
                .steps
                .iter()
                .zip(&computed.steps)
                .enumerate()
                .filter(|(_, (received, sent))| received.register != sent.register)
                .map(|(i, _)| i)
                .collect();
            scene.set_secondary_selection(register_id, &differ).map_err(stringify)?;
        }
        if let Finding::Data { byte, .. } = t.finding {
            scene.set_secondary_selection(corrected_id, &[byte]).map_err(stringify)?;
        }
        nodes.extend(receiver.nodes);
    }

    // The canvas is as wide and tall as its nodes need, with room east of them for the zoom toolbar.
    let (mut right, mut lowest) = (0.0_f64, 0.0_f64);
    for &node in &nodes {
        let rect = scene.node_rect(node).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
        lowest = lowest.max(rect.origin.y + rect.size.height);
    }
    let size = Size::new(right + MARGIN + TOOLBAR_ROOM, lowest + MARGIN);
    resize_svg(&document, DIAGRAM_ID, size)?;
    // The view box stays as large as the scene, and the element is made smaller than it. The browser then draws all of
    // it, text included, scaled down to fit.
    let diagram = required_element(&document, DIAGRAM_ID)?;
    for (name, value) in [("width", size.width * SCALE), ("height", size.height * SCALE)] {
        diagram
            .set_attribute(name, &value.to_string())
            .map_err(|e| format!("could not scale #{DIAGRAM_ID}: {e:?}"))?;
    }
    scene.refresh_layout().map_err(stringify)?;

    // A field that was wrong when a button was pressed is marked, in the colour for what is wrong with it.
    for (label, complaint) in FIELD_LABELS.into_iter().zip(&complaints) {
        mark_field(&document, label, complaint.as_ref().map(|c| c.problem));
    }

    // Nothing is made draggable: the boxes are placed to line up, and the explanations sit beside them.
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene,
            text: text.to_owned(),
            input,
        });
    });
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box now holds `text`. Draws the scene again for it, unless the user is composing text with an input method
/// editor. The scene is replaced along with its text box, and a composition does not survive that. So the scene waits
/// until the composition ends: see [`composition_ended`].
fn typed(id: NodeId, text: &str) {
    if !COMPOSING.get() {
        redraw(id, text);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again for `text`, then gives the new text box the keyboard focus, with the caret where it was, so
/// that typing carries on in it. A failure is logged to the console, since the text box's own callback has nowhere to
/// return it to.
fn redraw(id: NodeId, text: &str) {
    let caret = DEMO.with_borrow(|slot| slot.as_ref().and_then(|d| d.scene.text_input_caret(id).ok()));
    let result = build(text).and_then(|input| {
        DEMO.with_borrow(|slot| match slot {
            Some(demo) => demo.scene.focus_text_input(input, caret).map_err(stringify),
            None => Ok(()),
        })
    });
    if let Err(e) = result {
        web_sys::console::error_1(&format!("Could not draw the text: {e}").into());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The user has finished composing. Draws the scene for the text the text box holds now, if it is not the text the scene
/// was drawn for. The browser can send the last `input` event either before or after `compositionend`, so this looks
/// once the event has been handled, and draws nothing if the `input` event has already done so.
fn composition_ended() {
    let value = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(TEXT_BOX).ok().flatten())
        .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value());
    let input = DEMO.with_borrow(|slot| slot.as_ref().filter(|d| Some(&d.text) != value.as_ref()).map(|d| d.input));
    if let (Some(input), Some(value)) = (input, value) {
        redraw(input, &value);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Watches the stage for the start and end of a composition. Composition events bubble, and the stage outlives every
/// scene that is drawn in it, so these two listeners are attached once.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
fn watch_composition(document: &web_sys::Document) -> Result<(), String> {
    let stage = required_element(document, STAGE_ID)?;
    let started = Closure::<dyn FnMut()>::new(|| COMPOSING.set(true));
    let ended = Closure::<dyn FnMut()>::new(|| {
        COMPOSING.set(false);
        // Not at once: the text box may not hold the composed text until the events that follow have been handled.
        if let Some(window) = web_sys::window() {
            let later = Closure::once_into_js(composition_ended);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
        }
    });
    for (name, listener) in [("compositionstart", &started), ("compositionend", &ended)] {
        stage
            .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach the {name} listener: {e:?}"))?;
    }
    started.forget();
    ended.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again for the text it was drawn for, with the error as it is now, and says whether that worked. The
/// text box is replaced, and so is the button that was pressed. So the new button of the same `action` is given the
/// keyboard focus, as the text box is given it after a change of text. Without that, a keyboard user would find that
/// focus had gone back to the start of the page.
fn redraw_current(action: &str) -> bool {
    let text = DEMO.with_borrow(|slot| slot.as_ref().map_or_else(|| INITIAL_TEXT.to_owned(), |d| d.text.clone()));
    if let Err(e) = build(&text) {
        web_sys::console::error_1(&format!("Could not draw the error: {e}").into());
        return false;
    }
    let button = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| {
            d.query_selector(&format!("#{DIAGRAM_ID} g[data-action=\"{action}\"]"))
                .ok()
                .flatten()
        })
        .and_then(|button| button.dyn_into::<web_sys::SvgElement>().ok());
    if let Some(button) = button {
        let _ = button.focus();
    }
    true
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Says `message` to assistive technology, in a live region that stays on the page. The scene is drawn again for each
/// change, and a live region that is drawn along with it is new each time, which a screen reader may not announce. So
/// this one is in the window page, outside the scene.
fn announce(message: &str) {
    if let Some(region) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(ANNOUNCEMENT_ID))
    {
        region.set_text_content(Some(message));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the status line says: what is wrong with the fields if anything is, which is what the user needs to know first,
/// and otherwise what has been done to the data.
fn status_text(error: Option<(usize, u8)>, complaints: &[Option<Complaint>; 2]) -> String {
    let problems: Vec<&str> = complaints.iter().flatten().map(|c| c.message.as_str()).collect();
    if problems.is_empty() { error_status(error) } else { problems.join(" ") }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the status line says about `error`.
fn error_status(error: Option<(usize, u8)>) -> String {
    match error {
        Some((byte, bit)) => format!("Bit {bit} of byte {byte} is flipped in transmission."),
        None => "No error has been introduced.".to_owned(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks the field called `label` as wrong in the way `problem` says, or takes the mark off if there is none. The mark
/// is a fill on the node's box, and `aria-invalid` on its text box, so that it does not depend on colour. The fill the
/// box had is kept in `data-original-fill`, to put back.
fn mark_field(document: &web_sys::Document, label: &str, problem: Option<Problem>) {
    let Ok(Some(group)) = document.query_selector(&format!("#{DIAGRAM_ID} g[aria-label^=\"{label}\"]")) else {
        return;
    };
    if let Ok(Some(input)) = group.query_selector("input") {
        let _ = match problem {
            Some(_) => input.set_attribute("aria-invalid", "true"),
            None => input.remove_attribute("aria-invalid"),
        };
    }
    let Ok(Some(rect)) = group.query_selector("rect") else { return };
    match problem {
        Some(problem) => {
            if !rect.has_attribute("data-original-fill") {
                let _ = rect.set_attribute("data-original-fill", &rect.get_attribute("fill").unwrap_or_default());
            }
            let fill = match problem {
                Problem::NotANumber => NOT_A_NUMBER_FILL,
                Problem::OutOfRange => OUT_OF_RANGE_FILL,
            };
            let _ = rect.set_attribute("fill", fill);
        },
        None => {
            if let Some(original) = rect.get_attribute("data-original-fill") {
                let _ = rect.set_attribute("fill", &original);
                let _ = rect.remove_attribute("data-original-fill");
            }
        },
    }
}

/// The field `index` of [`FIELD_LABELS`] has been edited, so what was wrong with it no longer is known to be.
fn clear_complaint(index: usize) {
    let had = COMPLAINTS.with_borrow_mut(|complaints| complaints[index].take().is_some());
    if had && let Some(document) = web_sys::window().and_then(|w| w.document()) {
        mark_field(&document, FIELD_LABELS[index], None);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flips the bit that the two fields choose. Both are checked first. A field that is wrong is marked, the status line and
/// the live region say what is wrong with it, and no bit is flipped. Nothing is flipped in an empty text, which has no
/// bytes.
fn introduce_error() {
    let length = DEMO.with_borrow(|slot| slot.as_ref().map_or(0, |d| utf8_prefix(&d.text, INPUT_LEN).len()));
    if length == 0 {
        announce("There is no text to flip a bit in.");
        return;
    }
    let chosen = FLIP_BIT.with_borrow(|bit| FLIP_BYTE.with_borrow(|byte| bit_and_byte(bit, byte, length)));
    match chosen {
        Ok((byte, bit)) => {
            COMPLAINTS.with_borrow_mut(|complaints| *complaints = [None, None]);
            ERROR.set(Some((byte, bit)));
            if redraw_current("introduce") {
                announce(&error_status(Some((byte, bit))));
            }
        },
        Err(wrong) => {
            let message = status_text(None, &wrong);
            COMPLAINTS.with_borrow_mut(|complaints| *complaints = wrong);
            if redraw_current("introduce") {
                announce(&message);
            }
        },
    }
}

/// Puts the flipped bit back.
fn remove_error() {
    COMPLAINTS.with_borrow_mut(|complaints| *complaints = [None, None]);
    ERROR.set(None);
    if redraw_current("remove") {
        announce(&error_status(None));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a button in the scene's own layer `layer`, at `top_left`, that `watch_buttons` will find by its `action`. It is
/// a rectangle with a label, in a group that has the role of a button and can take the keyboard focus.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
fn add_button(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    top_left: Point,
    label: &str,
    action: &str,
) -> Result<(), String> {
    let make = |tag: &str, attributes: &[(&str, String)]| -> Result<web_sys::Element, String> {
        let element = document
            .create_element_ns(Some(SVG_NAMESPACE), tag)
            .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
        for (name, value) in attributes {
            element
                .set_attribute(name, value)
                .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
        }
        Ok(element)
    };
    let group = make(
        "g",
        &[
            ("role", "button".to_owned()),
            ("tabindex", "0".to_owned()),
            ("aria-label", label.to_owned()),
            ("data-action", action.to_owned()),
            ("style", "cursor: pointer;".to_owned()),
        ],
    )?;
    let rect = make(
        "rect",
        &[
            ("x", top_left.x.to_string()),
            ("y", top_left.y.to_string()),
            ("width", BUTTON_WIDTH.to_string()),
            ("height", BUTTON_HEIGHT.to_string()),
            ("rx", "6".to_owned()),
            ("fill", "#eef4ff".to_owned()),
            ("stroke", "#3b5b9a".to_owned()),
            ("stroke-width", "1.5".to_owned()),
        ],
    )?;
    let text = make(
        "text",
        &[
            ("x", (top_left.x + BUTTON_WIDTH / 2.0).to_string()),
            ("y", (top_left.y + BUTTON_HEIGHT / 2.0).to_string()),
            ("text-anchor", "middle".to_owned()),
            ("dominant-baseline", "central".to_owned()),
            ("font-size", "14".to_owned()),
            ("fill", "#1a1a1a".to_owned()),
            ("style", "pointer-events: none; user-select: none;".to_owned()),
        ],
    )?;
    text.set_text_content(Some(label));
    for (parent, child) in [(&group, &rect), (&group, &text), (layer, &group)] {
        parent
            .append_child(child)
            .map_err(|e| format!("could not add the button: {e:?}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons drawn by [`add_button`] work. One listener for clicks and one for the Enter and Space keys are on
/// the stage, which outlives every scene that is drawn in it, and find the button by its `data-action`. So a scene
/// drawn again needs no listeners of its own.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
fn watch_buttons(document: &web_sys::Document) -> Result<(), String> {
    let stage = required_element(document, STAGE_ID)?;
    let press = |target: Option<web_sys::EventTarget>| {
        let action = target
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-action]").ok().flatten())
            .and_then(|button| button.get_attribute("data-action"));
        match action.as_deref() {
            Some("introduce") => introduce_error(),
            Some("remove") => remove_error(),
            _ => {},
        }
    };
    let clicked = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| press(event.target()));
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" || event.key() == " " {
            let on_button = event
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e| e.closest("[data-action]").ok().flatten().is_some());
            if on_button {
                event.prevent_default();
                press(event.target());
            }
        }
    });
    stage
        .add_event_listener_with_callback("click", clicked.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the click listener: {e:?}"))?;
    stage
        .add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the keydown listener: {e:?}"))?;
    clicked.forget();
    key.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the CRC-32 scene in `crc32-window.html`, and fits the window to it. The sender's calculation goes down the
/// page, and the receiver's goes back up. The diagram is scaled down to 85%, text and all, since it is the largest of the
/// demos.
///
/// The sender's column, from the top:
///
/// 1. The text input node holds the text to checksum. Each change works the checksum out again.
/// 2. The bytes of the text as a hex dump, and under it the table index of each byte: the low byte of the register
///    XORed with the input byte.
/// 3. The register starts as all ones. For each byte, the table's entry at its index is XORed with the register shifted
///    right by a byte, to make the register after that byte. The table's entry is the result of shifting one byte
///    through the register a bit at a time, XORing in the polynomial `0xEDB88320` each time a one falls out. The note
///    shows those eight shifts for the first byte.
/// 4. After the last byte, the register is XORed with all ones to flip its bits. That is the CRC-32.
///
/// The receiver's column, from the bottom, so that the data flows up it. Its top row, the corrected bytes, is level with
/// the sender's input bytes, to compare the two:
///
/// 5. The bytes that arrived. The user can flip one bit of them on the way, with the two fields and the two buttons to the right of the length
///    note at the top of the diagram.
/// 6. The receiver works out the CRC of what arrived, by the same calculation.
/// 7. The syndrome is the CRC that was sent XORed with the CRC the receiver worked out. Zero means no error.
/// 8. Every single flipped bit of the data gives a syndrome of its own, and it does not depend on what the data holds.
///    The receiver finds the bit whose syndrome matches. A syndrome of a single one is a flipped bit of the CRC itself.
///    A syndrome that no single bit gives means that more bits changed: that is detected, and cannot be corrected.
/// 9. The bit is flipped back, and the corrected bytes have the CRC that was sent.
///
/// The boxes of each column are on one vertical axis, so each is centred over the one it connects to. A box that only
/// helps, such as the initial register and the final mask, is beside the one it feeds and level with it. The notes are
/// on the outside. Nothing is draggable, since the layout is the point.
///
/// Every value is worked out in plain Rust by [`compute`] and [`locate`]. The library does no checksumming. A node holds
/// the same number of values for as long as it lives, so each change draws the scene again, as the Base64 demo does.
/// Text past [`INPUT_LEN`] bytes is not covered by the checksum, and the cut falls between characters.
///
/// A scene is not drawn while the user is composing text with an input method editor, since drawing it replaces the
/// text box and ends the composition. It is drawn when the composition ends.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
pub(crate) fn build_crc32_window() -> Result<(), String> {
    let document = document()?;
    watch_composition(&document)?;
    watch_buttons(&document)?;
    build(INITIAL_TEXT)?;
    fit_window_to_stage(DIAGRAM_ID)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The CRC-32 scene is the largest of the demos. So the gallery panel keeps only the description, plus a button that
/// opens the scene in a window of its own: `crc32-window.html`, which runs [`build_crc32_window`] there. A named window
/// is reused. Pressing the button again focuses the one already open.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#crc32-open`, or if a listener could not be attached to it.
pub(crate) fn build_crc32_demo() -> Result<(), String> {
    wire_open_window(&document()?, "crc32-open", WINDOW_PAGE, WINDOW_NAME)
}

/// The page [`build_crc32_demo`]'s button opens.
const WINDOW_PAGE: &str = "crc32-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-crc32";

#[cfg(test)]
mod unit_tests;
