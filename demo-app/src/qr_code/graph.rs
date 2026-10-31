//! The graph of the scene: one node for each stage of the encoding, joined by the edges that the data flows along.
//!
//! Every node has the same number of cells for every input, so the graph is built once. Stepping, and typing new text,
//! only change what the nodes hold and which of their cells are dimmed, with calls that rewrite just what differs.

use super::snapshots::{
    BITSTREAM_CELLS, BYTE_CELLS, Content, DemoSnapshot, EDGES, EdgeKey, FORMAT_CELLS, NodeKey, SCORE_CELLS,
    SCORE_COLUMNS,
};
use demo_common::util::{document, stringify};
use svg_dom::root::utils::{Point, Rect};
use svg_dom_graph::{
    EdgeId, NodeId,
    scene::{
        BitValues, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, GridLayout, HexDumpFormat,
        LabellingStyle, NodeValues, Scene, Side,
    },
};
use wasm_bindgen::JsCast;

/// The width of the text input node, and the gaps between the nodes.
const INPUT_WIDTH: f64 = 380.0;
const GAP_X: f64 = 50.0;
const GAP_Y: f64 = 60.0;

/// The input node's name, and the wording of every other node's.
const INPUT_LABEL: &str = "ASCII text";
const BYTES_NAME: &str = "Input bytes";
const BITSTREAM_NAME: &str = "Bitstream";
const DATA_NAME: &str = "Data codewords";
const DIVISION_NAME: &str = "Division: data byte, factor";
const ERROR_CORRECTION_NAME: &str = "Error correction codewords";
const CODEWORDS_NAME: &str = "Codewords";
const SCORES_NAME: &str = "Mask scores: N1, N2, N3, N4, total";
const FORMAT_NAME: &str = "Format information";

/// The label of each row of the scores.
const MASK_LABELS: &[&str] = &["Mask 0", "Mask 1", "Mask 2", "Mask 3", "Mask 4", "Mask 5", "Mask 6", "Mask 7"];

/// How many bits wide a row of the bitstream is, and so a group of 8 is one codeword.
const BITSTREAM_COLUMNS: usize = 32;

/// The sides each edge leaves and arrives on, in the order of [`EDGES`].
const SIDES: [(Side, Side); 9] = [
    (Side::East, Side::West),
    (Side::South, Side::North),
    (Side::South, Side::North),
    (Side::East, Side::West),
    (Side::East, Side::West),
    (Side::South, Side::North),
    (Side::South, Side::North),
    (Side::South, Side::North),
    (Side::East, Side::West),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nodes and edges of the scene, and where they end.
pub(super) struct Graph {
    nodes: Vec<(NodeKey, NodeId)>,
    edges: Vec<(EdgeKey, EdgeId)>,
    /// The right edge of the right-most node, and the bottom edge of the lowest.
    pub(super) right: f64,
    pub(super) bottom: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far along the width of the node `name` the middle of its cells `first` to `last` is, from 0 to 1, of the `cells`
/// cells that it has as two hexadecimal digits each. It is read from where the digits are on the page, since the width
/// of a cell and the gaps between groups are the library's own. `None` if the node cannot be found, has not as many
/// digits as cells, or has no width, as when the page is hidden.
fn cell_span_fraction(
    document: &web_sys::Document,
    name: &str,
    first: usize,
    last: usize,
    cells: usize,
) -> Option<f64> {
    let group = document.query_selector(&format!("g[aria-label^=\"{name}\"]")).ok()??;
    let list = group.query_selector_all("text").ok()?;
    let mut centres = Vec::with_capacity(cells);
    for index in 0..list.length() {
        let element = list.get(index)?.dyn_into::<web_sys::Element>().ok()?;
        let text = element.text_content()?;
        if text.len() == 2 && text.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)) {
            let rect = element.get_bounding_client_rect();
            centres.push(rect.left() + rect.width() / 2.0);
        }
    }
    let node = group.get_bounding_client_rect();
    if centres.len() != cells || node.width() <= 0.0 {
        return None;
    }
    let middle = (centres[first] + centres[last]) / 2.0;
    Some(((middle - node.left()) / node.width()).clamp(0.0, 1.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn hex_row(values: Vec<u8>, columns: usize) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(values), DataFormat::Hexadecimal).with_layout(GridLayout::Columns(columns))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn bits(values: &[u8]) -> Result<NodeValues, String> {
    Ok(NodeValues::Bits(BitValues::new(1, values.to_vec()).map_err(stringify)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bytes as `hexdump -C` shows them, with each line's offset. A line is 16 bytes, and the unused places are zeros
/// that the scene dims, so the node has 17 bytes whatever the text.
fn bytes_content(content: &Content) -> DataNodeContent {
    debug_assert_eq!(content.bytes.len(), BYTE_CELLS);
    DataNodeContent::new(
        NodeValues::U8(content.bytes.to_vec()),
        DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn bitstream_content(content: &Content) -> Result<DataNodeContent, String> {
    Ok(DataNodeContent::new(bits(&content.bitstream)?, DataFormat::Binary)
        .with_layout(GridLayout::Columns(BITSTREAM_COLUMNS))
        .with_column_groups(8))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn data_content(content: &Content) -> DataNodeContent {
    hex_row(content.data.to_vec(), content.data.len())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn codewords_content(content: &Content) -> DataNodeContent {
    // A gap after the 19 data codewords sets the 7 error correction ones apart.
    hex_row(content.codewords.to_vec(), content.codewords.len()).with_column_groups(content.data.len())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn scores_content(content: &Content) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U16(content.scores.to_vec()), DataFormat::Decimal)
        .with_layout(GridLayout::Columns(SCORE_COLUMNS))
        .with_labelling_style(LabellingStyle::Text(MASK_LABELS))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn format_content(values: &[u8]) -> Result<DataNodeContent, String> {
    Ok(DataNodeContent::new(bits(values)?, DataFormat::Binary).with_layout(GridLayout::Columns(FORMAT_CELLS)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Graph {
    /// The node for `key`.
    pub(super) fn node(&self, key: NodeKey) -> Result<NodeId, String> {
        self.nodes
            .iter()
            .find(|&&(k, _)| k == key)
            .map(|&(_, id)| id)
            .ok_or_else(|| format!("the graph has no node {key:?}"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the node for `key` is, and how big.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the graph has no such node, or the library cannot say.
    pub(super) fn rect(&self, scene: &Scene, key: NodeKey) -> Result<Rect, String> {
        scene.node_rect(self.node(key)?).map_err(stringify)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the nodes and edges in `scene`, with their top left corner at `origin`.
    ///
    /// The rows go down: the input and its bytes, the bitstream, the data codewords with the division and the error
    /// correction beside them, all the codewords, and the scores with the format information. Each position is worked
    /// out from the size of what came before it, so nothing overlaps whatever a node's size comes to.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a library call fails.
    pub(super) fn build(
        scene: &Scene,
        content: &Content,
        origin: Point,
        on_input: impl Fn(&Scene, NodeId, &str) + 'static,
    ) -> Result<Self, String> {
        let mut nodes = Vec::new();
        let mut right = origin.x;

        let input = scene
            .add_text_input_node(origin, INPUT_WIDTH, INPUT_LABEL, &content.text, on_input)
            .map_err(stringify)?;
        nodes.push((NodeKey::Input, input));
        let input_rect = scene.node_rect(input).map_err(stringify)?;

        // Adds a data node whose top left corner is at `x`, `y`, and says what it is and how big.
        let mut add = |key: NodeKey,
                       name: &str,
                       node_content: DataNodeContent,
                       x: f64,
                       y: f64|
         -> Result<(NodeId, f64, f64), String> {
            let id = scene
                .add_named_data_node(Point::new(x, y), name, node_content)
                .map_err(stringify)?;
            nodes.push((key, id));
            let rect = scene.node_rect(id).map_err(stringify)?;
            right = right.max(rect.origin.x + rect.size.width);
            Ok((id, rect.size.width, rect.size.height))
        };

        // The text and its bytes share one horizontal midline, so the edge between them is a straight line. Whichever
        // of them is the shorter is moved down by half the difference.
        let bytes_x = origin.x + INPUT_WIDTH + GAP_X;
        let (bytes, _, bytes_height) = add(NodeKey::Bytes, BYTES_NAME, bytes_content(content), bytes_x, origin.y)?;
        let row_1_height = input_rect.size.height.max(bytes_height);
        scene
            .move_node(
                input,
                Point::new(origin.x, origin.y + (row_1_height - input_rect.size.height) / 2.0),
            )
            .map_err(stringify)?;
        scene
            .move_node(bytes, Point::new(bytes_x, origin.y + (row_1_height - bytes_height) / 2.0))
            .map_err(stringify)?;

        let row_2 = origin.y + row_1_height + GAP_Y;
        let (_, _, bitstream_height) =
            add(NodeKey::Bitstream, BITSTREAM_NAME, bitstream_content(content)?, origin.x, row_2)?;
        let row_3 = row_2 + bitstream_height + GAP_Y;
        let (_, data_width, data_height) =
            add(NodeKey::DataCodewords, DATA_NAME, data_content(content), origin.x, row_3)?;
        let division_x = origin.x + data_width + GAP_X;
        let (_, division_width, division_height) =
            add(NodeKey::Division, DIVISION_NAME, hex_row(vec![0, 0], 2), division_x, row_3)?;
        let (_, _, ec_height) = add(
            NodeKey::ErrorCorrection,
            ERROR_CORRECTION_NAME,
            hex_row(vec![0; 7], 7),
            division_x + division_width + GAP_X,
            row_3,
        )?;
        let row_4 = row_3 + data_height.max(division_height).max(ec_height) + GAP_Y;
        let (_, _, codewords_height) =
            add(NodeKey::Codewords, CODEWORDS_NAME, codewords_content(content), origin.x, row_4)?;
        let row_5 = row_4 + codewords_height + GAP_Y;
        let (_, scores_width, scores_height) =
            add(NodeKey::MaskScores, SCORES_NAME, scores_content(content), origin.x, row_5)?;
        let (_, _, format_height) = add(
            NodeKey::Format,
            FORMAT_NAME,
            format_content(&[0; FORMAT_CELLS])?,
            origin.x + scores_width + GAP_X,
            row_5,
        )?;
        let bottom = row_5 + scores_height.max(format_height);

        let mut graph = Self {
            nodes,
            edges: Vec::new(),
            right,
            bottom,
        };

        // Two connectors arrive on the north edge of the codewords. One comes from the data codewords, and lands over
        // the middle of the 19 data codewords. The other comes from the error correction, and lands over the middle of
        // the 7 error correction codewords, which are copied into the last 7 places. Where those are is read from the
        // page.
        let document = document()?;
        let total = content.codewords.len();
        let first_error_correction = content.data.len();
        let landing = |first: usize, last: usize| {
            cell_span_fraction(&document, CODEWORDS_NAME, first, last, total)
                .unwrap_or_else(|| (first + last + 1) as f64 / 2.0 / total as f64)
        };
        let data_landing = landing(0, first_error_correction - 1);
        let error_correction_landing = landing(first_error_correction, total - 1);

        for (&(from, to), &(from_side, to_side)) in EDGES.iter().zip(&SIDES) {
            let mut options = ConnectorOptions::default()
                .with_from_side(Some(from_side))
                .with_to_side(Some(to_side));
            if to == NodeKey::Codewords {
                let landing = if from == NodeKey::DataCodewords {
                    data_landing
                } else {
                    error_correction_landing
                };
                options = options
                    .with_to_position(Some(landing))
                    .with_connector_type(ConnectorType::Elbow { corner_radius: 8.0 });
            }
            let edge = scene
                .add_edge_with(graph.node(from)?, graph.node(to)?, options)
                .map_err(stringify)?;
            graph.edges.push(((from, to), edge));
        }
        Ok(graph)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes what the nodes hold for the whole of one input: the bytes, the bitstream, the data codewords, all the
    /// codewords, the scores and the text. The text is left alone if it is already what the text node holds, so that
    /// typing is not interrupted.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a library call fails. Writing it again then repairs whatever was not written.
    pub(super) fn write_content(&self, scene: &Scene, content: &Content) -> Result<(), String> {
        let input = self.node(NodeKey::Input)?;
        if scene.text_input_value(input).map_err(stringify)? != content.text {
            scene.set_text_input_value(input, &content.text).map_err(stringify)?;
        }
        let set = |key, values: NodeValues| -> Result<(), String> {
            scene.set_data_values(self.node(key)?, values).map_err(stringify)
        };
        set(NodeKey::Bytes, NodeValues::U8(content.bytes.to_vec()))?;
        set(NodeKey::Bitstream, bits(&content.bitstream)?)?;
        set(NodeKey::DataCodewords, NodeValues::U8(content.data.to_vec()))?;
        set(NodeKey::Codewords, NodeValues::U8(content.codewords.to_vec()))?;
        set(NodeKey::MaskScores, NodeValues::U16(content.scores.to_vec()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows `snapshot`: what the division, the remainder and the format information hold, which cells are dimmed,
    /// which nodes and edges are active, and which codeword and which row of the scores are marked.
    ///
    /// Every call writes the whole of the snapshot, and the library skips what is already so. So it does not depend on
    /// any call before it, and a call that failed partway is put right by the next.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a library call fails.
    pub(super) fn show(&self, scene: &Scene, snapshot: &DemoSnapshot) -> Result<(), String> {
        let values = |key, values: NodeValues| -> Result<(), String> {
            scene.set_data_values(self.node(key)?, values).map_err(stringify)
        };
        values(NodeKey::Division, NodeValues::U8(snapshot.division.to_vec()))?;
        values(NodeKey::ErrorCorrection, NodeValues::U8(snapshot.register.to_vec()))?;
        values(NodeKey::Format, bits(&snapshot.format)?)?;

        let unreached = &snapshot.unreached;
        for (key, cells) in [
            (NodeKey::Bytes, &unreached.bytes),
            (NodeKey::Bitstream, &unreached.bitstream),
            (NodeKey::DataCodewords, &unreached.data),
            (NodeKey::Division, &unreached.division),
            (NodeKey::ErrorCorrection, &unreached.error_correction),
            (NodeKey::Codewords, &unreached.codewords),
            (NodeKey::MaskScores, &unreached.scores),
            (NodeKey::Format, &unreached.format),
        ] {
            scene.set_unreached_cells(self.node(key)?, cells).map_err(stringify)?;
        }

        let dividing: Vec<usize> = snapshot.dividing.into_iter().collect();
        scene
            .set_secondary_selection(self.node(NodeKey::DataCodewords)?, &dividing)
            .map_err(stringify)?;
        scene
            .set_secondary_selection(self.node(NodeKey::Bitstream)?, &snapshot.marked_bits)
            .map_err(stringify)?;

        let marked: Vec<usize> = snapshot.placing.into_iter().collect();
        scene
            .set_secondary_selection(self.node(NodeKey::Codewords)?, &marked)
            .map_err(stringify)?;
        let winner: Vec<usize> = snapshot
            .winner
            .into_iter()
            .flat_map(|mask| mask * SCORE_COLUMNS..(mask + 1) * SCORE_COLUMNS)
            .collect();
        debug_assert!(winner.iter().all(|&cell| cell < SCORE_CELLS));
        scene
            .set_secondary_selection(self.node(NodeKey::MaskScores)?, &winner)
            .map_err(stringify)?;

        for &(key, id) in &self.nodes {
            scene.set_focus(id, snapshot.active_nodes.contains(&key)).map_err(stringify)?;
        }
        for &(key, id) in &self.edges {
            scene
                .set_edge_focus(id, snapshot.active_edges.contains(&key))
                .map_err(stringify)?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many cells the bitstream has, which the snapshots and this agree on.
const _: () = assert!(BITSTREAM_CELLS % 8 == 0);
