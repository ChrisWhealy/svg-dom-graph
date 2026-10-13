//! `panel-bool-size` / `#bools-sizes-diagram`: Booleans written every way a `BoolFormat` can write them, a truth table
//! of logical operator nodes, and the pointer-sized `usize` and `isize`. See [`build_bools_and_sizes_demo`]'s own doc
//! comment for what each node shows.

use crate::util::{ensure_svg_in, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::scene::{
    BinaryOperator, BoolFormat, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene, Side,
    ToolbarOptions,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("bools_and_sizes.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// The space left of the first node and above the first line, and between one node and the next, and one line and the
/// next.
const MARGIN: f64 = 20.0;
const H_GAP: f64 = 30.0;
const V_GAP: f64 = 36.0;
/// Room east of the widest line for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

/// The distance between the left-hand edge of one truth table column and the next, and between one row and the next.
const COLUMN_PITCH: f64 = 300.0;
const ROW_PITCH: f64 = 170.0;
/// How far right of its operands an operator node sits, and how far the second operand sits below the first.
const OPERATOR_OFFSET: f64 = 160.0;
const OPERAND_PITCH: f64 = 90.0;

/// What each Boolean is, written beside its row.
const BOOL_LABELS: &[&str] = &["true", "false"];

/// What each pointer-sized value is, written beside its row.
const USIZE_LABELS: &[&str] = &["zero", "one", "max"];
const ISIZE_LABELS: &[&str] = &["min", "-1", "zero", "max"];

/// What a two-input logical operator does to its operands, worked out here in plain Rust.
type Logic = fn(bool, bool) -> bool;

/// How each row of the truth table writes its operands and its result: as marks, as bits, as the byte that a Boolean is
/// held in, and as the words. Every row has a format of its own, so that one table shows each way of writing a Boolean.
const ROW_FORMATS: [DataFormat; 4] = [
    DataFormat::Bool(BoolFormat::Icon),
    DataFormat::Bool(BoolFormat::Bit),
    DataFormat::Hexadecimal,
    DataFormat::Bool(BoolFormat::TrueFalse),
];

/// Every pair of operands of a two-input logical operator, in the order that a truth table lists them.
const OPERANDS: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node of a line: a `name`, and its `content`.
struct Item {
    name: &'static str,
    content: DataNodeContent,
}

fn item(name: &'static str, content: DataNodeContent) -> Item {
    Item { name, content }
}

/// A column of `values` in `format`, each labelled with the matching one of `labels`.
fn column(values: NodeValues, format: DataFormat, labels: &'static [&'static str]) -> DataNodeContent {
    DataNodeContent::new(values, format)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(labels))
}

/// A column of `true` and `false` in `format`.
fn flags(format: DataFormat) -> DataNodeContent {
    column(NodeValues::Bool(vec![true, false]), format, BOOL_LABELS)
}

/// One Boolean in `format`.
fn flag(value: bool, format: DataFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(vec![value]), format)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo, one line of nodes at a time and then a truth table. Each line is read across.
///
/// - **Line 1: a Boolean in every format.** The words, the bits, and a green check mark and a red cross. Hexadecimal is
///   the byte that a Boolean is held in. Every format gives `true` and `false` a cell of the same width, so a node that
///   changes from one to the other does not outgrow its cells. A screen reader is told `true` or `false` for a mark,
///   and not the name of its glyph.
/// - **Line 2: `usize` and `isize`.** Their width is that of the target the demo runs on. That is 32 bits on `wasm32`,
///   so the largest `usize` has ten digits and its hexadecimal is four bytes, where on a 64-bit host it would have
///   twenty digits and eight bytes. The most negative `isize` is the widest in its node, since its sign makes it a
///   character longer than the largest.
/// - **The truth table.** One column for each of `AND`, `OR` and `XOR`, and a row for each pair of operands. The
///   operands and the result of a row are all written one way: the first row as marks, the second as bits, the third as
///   the byte that a Boolean is held in, and the fourth as the words. The logical operators accept Booleans in any
///   format, and the result of each is worked out here in plain Rust, as it is for every operator node. The shifts, the
///   rotations and the arithmetic operators do not accept a Boolean, since there is no sum of two of them and no `true`
///   to rotate.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#bools-sizes-diagram`.
pub(crate) fn build_bools_and_sizes_demo() -> Result<(), String> {
    let document = crate::util::document()?;
    ensure_svg_in(
        &document,
        "bools-sizes-diagram-stage",
        "bools-sizes-diagram",
        Size::new(1000.0, 1200.0),
    )?;
    let svg = SvgRoot::attach("bools-sizes-diagram").map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let lines = [
        vec![
            item("true and false", flags(DataFormat::Bool(BoolFormat::TrueFalse))),
            item("As bits", flags(DataFormat::Bool(BoolFormat::Bit))),
            item("As icons", flags(DataFormat::Bool(BoolFormat::Icon))),
            item("As a byte", flags(DataFormat::Hexadecimal)),
        ],
        vec![
            item(
                "usize, decimal",
                column(NodeValues::Usize(vec![0, 1, usize::MAX]), DataFormat::Decimal, USIZE_LABELS),
            ),
            item(
                "usize, hexadecimal",
                column(NodeValues::Usize(vec![0, 1, usize::MAX]), DataFormat::Hexadecimal, USIZE_LABELS),
            ),
            item(
                "isize, decimal",
                column(
                    NodeValues::Isize(vec![isize::MIN, -1, 0, isize::MAX]),
                    DataFormat::Decimal,
                    ISIZE_LABELS,
                ),
            ),
            item(
                "isize, hexadecimal",
                column(
                    NodeValues::Isize(vec![isize::MIN, -1, 0, isize::MAX]),
                    DataFormat::Hexadecimal,
                    ISIZE_LABELS,
                ),
            ),
        ],
    ];

    let mut y = MARGIN;
    let mut right: f64 = 0.0;
    for line in lines {
        let mut x = MARGIN;
        let mut tallest: f64 = 0.0;
        for Item { name, content } in line {
            let node = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
            let rect: Rect = scene.node_rect(node).map_err(stringify)?;
            x += rect.size.width + H_GAP;
            tallest = tallest.max(rect.size.height);
        }
        right = right.max(x - H_GAP);
        y += tallest + V_GAP;
    }

    let truth_table: [(BinaryOperator, Logic); 3] = [
        (BinaryOperator::And, |a, b| a & b),
        (BinaryOperator::Or, |a, b| a | b),
        (BinaryOperator::Xor, |a, b| a ^ b),
    ];
    let mut bottom = y;
    for (column, (operator, apply)) in truth_table.into_iter().enumerate() {
        let x = MARGIN + COLUMN_PITCH * column as f64;
        for (row, ((a, b), format)) in OPERANDS.into_iter().zip(ROW_FORMATS).enumerate() {
            let top = y + ROW_PITCH * row as f64;
            let first = scene
                .add_named_data_node(Point::new(x, top), "A", flag(a, format))
                .map_err(stringify)?;
            let second = scene
                .add_named_data_node(Point::new(x, top + OPERAND_PITCH), "B", flag(b, format))
                .map_err(stringify)?;
            let rect: Rect = scene.node_rect(second).map_err(stringify)?;
            bottom = bottom.max(rect.origin.y + rect.size.height);
            scene
                .add_binary_operator_node(
                    Point::new(x + OPERATOR_OFFSET, top + OPERAND_PITCH / 2.0),
                    operator,
                    (first, second),
                    flag(apply(a, b), format),
                )
                .map_err(stringify)?;
        }
    }
    let table_right = MARGIN + COLUMN_PITCH * truth_table.len() as f64;
    right = right.max(table_right);

    resize_svg(
        &document,
        "bools-sizes-diagram",
        Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN),
    )?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
