//! The four data-node panels that show whole numbers and bytes:
//!
//! - `panel-unsigned` / `#unsigned-diagram` — [`build_unsigned_demo`]: `u8`, `u16`, `u32` and `u64`.
//! - `panel-signed` / `#signed-diagram` — [`build_signed_demo`]: `i8`, `i16`, `i32` and `i64`.
//! - `panel-hexdump` / `#hexdump-diagram` — [`build_hexdump_demo`]: bytes as `hexdump -C` shows them.
//! - `panel-ascii` / `#ascii-diagram` — [`build_ascii_demo`]: bytes as characters.
//!
//! Each shows draggable nodes whose content is a [`DataNodeContent`] grid of values rather than a plain text label.

use crate::util::{Item, build_lines_scene, item};
use std::cell::RefCell;
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, HexDumpFormat, NodeValues, Scene};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("data.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for each demo's own, separate `Scene`.
    static UNSIGNED_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    static SIGNED_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    static HEXDUMP_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    static ASCII_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// The three ways a whole number can be written: as decimal digits, as the hexadecimal digits of each byte, and as the
/// binary digits of each byte. Each is named in the node that shows it.
const FORMATS: [(&str, DataFormat); 3] = [
    ("decimal", DataFormat::Decimal),
    ("hexadecimal", DataFormat::Hexadecimal),
    ("binary", DataFormat::Binary),
];

/// One node for each of [`FORMATS`], holding `values` and named `{what}, {format}`.
fn in_every_format(what: &str, values: &NodeValues) -> Vec<Item> {
    FORMATS
        .iter()
        .map(|&(name, format)| item(format!("{what}, {name}"), DataNodeContent::new(values.clone(), format)))
        .collect()
}

/// A line for each of a single value and several, with each line's nodes one beside the next, or if `wide`, the binary
/// one apart on a line of its own. A binary node has eight digits for every byte, so one of a 32-bit or 64-bit value is
/// wider than a decimal or hexadecimal one beside it by a margin that would push the line off the canvas.
fn lines_for(type_name: &str, single: NodeValues, several: NodeValues, wide: bool) -> Vec<Vec<Item>> {
    let mut lines = Vec::new();
    for (what, values) in [
        (format!("{type_name}, one value"), single),
        (format!("{type_name}, several"), several),
    ] {
        let mut nodes = in_every_format(&what, &values);
        if wide {
            let binary = nodes.split_off(2);
            lines.push(nodes);
            lines.push(binary);
        } else {
            lines.push(nodes);
        }
    }
    lines
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the unsigned integer demo: for each of `u8`, `u16`, `u32` and `u64`, a single value and several values, each
/// written in decimal, hexadecimal and binary. Every box is draggable, and every box is sized to fit its own content
/// automatically, unlike [`crate::tree::build_demo_tree`]'s caller-sized boxes.
///
/// The counts of several values are deliberately chosen to cover every combination
/// [`DataNodeContent::shape`](svg_dom_graph::scene::DataNodeContent::shape)'s layout rule can produce:
///
/// - `u8`, 8 values: divides evenly by a power of two (`2`) but is not itself a square — renders as 2 rows of 4.
/// - `u16`, 5 values: no power-of-two divisor at all — falls back to the closest-to-square shape, 3 rows of 2 (one slot
///   left blank).
/// - `u32`, 9 values: a perfect square with no power-of-two divisor. It falls back to the same closest-to-square rule,
///   which for a perfect square is an exact fit: 3 rows of 3.
/// - `u64`, 4 values: both a perfect square and evenly divisible by a power of two (`2`) — the two rules agree, landing
///   on the same 2×2 shape either way.
///
/// A single `u64` under [`DataFormat::Binary`] is 64 digits, nybble-grouped and byte-separated.
/// [`GridLayout::Automatic`](svg_dom_graph::scene::GridLayout::Automatic) sizes a grid by cell *count*, not physical
/// width. So that node is also this demo's own worked example of the extreme cell-aspect-ratio case. That case
/// motivates [`GridLayout::MaxColumns`](svg_dom_graph::scene::GridLayout::MaxColumns) — see that type's own doc
/// comment.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#unsigned-diagram-stage`.
pub(crate) fn build_unsigned_demo() -> Result<(), String> {
    let mut lines = Vec::new();
    lines.extend(lines_for(
        "u8",
        NodeValues::U8(vec![0xAB]),
        NodeValues::U8(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]),
        false,
    ));
    lines.extend(lines_for(
        "u16",
        NodeValues::U16(vec![0x1234]),
        NodeValues::U16(vec![100, 250, 500, 1000, 65535]),
        false,
    ));
    lines.extend(lines_for(
        "u32",
        NodeValues::U32(vec![0xDEAD_BEEF]),
        NodeValues::U32(vec![
            0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444, 0x5555_5555, 0x6666_6666, 0x7777_7777, 0x8888_8888,
            0x9999_9999,
        ]),
        true,
    ));
    lines.extend(lines_for(
        "u64",
        NodeValues::U64(vec![0xF0E1_D2C3_B4A5_9687]),
        NodeValues::U64(vec![
            0x0011_2233_4455_6677,
            0x8899_AABB_CCDD_EEFF,
            0x1234_5678_90AB_CDEF,
            0xFEDC_BA09_8765_4321,
        ]),
        true,
    ));

    let scene = build_lines_scene("unsigned-diagram", lines, true)?;
    // Keeps this Scene's only strong handle alive for the page's lifetime — see UNSIGNED_SCENE's own doc comment.
    UNSIGNED_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the signed integer demo: for each of `i8`, `i16`, `i32` and `i64`, a single value and several values, each
/// written in decimal, hexadecimal and binary.
///
/// A sign is shown only for a value that is negative. A positive value, and zero, show none. Hexadecimal and binary
/// show the value's two's-complement bytes, so an `i8` of `-2` reads `FE` in hexadecimal, and the most negative value
/// of each width reads as a one followed by zeros. Every group of several values holds a negative, a zero and a
/// positive value, and the extremes of its width.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#signed-diagram-stage`.
pub(crate) fn build_signed_demo() -> Result<(), String> {
    let mut lines = Vec::new();
    lines.extend(lines_for(
        "i8",
        NodeValues::I8(vec![-2]),
        NodeValues::I8(vec![i8::MIN, -42, -1, 0, 1, 42, 100, i8::MAX]),
        false,
    ));
    lines.extend(lines_for(
        "i16",
        NodeValues::I16(vec![-2]),
        NodeValues::I16(vec![i16::MIN, -1, 0, 1000, i16::MAX]),
        false,
    ));
    lines.extend(lines_for(
        "i32",
        NodeValues::I32(vec![-1_234_567]),
        NodeValues::I32(vec![i32::MIN, -100_000, -5, -1, 0, 5, 42, 100_000, i32::MAX]),
        true,
    ));
    lines.extend(lines_for(
        "i64",
        NodeValues::I64(vec![-2]),
        NodeValues::I64(vec![i64::MIN, -1, 0, i64::MAX]),
        true,
    ));

    let scene = build_lines_scene("signed-diagram", lines, true)?;
    // Keeps this Scene's only strong handle alive for the page's lifetime — see SIGNED_SCENE's own doc comment.
    SIGNED_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the hexdump demo: the same bytes as a hex dump in the format of `hexdump -C`, written the four ways that
/// [`HexDumpFormat`] allows, and then a single byte and a short run of bytes. Each line of sixteen bytes is a cell of
/// its own, and each node is a line of its own, since a dump line is wider than half the canvas.
///
/// - **Neither:** each line's offset is hidden, as it is by default, and the line that ends `hexdump -C` with the total
///   length is not shown.
/// - **With offsets:** [`HexDumpFormat::with_offset`] shows each line's offset.
/// - **With the total length:** [`HexDumpFormat::with_total_length`] adds the final line, which holds the number of
///   bytes.
/// - **With both.**
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#hexdump-diagram-stage`.
pub(crate) fn build_hexdump_demo() -> Result<(), String> {
    const DUMPED: &[u8] = b"Hello, World!\n\x00\x01\x02\xFE\xFF Huffman codes";
    let dump = |name: &str, bytes: &[u8], format: HexDumpFormat| {
        item(
            name,
            DataNodeContent::new(NodeValues::U8(bytes.to_vec()), DataFormat::HexDump(format)),
        )
    };

    let lines = vec![
        vec![dump("No offset, no total length (the default)", DUMPED, HexDumpFormat::new())],
        vec![dump("With offsets", DUMPED, HexDumpFormat::new().with_offset(true))],
        vec![dump(
            "With the total length",
            DUMPED,
            HexDumpFormat::new().with_total_length(true),
        )],
        vec![dump(
            "With offsets and the total length",
            DUMPED,
            HexDumpFormat::new().with_offset(true).with_total_length(true),
        )],
        vec![dump(
            "A single byte",
            &[0x7F],
            HexDumpFormat::new().with_offset(true).with_total_length(true),
        )],
        vec![dump(
            "Fewer than sixteen bytes",
            b"Short line",
            HexDumpFormat::new().with_offset(true).with_total_length(true),
        )],
    ];

    let scene = build_lines_scene("hexdump-diagram", lines, true)?;
    // Keeps this Scene's only strong handle alive for the page's lifetime — see HEXDUMP_SCENE's own doc comment.
    HEXDUMP_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the ASCII demo, which shows bytes as characters in the two ways [`DataFormat`] offers.
///
/// - **[`DataFormat::Ascii`]** writes every byte as one character in a cell of its own: a printable byte as itself, a
///   space as `␣`, and anything else as `·`. It is shown for a single `u8`, for several, and for wider values, whose
///   bytes are shown in byte order with no separator.
/// - **[`DataFormat::PlainText`]** draws the whole run of bytes as one line of text in a single box with no grid. It
///   accepts only `u8` values that are printable ASCII, and a space stays a space.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#ascii-diagram-stage`.
pub(crate) fn build_ascii_demo() -> Result<(), String> {
    const PLAIN: &[u8] = b"The quick brown fox jumps over the lazy dog";

    let lines = vec![
        vec![
            item(
                "u8, one value",
                DataNodeContent::new(NodeValues::U8(vec![b'A']), DataFormat::Ascii),
            ),
            item(
                "u32, one value",
                DataNodeContent::new(NodeValues::U32(vec![0x4142_4344]), DataFormat::Ascii),
            ),
            item(
                "u16, several",
                DataNodeContent::new(NodeValues::U16(vec![0x4869, 0x2074, 0x6865, 0x7265]), DataFormat::Ascii)
                    .with_layout(GridLayout::Rows(1)),
            ),
        ],
        vec![item(
            "u8, several: a space shows as ␣ and an unprintable byte as ·",
            DataNodeContent::new(NodeValues::U8(b"Hello, World!\n\x00\xFF".to_vec()), DataFormat::Ascii)
                .with_layout(GridLayout::Rows(2)),
        )],
        vec![
            item(
                "Plain text, one run",
                DataNodeContent::new(NodeValues::U8(PLAIN.to_vec()), DataFormat::PlainText),
            ),
            item(
                "Plain text, short",
                DataNodeContent::new(NodeValues::U8(b"svg-dom-graph".to_vec()), DataFormat::PlainText),
            ),
        ],
    ];

    let scene = build_lines_scene("ascii-diagram", lines, true)?;
    // Keeps this Scene's only strong handle alive for the page's lifetime — see ASCII_SCENE's own doc comment.
    ASCII_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
