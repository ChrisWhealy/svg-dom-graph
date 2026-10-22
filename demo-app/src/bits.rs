//! The bit array panel:
//!
//! - `panel-bits` / `#bits-diagram` — [`build_bits_demo`]: values from one to seven bits wide, single and in arrays.

use demo_common::util::{Item, build_lines_scene, item, stringify};
use std::cell::RefCell;
use svg_dom_graph::scene::{BitValues, DataFormat, DataNodeContent, NodeValues, Scene};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("bits.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own `Scene`.
    static BITS_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// The three ways a bit value can be written: as decimal digits, as hexadecimal digits, and as binary digits.
const FORMATS: [(&str, DataFormat); 3] = [
    ("decimal", DataFormat::Decimal),
    ("hexadecimal", DataFormat::Hexadecimal),
    ("binary", DataFormat::Binary),
];

/// `values`, each `width` bits wide.
fn bits(width: u8, values: &[u8]) -> Result<NodeValues, String> {
    Ok(NodeValues::Bits(BitValues::new(width, values.to_vec()).map_err(stringify)?))
}

/// One node for each of [`FORMATS`], holding `values` and named `{what}, {format}`.
fn in_every_format(what: &str, values: &NodeValues) -> Vec<Item> {
    FORMATS
        .iter()
        .map(|&(name, format)| item(format!("{what}, {name}"), DataNodeContent::new(values.clone(), format)))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the bit array demo: for each width from 1 to 7, a single value and several values, each written in decimal,
/// hexadecimal and binary. Every box is draggable.
///
/// A [`BitValues`] holds values that are all exactly `width` bits wide, and refuses one that does not fit. The binary
/// format writes exactly `width` digits with no gap, since the grouping of a byte into two nybbles means nothing for a
/// width that is not a multiple of four. The hexadecimal format writes one digit for each four bits, or part of four.
///
/// The last line shows why the type exists. The value 19 as a `u8` and as a six-bit value is the same number, but the
/// `u8` has two leading zero bits that the six-bit value does not.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#bits-diagram-stage`.
pub(crate) fn build_bits_demo() -> Result<(), String> {
    let mut lines = Vec::new();
    for (width, single, several) in [
        (1, &[1][..], &[1, 0, 1, 1, 0, 0, 1, 0][..]),
        (2, &[2], &[0, 1, 2, 3]),
        (3, &[5], &[0, 1, 2, 3, 4, 5, 6, 7]),
        (4, &[0xA], &[0x0, 0x3, 0x7, 0xA, 0xF]),
        (5, &[22], &[0, 7, 13, 22, 31, 17, 5, 29, 11]),
        (6, &[19], &[19, 22, 5, 46]),
        (7, &[0x41], &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x7F]),
    ] {
        lines.push(in_every_format(&format!("{width}-bit, one value"), &bits(width, single)?));
        lines.push(in_every_format(&format!("{width}-bit, several"), &bits(width, several)?));
    }
    lines.push(vec![
        item("u8, binary", DataNodeContent::new(NodeValues::U8(vec![19]), DataFormat::Binary)),
        item("6-bit, binary", DataNodeContent::new(bits(6, &[19])?, DataFormat::Binary)),
    ]);

    let scene = build_lines_scene("bits-diagram", lines, true)?;
    BITS_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
