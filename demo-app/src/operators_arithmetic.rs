//! `panel-operators-arithmetic` / `#operators-arithmetic-diagram`: two operand nodes feeding an operator node, for
//! every [`ArithmeticOperator`] this crate names, in three columns of unsigned integers, signed integers and floating
//! point values. Every result shown is computed here with plain Rust arithmetic, never by the library itself — see
//! [`build_arithmetic_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{ArithmeticOperator, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene, Side, ToolbarOptions},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators_arithmetic.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One row of the demo: an operator, its two operands, and the result the caller computed for them. All three hold the
/// same [`NodeValues`] type, which is what [`Scene::add_arithmetic_operator_node`] requires.
struct Row {
    operator: ArithmeticOperator,
    a: NodeValues,
    b: NodeValues,
    result: NodeValues,
}

/// The distance between the left-hand edge of one column and the next. It is wide enough for the longest operator node
/// of any column, which is the one that shows `0.30000000000000004`.
const COLUMN_PITCH: f64 = 380.0;
/// The distance between one row and the next. A row is two operands stacked, and an operator node beside them.
const ROW_PITCH: f64 = 170.0;
/// Every row's operand(s) share their column's left-hand x; every row's operator node is a fixed distance to its right.
const X_MARGIN: f64 = 20.0;
const OPERATOR_OFFSET: f64 = 240.0;
const Y_MARGIN: f64 = 20.0;
/// The vertical distance between the two operands of a row, and from the first operand down to the operator node.
const OPERAND_PITCH: f64 = 90.0;
/// The room east of the last column for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every row of the demo, one column for each kind of value and one row for each [`ArithmeticOperator`], each result
/// computed here in plain Rust.
///
/// Each operand width appears at least once. Every operand is a `let` binding, not a literal inside the sum, because
/// the compiler would otherwise fold the arithmetic itself and the example would stop showing what a caller does.
fn columns() -> [Vec<Row>; 3] {
    // Unsigned. `Subtract` never underflows, since the first operand is always `>=` the second, and neither `Divide`
    // nor `Modulus` divides by zero. Both are the caller's own responsibility, never checked by the library.
    let (add_a, add_b): (u8, u8) = (180, 45);
    let (sub_a, sub_b): (u16, u16) = (50_000, 12_345);
    let (mul_a, mul_b): (u32, u32) = (50_000, 70_000);
    let (div_a, div_b): (u64, u64) = (100_000_000_000, 4);
    let (mod_a, mod_b): (u8, u8) = (200, 7);
    let unsigned = vec![
        row(
            ArithmeticOperator::Add,
            NodeValues::U8(vec![add_a]),
            NodeValues::U8(vec![add_b]),
            NodeValues::U8(vec![add_a + add_b]),
        ),
        row(
            ArithmeticOperator::Subtract,
            NodeValues::U16(vec![sub_a]),
            NodeValues::U16(vec![sub_b]),
            NodeValues::U16(vec![sub_a - sub_b]),
        ),
        row(
            ArithmeticOperator::Multiply,
            NodeValues::U32(vec![mul_a]),
            NodeValues::U32(vec![mul_b]),
            NodeValues::U32(vec![mul_a * mul_b]),
        ),
        row(
            ArithmeticOperator::Divide,
            NodeValues::U64(vec![div_a]),
            NodeValues::U64(vec![div_b]),
            NodeValues::U64(vec![div_a / div_b]),
        ),
        row(
            ArithmeticOperator::Modulus,
            NodeValues::U8(vec![mod_a]),
            NodeValues::U8(vec![mod_b]),
            NodeValues::U8(vec![mod_a % mod_b]),
        ),
    ];

    // Signed. A negative value shows its sign and a positive one does not. Subtraction can now go below zero, a
    // division truncates towards zero, and a remainder takes the sign of the dividend.
    let (add_a, add_b): (i16, i16) = (-1_200, 450);
    let (sub_a, sub_b): (i32, i32) = (1_000, 25_000);
    let (mul_a, mul_b): (i8, i8) = (-12, 9);
    let (div_a, div_b): (i64, i64) = (-100_000_000_001, 4);
    let (mod_a, mod_b): (i32, i32) = (-17, 5);
    let signed = vec![
        row(
            ArithmeticOperator::Add,
            NodeValues::I16(vec![add_a]),
            NodeValues::I16(vec![add_b]),
            NodeValues::I16(vec![add_a + add_b]),
        ),
        row(
            ArithmeticOperator::Subtract,
            NodeValues::I32(vec![sub_a]),
            NodeValues::I32(vec![sub_b]),
            NodeValues::I32(vec![sub_a - sub_b]),
        ),
        row(
            ArithmeticOperator::Multiply,
            NodeValues::I8(vec![mul_a]),
            NodeValues::I8(vec![mul_b]),
            NodeValues::I8(vec![mul_a * mul_b]),
        ),
        row(
            ArithmeticOperator::Divide,
            NodeValues::I64(vec![div_a]),
            NodeValues::I64(vec![div_b]),
            NodeValues::I64(vec![div_a / div_b]),
        ),
        row(
            ArithmeticOperator::Modulus,
            NodeValues::I32(vec![mod_a]),
            NodeValues::I32(vec![mod_b]),
            NodeValues::I32(vec![mod_a % mod_b]),
        ),
    ];

    // Floating point. The result is whatever the float arithmetic really produced, so it shows the rounding of each
    // width: `0.1 + 0.2` is not `0.3` as an `f64`, and `1.1 * 3` is not `3.3` as an `f32`. A float remainder takes the
    // sign of the dividend, as an integer one does.
    let (add_a, add_b): (f64, f64) = (0.1, 0.2);
    let (sub_a, sub_b): (f32, f32) = (1.5, 4.25);
    let (mul_a, mul_b): (f32, f32) = (1.1, 3.0);
    let (div_a, div_b): (f64, f64) = (1.0, 3.0);
    let (mod_a, mod_b): (f64, f64) = (-7.5, 2.0);
    let floats = vec![
        row(
            ArithmeticOperator::Add,
            NodeValues::F64(vec![add_a]),
            NodeValues::F64(vec![add_b]),
            NodeValues::F64(vec![add_a + add_b]),
        ),
        row(
            ArithmeticOperator::Subtract,
            NodeValues::F32(vec![sub_a]),
            NodeValues::F32(vec![sub_b]),
            NodeValues::F32(vec![sub_a - sub_b]),
        ),
        row(
            ArithmeticOperator::Multiply,
            NodeValues::F32(vec![mul_a]),
            NodeValues::F32(vec![mul_b]),
            NodeValues::F32(vec![mul_a * mul_b]),
        ),
        row(
            ArithmeticOperator::Divide,
            NodeValues::F64(vec![div_a]),
            NodeValues::F64(vec![div_b]),
            NodeValues::F64(vec![div_a / div_b]),
        ),
        row(
            ArithmeticOperator::Modulus,
            NodeValues::F64(vec![mod_a]),
            NodeValues::F64(vec![mod_b]),
            NodeValues::F64(vec![mod_a % mod_b]),
        ),
    ];

    [unsigned, signed, floats]
}

fn row(operator: ArithmeticOperator, a: NodeValues, b: NodeValues, result: NodeValues) -> Row {
    Row { operator, a, b, result }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: two operand nodes feeding an operator node, for every [`ArithmeticOperator`] this crate
/// names, in three columns. The first holds unsigned integers (`u8`, `u16`, `u32`, `u64`), the second signed integers
/// (`i8`, `i16`, `i32`, `i64`) and the third floating point values (`f32`, `f64`). See [`columns`] for what each row
/// shows.
///
/// Each operand is a [`Scene::add_named_data_node`] node, labelled `"A"`/`"B"`, rather than a plain
/// [`Scene::add_data_node`] box. The raw value's own outer box wraps its value cell, the same way an operator node's
/// own outer box wraps its result. Naming `inputs.0`/`inputs.1` this way pairs naturally with the non-commutative
/// operators' own "L"/"R" port markers: `A` is always the left-hand operand, `B` the right-hand one.
///
/// Every result shown is computed in [`columns`], with plain Rust operators (`+`, `-`, `*`, `/`, `%`). `svg_dom_graph`
/// itself never evaluates an operator. See [`Scene::add_arithmetic_operator_node`]'s own doc comment for why. So this
/// function's job is exactly the one a real caller would have: compute the real value, then hand it to the library
/// alongside the operator that produced it. An operator node's own displayed value is always whatever the caller
/// already computed. See [`ArithmeticOperator`]'s own doc comment.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-arithmetic-diagram`.
pub(crate) fn build_arithmetic_operator_demo() -> Result<(), String> {
    let columns = columns();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    let size = Size::new(
        X_MARGIN + COLUMN_PITCH * columns.len() as f64 + TOOLBAR_ROOM,
        Y_MARGIN + ROW_PITCH * rows as f64,
    );
    crate::util::ensure_svg_in(
        &crate::util::document()?,
        "operators-arithmetic-diagram-stage",
        "operators-arithmetic-diagram",
        size,
    )?;
    let svg = SvgRoot::attach("operators-arithmetic-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    let place = |content: DataNodeContent, name: &str, x: f64, y: f64| -> Result<NodeId, String> {
        let node = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
        Ok(node)
    };

    for (column, column_rows) in columns.into_iter().enumerate() {
        let x = X_MARGIN + COLUMN_PITCH * column as f64;
        for (index, row) in column_rows.into_iter().enumerate() {
            let y = Y_MARGIN + ROW_PITCH * index as f64;
            let a = place(DataNodeContent::new(row.a, DataFormat::Decimal), "A", x, y)?;
            let b = place(DataNodeContent::new(row.b, DataFormat::Decimal), "B", x, y + OPERAND_PITCH)?;
            let operator = scene
                .add_arithmetic_operator_node(
                    Point::new(x + OPERATOR_OFFSET, y + OPERAND_PITCH / 2.0),
                    row.operator,
                    (a, b),
                    DataNodeContent::new(row.result, DataFormat::Decimal),
                )
                .map_err(stringify)?;
            scene.make_draggable_with(operator, drag_options).map_err(stringify)?;
        }
    }

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
