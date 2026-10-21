//! Laying the boxes of one column out on an axis.

use super::WORD_COLUMNS;
use crate::util::stringify;
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{BinaryOperator, DataFormat, DataNodeContent, GridLayout, HexDumpFormat, NodeValues, Scene},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One box of a column, with the note that explains it, and perhaps another box that only helps it.
pub(super) struct Spec {
    pub(super) name: &'static str,
    pub(super) content: DataNodeContent,
    pub(super) explanation: String,
    /// A box that feeds this one, beside it and level with it, and its name.
    pub(super) aside: Option<(&'static str, DataNodeContent)>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the nodes of one column go: the boxes on a single vertical axis, so that each is centred over the one it
/// connects to, and the explanations in a column of their own.
pub(super) struct Column<'a> {
    pub(super) scene: &'a Scene,
    /// The x of the axis the boxes are centred on.
    pub(super) axis: f64,
    /// The x of the left edge of the explanations, and how wide they are.
    pub(super) explain_left: f64,
    pub(super) explanation_width: f64,
    /// The x of the middle of the boxes that only help.
    pub(super) side: f64,
    pub(super) nodes: Vec<NodeId>,
}

impl Column<'_> {
    /// How tall the row for `spec` is: the tallest of its box, its explanation and its aside.
    pub(super) fn height(&self, spec: &Spec) -> Result<f64, String> {
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds the row for `spec` with its top at `top` and `height` tall, everything in it centred on the middle. Returns
    /// the ids of the box and of the aside.
    pub(super) fn place(&mut self, top: f64, height: f64, spec: Spec) -> Result<(NodeId, Option<NodeId>), String> {
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds `explanation` in the column of explanations, centred on `middle`.
    pub(super) fn explain(&mut self, middle: f64, explanation: &str) -> Result<(), String> {
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How tall an XOR operator node with `result` and its `explanation` is.
    pub(super) fn operator_height(&self, result: &DataNodeContent, explanation: &str) -> Result<f64, String> {
        let size = self.scene.measure_operator_box("XOR", result).map_err(stringify)?;
        let words = self
            .scene
            .measure_markdown_node(self.explanation_width, explanation)
            .map_err(stringify)?;
        Ok(size.height.max(words.height))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an XOR operator node of `inputs`, with its top at `top` and `height` tall, and its explanation.
    pub(super) fn place_operator(
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single 32-bit value written in hexadecimal.
pub(super) fn word(value: u32) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(vec![value]), DataFormat::Hexadecimal)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Several 32-bit values written in hexadecimal, `WORD_COLUMNS` to a row.
pub(super) fn array_of_words(values: Vec<u32>) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U32(values), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(WORD_COLUMNS))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bytes` as a hex dump with offsets.
pub(super) fn hexdump(bytes: &[u8]) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::U8(bytes.to_vec()),
        DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `text` as a single run of text in a box. Only printable ASCII can be drawn this way.
pub(super) fn plain(text: &str) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(text.as_bytes().to_vec()), DataFormat::PlainText)
}
