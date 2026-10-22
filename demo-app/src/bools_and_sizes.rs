//! The Boolean and pointer-sized integer panels:
//!
//! - `panel-bool` / `#bools-diagram`, `#bools-truefalse-diagram`, `#bools-bit-diagram` and `#bools-icon-diagram` —
//!   [`build_bools_demo`]: Booleans written every way a `BoolFormat` can write them, and a truth table of logical
//!   operator nodes for each of the three.
//! - `panel-size` / `#sizes-diagram` — [`build_sizes_demo`]: the pointer-sized `usize` and `isize`.

use demo_common::util::{Item, build_lines_scene, ensure_svg_in, item, resize_svg, stringify};
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
    // Same reasoning as `tree::SCENE`'s own doc comment, for each demo's own, separate `Scene`s.
    static BOOLS_SCENES: RefCell<Vec<Scene>> = const { RefCell::new(Vec::new()) };
    static SIZES_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// The space left of the first node and above the first line of a truth table.
const MARGIN: f64 = 20.0;
/// Room east of the widest column for the zoom toolbar.
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

/// Every pair of operands of a two-input logical operator, in the order that a truth table lists them.
const OPERANDS: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

/// The three ways of writing a Boolean, with the id of the canvas that shows a truth table in each.
const TRUTH_TABLES: [(&str, BoolFormat); 3] = [
    ("bools-truefalse-diagram", BoolFormat::TrueFalse),
    ("bools-bit-diagram", BoolFormat::Bit),
    ("bools-icon-diagram", BoolFormat::Icon),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A column of `values` in `format`, each labelled with the matching one of `labels`.
fn column(values: NodeValues, format: DataFormat, labels: &'static [&'static str]) -> DataNodeContent {
    DataNodeContent::new(values, format)
        .with_layout(GridLayout::Columns(1))
        .with_labelling_style(LabellingStyle::Text(labels))
}

/// One Boolean in `format`.
fn flag(value: bool, format: DataFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(vec![value]), format)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Boolean demo: a Boolean in every format, and then a truth table for each of the three `BoolFormat`s.
///
/// - **Booleans in every format.**
///
///   One line for each of the words, the bits, and an icon (a green check mark or a red cross). A line holds `true` and
///   `false` on their own and then both in an array. A fourth line shows hexadecimal, which is the byte that a Boolean
///   is held in. Every format gives `true` and `false` a cell of the same width, so a node that changes from one to the
///   other does not outgrow its cells. A screen reader is told `true` or `false` for a mark, and not the name of its
///   glyph.
/// - **Three truth tables.**
///
///   Each has a column for each of `AND`, `OR` and `XOR`, and a row for each pair of operands, and each writes its
///   operands and its results in one `BoolFormat`: first the words, then the bits, then the marks. The logical
///   operators accept Booleans in any format, and the result of each is worked out here in plain Rust, as it is for
///   every operator node. The shifts, the rotations and the arithmetic operators do not accept a Boolean, since there
///   is no sum of two of them and no `true` to rotate.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#bools-diagram-stage` or the stage of any
/// truth table.
pub(crate) fn build_bools_demo() -> Result<(), String> {
    let formats: [(&str, DataFormat); 4] = [
        ("as text", DataFormat::Bool(BoolFormat::TrueFalse)),
        ("as bits", DataFormat::Bool(BoolFormat::Bit)),
        ("as icons", DataFormat::Bool(BoolFormat::Icon)),
        ("as a byte", DataFormat::Hexadecimal),
    ];
    let lines: Vec<Vec<Item>> = formats
        .into_iter()
        .map(|(name, format)| {
            vec![
                item(format!("true, {name}"), flag(true, format)),
                item(format!("false, {name}"), flag(false, format)),
                item(
                    format!("An array, {name}"),
                    column(NodeValues::Bool(vec![true, false]), format, BOOL_LABELS),
                ),
            ]
        })
        .collect();
    let mut scenes = vec![build_lines_scene("bools-diagram", lines, false)?];

    for (svg_id, format) in TRUTH_TABLES {
        scenes.push(build_truth_table(svg_id, DataFormat::Bool(format))?);
    }

    // Keeps every Scene's only strong handle alive for the page's lifetime — see BOOLS_SCENES's own doc comment.
    BOOLS_SCENES.with_borrow_mut(|slot| *slot = scenes);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a truth table into `#{svg_id}` inside `#{svg_id}-stage`: one column for each of `AND`, `OR` and `XOR`, and in
/// each a row for every pair of operands. Both operands, and the result, are written in `format`. Returns the scene.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing the stage, or if any library call fails.
fn build_truth_table(svg_id: &str, format: DataFormat) -> Result<Scene, String> {
    let document = demo_common::util::document()?;
    ensure_svg_in(&document, &format!("{svg_id}-stage"), svg_id, Size::new(1000.0, 800.0))?;
    let svg = SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let operators: [(BinaryOperator, Logic); 3] = [
        (BinaryOperator::And, |a, b| a & b),
        (BinaryOperator::Or, |a, b| a | b),
        (BinaryOperator::Xor, |a, b| a ^ b),
    ];
    let mut bottom = MARGIN;
    for (column, (operator, apply)) in operators.into_iter().enumerate() {
        let x = MARGIN + COLUMN_PITCH * column as f64;
        for (row, (a, b)) in OPERANDS.into_iter().enumerate() {
            let top = MARGIN + ROW_PITCH * row as f64;
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
    let right = MARGIN + COLUMN_PITCH * operators.len() as f64;

    resize_svg(&document, svg_id, Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN))?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok(scene)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the pointer-sized integer demo: `usize` and `isize`, each as a single value and as a column of several, in
/// decimal, hexadecimal and binary.
///
/// Their width is that of the target the demo runs on. That is 32 bits on `wasm32`, so the largest `usize` has ten
/// digits and its hexadecimal is four bytes, where on a 64-bit host it would have twenty digits and eight bytes. The
/// most negative `isize` is the widest in its node, since its sign makes it a character longer than the largest.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#sizes-diagram-stage`.
pub(crate) fn build_sizes_demo() -> Result<(), String> {
    let formats: [(&str, DataFormat); 3] = [
        ("decimal", DataFormat::Decimal),
        ("hexadecimal", DataFormat::Hexadecimal),
        ("binary", DataFormat::Binary),
    ];
    let usize_one = || NodeValues::Usize(vec![255]);
    let usize_many = || NodeValues::Usize(vec![0, 1, usize::MAX]);
    let isize_one = || NodeValues::Isize(vec![-2]);
    let isize_many = || NodeValues::Isize(vec![isize::MIN, -1, 0, isize::MAX]);

    let line = |name: &'static str, one: bool| -> Vec<Item> {
        formats
            .into_iter()
            .map(|(format_name, format)| {
                let content = match (name, one) {
                    ("usize", true) => DataNodeContent::new(usize_one(), format),
                    ("usize", false) => column(usize_many(), format, USIZE_LABELS),
                    (_, true) => DataNodeContent::new(isize_one(), format),
                    (_, false) => column(isize_many(), format, ISIZE_LABELS),
                };
                item(
                    format!("{name}, {}, {format_name}", if one { "one value" } else { "several" }),
                    content,
                )
            })
            .collect()
    };
    let lines = vec![
        line("usize", true),
        line("usize", false),
        line("isize", true),
        line("isize", false),
    ];

    let scene = build_lines_scene("sizes-diagram", lines, false)?;
    // Keeps this Scene's only strong handle alive for the page's lifetime — see SIZES_SCENE's own doc comment.
    SIZES_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
