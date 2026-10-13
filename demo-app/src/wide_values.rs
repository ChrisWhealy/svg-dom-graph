//! `panel-128-bit` / `#wide-diagram`: the 128-bit values a data node can hold. A `u128` and an `i128` across their
//! whole range, and a `Uuid` in every way a `UuidFormat` can write it. See [`build_wide_demo`]'s own doc comment for
//! what each node shows.

use crate::util::{ensure_svg_in, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::scene::{
    DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene, Side, ToolbarOptions, Uuid, UuidCase,
    UuidFormat, UuidLayout, UuidStyle,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("wide_values.rs");

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

/// What each `u128` is, written beside its row. They are zero, one, the first value that does not fit in 64 bits, the
/// first that has its top bit set, and the largest.
const UNSIGNED_LABELS: &[&str] = &["zero", "one", "2^64", "2^127", "max"];
const UNSIGNED: [u128; 5] = [0, 1, 1 << 64, 1 << 127, u128::MAX];

/// What each `i128` is: the most negative, a negative that does not fit in 64 bits, minus one, zero and the largest.
const SIGNED_LABELS: &[&str] = &["min", "-2^64", "-1", "zero", "max"];
const SIGNED: [i128; 5] = [i128::MIN, -(1 << 64), -1, 0, i128::MAX];

/// A UUID of each version that RFC 9562 defines, and the nil and max UUIDs, which have no version. Version 2 is not
/// among them, since RFC 9562 reserves it for DCE Security and does not define it. Each is a published
/// example, so each is a real UUID of its version and a reader can look it up.
const VERSION_LABELS: &[&str] = &[
    "nil", "v1 time", "v3 MD5", "v4 random", "v5 SHA-1", "v6 time", "v7 Unix time", "v8 custom", "max",
];
const VERSIONS: [(u128, Option<u8>); 9] = [
    (0, None),
    (0xc232ab00_9414_11ec_b3c8_9f6bdeced846, Some(1)),
    (0x6fa459ea_ee8a_3ca4_894e_db77e160355e, Some(3)),
    (0x9b1deb4d_3b7d_4bad_9bdd_2b0d7b3dcb6d, Some(4)),
    (0x886313e1_3b8a_5372_9b90_0c9aee199e5d, Some(5)),
    (0x1ec9414c_232a_6b00_b3c8_9f6bdeced846, Some(6)),
    (0x017f22e2_79b0_7cc3_98c4_dc0c0c07398f, Some(7)),
    (0x2489e9ad_2ee2_8e00_8ec9_32d5f69181c0, Some(8)),
    (u128::MAX, None),
];

/// The UUID that every format below writes, which is the example RFC 9562 itself uses.
const EXAMPLE: u128 = 0x123e4567_e89b_12d3_a456_426614174000;

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

/// `EXAMPLE` as one UUID value, shown in `format`.
fn example(format: UuidFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]), DataFormat::Uuid(format))
}

/// The sixteen bytes of `EXAMPLE` as a Microsoft GUID holds them in memory: the first three fields least significant
/// byte first. Reversing those fields is its own inverse, so this is also how a GUID's bytes become the UUID's.
fn example_as_guid_bytes() -> Uuid {
    let mut bytes = Uuid::from_u128(EXAMPLE).to_bytes();
    bytes[0..4].reverse();
    bytes[4..6].reverse();
    bytes[6..8].reverse();
    Uuid::from_bytes(bytes)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the 128-bit demo, one line of nodes at a time. Each line is read across.
///
/// - **Line 1: a `u128`,** in decimal and in hexadecimal. The values run from zero to the largest, which has 39 digits,
///   more than any `u64` can hold.
/// - **Line 2: an `i128`,** in decimal and in hexadecimal. A negative value has a sign in decimal, and in hexadecimal
///   it is two's complement, so `-1` is sixteen bytes of `FF`. The most negative value is the widest, since its sign
///   makes it one character longer than the largest.
/// - **Line 3: a UUID of every version.** The same kind of value as a `u128`, but written as a UUID, with the digit
///   that gives its version in the third group. The second node is the same UUIDs under `Decimal`, as the numbers
///   that they are.
/// - **Lines 4 and 5: every way to write one UUID.** The same UUID is hyphenated, simple, braced, a URN and uppercase.
///   Every format writes a fixed number of characters, so a node is as wide for one UUID as for any other.
/// - **Lines 6 and 7: byte layouts.** A UUID that is an RFC 9562 one is stored most significant byte first. A Microsoft
///   GUID is stored with its first three fields reversed, so the same UUID is a different sixteen bytes. A node shows
///   which bytes it holds, and then the layout that they are read in decides the text. Read in the wrong one, the same
///   bytes are a different UUID.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#wide-diagram`.
pub(crate) fn build_wide_demo() -> Result<(), String> {
    let document = crate::util::document()?;
    ensure_svg_in(&document, "wide-diagram-stage", "wide-diagram", Size::new(1400.0, 2200.0))?;
    let svg = SvgRoot::attach("wide-diagram").map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let uuids = |format: DataFormat| {
        column(
            NodeValues::Uuid(VERSIONS.iter().map(|&(value, _)| Uuid::from_u128(value)).collect()),
            format,
            VERSION_LABELS,
        )
    };
    let lines = [
        vec![
            item(
                "u128, decimal",
                column(NodeValues::U128(UNSIGNED.to_vec()), DataFormat::Decimal, UNSIGNED_LABELS),
            ),
            item(
                "u128, hexadecimal",
                column(NodeValues::U128(UNSIGNED.to_vec()), DataFormat::Hexadecimal, UNSIGNED_LABELS),
            ),
        ],
        vec![
            item(
                "i128, decimal",
                column(NodeValues::I128(SIGNED.to_vec()), DataFormat::Decimal, SIGNED_LABELS),
            ),
            item(
                "i128, hexadecimal",
                column(NodeValues::I128(SIGNED.to_vec()), DataFormat::Hexadecimal, SIGNED_LABELS),
            ),
        ],
        vec![
            item("UUID of every version", uuids(DataFormat::Uuid(UuidFormat::new()))),
            item("The same UUIDs, as numbers", uuids(DataFormat::Decimal)),
        ],
        vec![
            item("Hyphenated (the default)", example(UuidFormat::new())),
            item("Simple", example(UuidFormat::new().with_style(UuidStyle::Simple))),
            item("Braced", example(UuidFormat::new().with_style(UuidStyle::Braced))),
        ],
        vec![
            item("URN", example(UuidFormat::new().with_style(UuidStyle::Urn))),
            item("Uppercase", example(UuidFormat::new().with_case(UuidCase::Upper))),
        ],
        vec![
            item(
                "RFC 9562 bytes",
                DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]), DataFormat::Hexadecimal),
            ),
            item(
                "GUID bytes in memory",
                DataNodeContent::new(NodeValues::Uuid(vec![example_as_guid_bytes()]), DataFormat::Hexadecimal),
            ),
        ],
        vec![
            item(
                "GUID bytes, read as a GUID",
                DataNodeContent::new(
                    NodeValues::Uuid(vec![example_as_guid_bytes()]),
                    DataFormat::Uuid(UuidFormat::new().with_layout(UuidLayout::Guid)),
                ),
            ),
            item(
                "GUID bytes, read as RFC 9562",
                DataNodeContent::new(
                    NodeValues::Uuid(vec![example_as_guid_bytes()]),
                    DataFormat::Uuid(UuidFormat::new()),
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

    resize_svg(
        &document,
        "wide-diagram",
        Size::new(right + MARGIN + TOOLBAR_ROOM, y - V_GAP + MARGIN),
    )?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn every_label_names_its_own_value() -> Result<(), String> {
        for (what, values, labels) in [
            ("UNSIGNED", UNSIGNED.len(), UNSIGNED_LABELS.len()),
            ("SIGNED", SIGNED.len(), SIGNED_LABELS.len()),
            ("VERSIONS", VERSIONS.len(), VERSION_LABELS.len()),
        ] {
            if values != labels {
                return Err(format!("{what} has {values} values but {labels} labels"));
            }
        }
        Ok(())
    }

    /// A UUID names its own version in the top four bits of its seventh byte, and its variant in the top two bits of
    /// its ninth. RFC 9562's own variant is `10`.
    #[test]
    fn every_example_is_a_real_uuid_of_the_version_it_is_labelled_with() -> Result<(), String> {
        for (&(value, version), label) in VERSIONS.iter().zip(VERSION_LABELS) {
            let Some(version) = version else { continue };
            let field = (value >> 76) as u8 & 0xF;
            if field != version {
                return Err(format!("{label}: the version field is {field}, not {version}"));
            }
            let variant = (value >> 62) as u8 & 0b11;
            if variant != 0b10 {
                return Err(format!("{label}: the variant bits are {variant:02b}, not 10"));
            }
        }
        Ok(())
    }

    #[test]
    fn the_guid_bytes_are_the_example_with_its_first_three_fields_reversed() -> Result<(), String> {
        let bytes = example_as_guid_bytes().to_bytes();
        let expected = [
            0x67, 0x45, 0x3E, 0x12, 0x9B, 0xE8, 0xD3, 0x12, 0xA4, 0x56, 0x42, 0x66, 0x14, 0x17, 0x40, 0x00,
        ];
        if bytes == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:02X?}, got {bytes:02X?}"))
        }
    }
}
