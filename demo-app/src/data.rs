//! `panel-data` / `#data-diagram`: draggable nodes whose content is a [`DataNodeContent`] grid of values rather
//! than a plain text label — one single-value and one multi-value node per integer width (`u8`/`u16`/`u32`/`u64`),
//! the multi-value counts chosen to cover every combination the grid layout rule can produce. See
//! [`build_data_demo`]'s own doc comment for exactly which.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, DragOptions, NodeValues, Scene};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("data.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the data-node demo: one row per integer width (`u8`, `u16`, `u32`, `u64`), each row holding a single-value
/// box on the left and a multi-value box on the right — every box draggable, and every box sized to fit its own content
/// automatically, unlike [`crate::tree::build_demo_tree`]'s caller-sized boxes.
///
/// The four multi-value counts are deliberately chosen to cover every combination
/// [`DataNodeContent::shape`](svg_dom_graph::scene::DataNodeContent::shape)'s layout rule can produce:
///
/// - `u8`, 8 values: divides evenly by a power of two (`2`) but is not itself a square — renders as 2 rows of 4.
/// - `u16`, 5 values: no power-of-two divisor at all — falls back to the closest-to-square shape, 3 rows of 2 (one slot
///   left blank).
/// - `u32`, 9 values: a perfect square with no power-of-two divisor — falls back to the same closest-to-square rule,
///   which for a perfect square is an exact fit: 3 rows of 3.
/// - `u64`, 4 values: both a perfect square and evenly divisible by a power of two (`2`) — the two rules agree, landing
///   on the same 2×2 shape either way.
///
/// A fifth, standalone row below those four shows a single `u64` value under [`DataFormat::Binary`] — 64 digits,
/// nybble-grouped and byte-separated. [`GridLayout::Automatic`](svg_dom_graph::scene::GridLayout::Automatic) sizes a
/// grid by cell *count*, not physical width. So this is also the demo's own worked example of the extreme
/// cell-aspect-ratio case. That case motivates [`GridLayout::MaxColumns`](svg_dom_graph::scene::GridLayout::MaxColumns)
/// — see that type's own doc comment.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#data-diagram`.
pub(crate) fn build_data_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("data-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    // Bounded to the diagram's own viewBox — see build_demo_tree's own comment for why.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every row's single-value box shares this left-hand x; every row's multi-value box shares this one, to its
    // right, regardless of how wide either box actually turns out to be.
    const X_SINGLE: f64 = 20.0;
    const X_MULTI: f64 = 260.0;

    let place = |x: f64, y: f64, content: DataNodeContent| -> Result<(), String> {
        let node = scene.add_data_node(Point::new(x, y), content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)
    };

    // u8: a single value, and 8 values — divides evenly by a power of two (2 rows of 4).
    place(
        X_SINGLE,
        20.0,
        DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        20.0,
        DataNodeContent::new(
            NodeValues::U8(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]),
            DataFormat::Binary,
        ),
    )?;

    // u16: a single value, and 5 values — no power-of-two divisor, so this falls back to the closest-to-square
    // shape (3 rows of 2, one slot blank). Decimal here, rather than hex/binary, to also show a format with no
    // byte-group splitting.
    place(
        X_SINGLE,
        140.0,
        DataNodeContent::new(NodeValues::U16(vec![0x1234]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        140.0,
        DataNodeContent::new(NodeValues::U16(vec![100, 250, 500, 1000, 65535]), DataFormat::Decimal),
    )?;

    // u32: a single value, and 9 values — a perfect square with no power-of-two divisor, landing on an exact 3x3
    // square via the same fallback rule.
    place(
        X_SINGLE,
        290.0,
        DataNodeContent::new(NodeValues::U32(vec![0xDEAD_BEEF]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        290.0,
        DataNodeContent::new(
            NodeValues::U32(vec![
                0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444, 0x5555_5555, 0x6666_6666, 0x7777_7777, 0x8888_8888,
                0x9999_9999,
            ]),
            DataFormat::Hexadecimal,
        ),
    )?;

    // u64: a single value, and 4 values — both a perfect square and evenly divisible by a power of two, so the
    // two layout rules agree on the same 2x2 shape.
    place(
        X_SINGLE,
        440.0,
        DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        440.0,
        DataNodeContent::new(
            NodeValues::U64(vec![
                0x0011223344556677,
                0x8899AABBCCDDEEFF,
                0x1234567890ABCDEF,
                0xFEDCBA0987654321,
            ]),
            DataFormat::Hexadecimal,
        ),
    )?;

    // A single u64 under Binary: 64 digits, nybble-grouped and byte-separated. This cell renders far wider than
    // it is tall — the extreme cell-aspect-ratio case GridLayout::Automatic's own doc comment describes. It is
    // also the one GridLayout::MaxColumns exists to let a caller cap. A standalone row of its own, not paired
    // with a multi-value box, since this single cell is already wide enough to need the room.
    place(
        X_SINGLE,
        590.0,
        DataNodeContent::new(NodeValues::U64(vec![0x0102030405060708]), DataFormat::Binary),
    )?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
