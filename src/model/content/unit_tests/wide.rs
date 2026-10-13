//! `NodeValues::U128`/`I128`/`Uuid`: the whole range of a 128-bit value, every [`UuidFormat`], and the width of a
//! cell, which for a UUID is the same whatever its value.

use super::super::*;
use super::support::{cells, check_eq};
use crate::test_support::check;

fn uuid_text(value: u128, format: UuidFormat) -> String {
    let content = DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(value)]), DataFormat::Uuid(format));
    cells(&content).remove(0)
}

/// An illustrative UUID value, with no specific meaning of its own.
const EXAMPLE: u128 = 0x123e4567_e89b_12d3_a456_426614174000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   u128 and i128
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_u128_reads_in_decimal_across_its_whole_range() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U128(vec![0, 1, u128::MAX]), DataFormat::Decimal);
    check_eq(
        cells(&content),
        vec![
            "0".to_owned(),
            "1".to_owned(),
            "340282366920938463463374607431768211455".to_owned(),
        ],
    )
}

#[test]
fn an_i128_shows_a_sign_only_when_it_is_negative() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::I128(vec![0, 7, -7, i128::MAX, i128::MIN]), DataFormat::Decimal);
    check_eq(
        cells(&content),
        vec![
            "0".to_owned(),
            "7".to_owned(),
            "-7".to_owned(),
            "170141183460469231731687303715884105727".to_owned(),
            "-170141183460469231731687303715884105728".to_owned(),
        ],
    )
}

#[test]
fn a_u128_is_sixteen_bytes_in_hexadecimal_in_either_byte_order() -> Result<(), String> {
    let value: u128 = 0x0001_0203_0405_0607_0809_0A0B_0C0D_0E0F;
    let big = DataNodeContent::new(NodeValues::U128(vec![value]), DataFormat::Hexadecimal);
    check_eq(cells(&big), vec!["00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F".to_owned()])?;
    let little = big.with_byte_order(ByteOrder::LittleEndian);
    check_eq(
        cells(&little),
        vec!["0F 0E 0D 0C 0B 0A 09 08 07 06 05 04 03 02 01 00".to_owned()],
    )
}

#[test]
fn an_i128_is_its_twos_complement_in_hexadecimal() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::I128(vec![-1, i128::MIN]), DataFormat::Hexadecimal);
    check_eq(
        cells(&content),
        vec![
            "FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF".to_owned(),
            "80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00".to_owned(),
        ],
    )
}

#[test]
fn a_u128_is_sixteen_bytes_in_binary() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U128(vec![1]), DataFormat::Binary);
    let text = cells(&content).remove(0);
    check_eq(text.len(), 16 * 9 + 15)?;
    check(text.ends_with("0000 0001"), "the least significant byte is last")
}

#[test]
fn the_widest_u128_is_the_largest() -> Result<(), String> {
    let mut out = String::new();
    DataNodeContent::new(NodeValues::U128(vec![5, u128::MAX, 1_000]), DataFormat::Decimal).widest_cell_string(&mut out);
    check_eq(out.as_str(), "340282366920938463463374607431768211455")
}

#[test]
fn the_widest_i128_is_the_most_negative_when_its_sign_makes_it_longer() -> Result<(), String> {
    // i128::MIN has the same digits as i128::MAX plus one more, and a sign.
    let mut out = String::new();
    DataNodeContent::new(NodeValues::I128(vec![i128::MAX, 3, i128::MIN]), DataFormat::Decimal)
        .widest_cell_string(&mut out);
    check_eq(out.as_str(), "-170141183460469231731687303715884105728")?;
    DataNodeContent::new(NodeValues::I128(vec![i128::MAX, -3]), DataFormat::Decimal).widest_cell_string(&mut out);
    check_eq(out.as_str(), "170141183460469231731687303715884105727")
}

#[test]
fn the_128_bit_types_are_named_and_coloured_apart_from_every_other_type() -> Result<(), String> {
    let values = [
        NodeValues::U8(vec![1]),
        NodeValues::U16(vec![1]),
        NodeValues::U32(vec![1]),
        NodeValues::U64(vec![1]),
        NodeValues::U128(vec![1]),
        NodeValues::I8(vec![1]),
        NodeValues::I16(vec![1]),
        NodeValues::I32(vec![1]),
        NodeValues::I64(vec![1]),
        NodeValues::I128(vec![1]),
        NodeValues::F32(vec![1.0]),
        NodeValues::F64(vec![1.0]),
        NodeValues::Uuid(vec![Uuid::from_u128(1)]),
    ];
    let mut colours: Vec<_> = values.iter().map(NodeValues::type_colour).collect();
    colours.sort_unstable();
    colours.dedup();
    check_eq(colours.len(), values.len())?;
    let mut names: Vec<_> = values.iter().map(NodeValues::type_name).collect();
    names.sort_unstable();
    names.dedup();
    check_eq(names.len(), values.len())?;
    check_eq(
        (values[4].type_name(), values[9].type_name(), values[12].type_name()),
        ("u128", "i128", "uuid"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   UUID text
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_default_format_is_the_canonical_lowercase_hyphenated_form() -> Result<(), String> {
    check_eq(
        uuid_text(EXAMPLE, UuidFormat::new()),
        "123e4567-e89b-12d3-a456-426614174000".to_owned(),
    )?;
    check_eq(UuidFormat::new(), UuidFormat::default())
}

#[test]
fn every_style_punctuates_the_same_digits_its_own_way() -> Result<(), String> {
    let format = |style| UuidFormat::new().with_style(style);
    check_eq(
        uuid_text(EXAMPLE, format(UuidStyle::Hyphenated)),
        "123e4567-e89b-12d3-a456-426614174000".to_owned(),
    )?;
    check_eq(
        uuid_text(EXAMPLE, format(UuidStyle::Simple)),
        "123e4567e89b12d3a456426614174000".to_owned(),
    )?;
    check_eq(
        uuid_text(EXAMPLE, format(UuidStyle::Braced)),
        "{123e4567-e89b-12d3-a456-426614174000}".to_owned(),
    )?;
    check_eq(
        uuid_text(EXAMPLE, format(UuidStyle::Urn)),
        "urn:uuid:123e4567-e89b-12d3-a456-426614174000".to_owned(),
    )
}

#[test]
fn uppercase_changes_the_digits_and_never_the_urn_prefix() -> Result<(), String> {
    let upper = UuidFormat::new().with_case(UuidCase::Upper);
    check_eq(uuid_text(EXAMPLE, upper), "123E4567-E89B-12D3-A456-426614174000".to_owned())?;
    check_eq(
        uuid_text(EXAMPLE, upper.with_style(UuidStyle::Urn)),
        "urn:uuid:123E4567-E89B-12D3-A456-426614174000".to_owned(),
    )?;
    check_eq(
        uuid_text(EXAMPLE, upper.with_style(UuidStyle::Braced)),
        "{123E4567-E89B-12D3-A456-426614174000}".to_owned(),
    )
}

#[test]
fn the_nil_and_max_uuids_are_all_zeros_and_all_ones() -> Result<(), String> {
    check_eq(
        uuid_text(0, UuidFormat::new()),
        "00000000-0000-0000-0000-000000000000".to_owned(),
    )?;
    check_eq(
        uuid_text(u128::MAX, UuidFormat::new()),
        "ffffffff-ffff-ffff-ffff-ffffffffffff".to_owned(),
    )
}

#[test]
fn a_leading_zero_is_never_dropped() -> Result<(), String> {
    check_eq(
        uuid_text(1, UuidFormat::new()),
        "00000000-0000-0000-0000-000000000001".to_owned(),
    )
}

#[test]
fn the_guid_layout_reverses_the_first_three_fields_and_no_others() -> Result<(), String> {
    // The sixteen bytes of a GUID as they sit in memory.
    let in_memory = Uuid::from_bytes([
        0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
    ]);
    let content = |layout| {
        DataNodeContent::new(
            NodeValues::Uuid(vec![in_memory]),
            DataFormat::Uuid(UuidFormat::new().with_layout(layout)),
        )
    };
    check_eq(
        cells(&content(UuidLayout::Guid)),
        vec!["00112233-4455-6677-8899-aabbccddeeff".to_owned()],
    )?;
    // Read as RFC bytes, the same sixteen bytes are a different UUID.
    check_eq(
        cells(&content(UuidLayout::Rfc)),
        vec!["33221100-5544-7766-8899-aabbccddeeff".to_owned()],
    )
}

#[test]
fn the_layout_changes_the_bytes_that_are_read_and_not_the_style() -> Result<(), String> {
    let guid = UuidFormat::new().with_layout(UuidLayout::Guid).with_style(UuidStyle::Simple);
    check_eq(uuid_text(EXAMPLE, guid).len(), 32)
}

#[test]
fn every_format_has_one_width_whatever_the_value() -> Result<(), String> {
    for (style, width) in [
        (UuidStyle::Simple, 32),
        (UuidStyle::Hyphenated, 36),
        (UuidStyle::Braced, 38),
        (UuidStyle::Urn, 45),
    ] {
        for case in [UuidCase::Lower, UuidCase::Upper] {
            for layout in [UuidLayout::Rfc, UuidLayout::Guid] {
                let format = UuidFormat::new().with_style(style).with_case(case).with_layout(layout);
                for value in [0, 1, EXAMPLE, u128::MAX] {
                    check_eq(uuid_text(value, format).len(), width)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn the_widest_uuid_is_the_first_since_every_one_is_as_wide() -> Result<(), String> {
    let mut out = String::new();
    let values = vec![Uuid::from_u128(EXAMPLE), Uuid::from_u128(0)];
    DataNodeContent::new(NodeValues::Uuid(values), DataFormat::Uuid(UuidFormat::new())).widest_cell_string(&mut out);
    check_eq(out.as_str(), "123e4567-e89b-12d3-a456-426614174000")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   UUID as bytes and as a number
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_uuid_reads_as_bytes_or_as_a_number_in_the_other_formats() -> Result<(), String> {
    let uuid = vec![Uuid::from_u128(EXAMPLE)];
    let as_format = |format| DataNodeContent::new(NodeValues::Uuid(uuid.clone()), format);
    check_eq(
        cells(&as_format(DataFormat::Hexadecimal)),
        vec!["12 3E 45 67 E8 9B 12 D3 A4 56 42 66 14 17 40 00".to_owned()],
    )?;
    check_eq(
        cells(&as_format(DataFormat::Hexadecimal).with_byte_order(ByteOrder::LittleEndian)),
        vec!["00 40 17 14 66 42 56 A4 D3 12 9B E8 67 45 3E 12".to_owned()],
    )?;
    check_eq(cells(&as_format(DataFormat::Decimal)), vec![EXAMPLE.to_string()])
}

#[test]
fn the_byte_order_does_not_change_a_uuids_own_text() -> Result<(), String> {
    let content = DataNodeContent::new(
        NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]),
        DataFormat::Uuid(UuidFormat::new()),
    );
    let little = content.clone().with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&content), cells(&little))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Validity, equality and the Uuid type
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_uuid_format_is_valid_only_for_uuid_values() -> Result<(), String> {
    let format = DataFormat::Uuid(UuidFormat::new());
    let uuid = DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(1)]), format);
    check_eq(uuid.uuid_format_is_valid(), true)?;
    for values in [NodeValues::U128(vec![1]), NodeValues::U64(vec![1]), NodeValues::F64(vec![1.0])] {
        check_eq(DataNodeContent::new(values, format).uuid_format_is_valid(), false)?;
    }
    // Uuid values under another format are valid, since that format is plain bytes or a number.
    check_eq(
        DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(1)]), DataFormat::Hexadecimal)
            .uuid_format_is_valid(),
        true,
    )
}

#[test]
fn only_uuid_content_holds_uuids() -> Result<(), String> {
    check_eq(
        DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(1)]), DataFormat::Decimal).holds_uuids(),
        true,
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U128(vec![1]), DataFormat::Decimal).holds_uuids(),
        false,
    )
}

#[test]
fn a_u128_and_a_uuid_of_the_same_bits_are_different_values() -> Result<(), String> {
    check(
        NodeValues::U128(vec![EXAMPLE]) != NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]),
        "a node showing a number is not showing a UUID",
    )?;
    check_eq(NodeValues::U128(vec![EXAMPLE]), NodeValues::U128(vec![EXAMPLE]))?;
    check_eq(
        NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]),
        NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]),
    )
}

#[test]
fn replacing_a_uuid_reports_only_the_one_that_changed() -> Result<(), String> {
    let first = Uuid::from_u128(EXAMPLE);
    let mut content = DataNodeContent::new(
        NodeValues::Uuid(vec![first, Uuid::from_u128(0)]),
        DataFormat::Uuid(UuidFormat::new()),
    );
    let mut changed = Vec::new();
    let mut scratch = String::new();
    let accepted = content
        .try_replace_values(
            NodeValues::Uuid(vec![first, Uuid::from_u128(u128::MAX)]),
            &mut scratch,
            |i, text| -> Result<(), ()> {
                changed.push((i, text.to_owned()));
                Ok(())
            },
        )
        .unwrap_or(false);
    check_eq(accepted, true)?;
    check_eq(changed, vec![(1, "ffffffff-ffff-ffff-ffff-ffffffffffff".to_owned())])
}

#[test]
fn a_uuid_converts_to_and_from_its_bytes_and_its_number() -> Result<(), String> {
    let uuid = Uuid::from_u128(EXAMPLE);
    check_eq(Uuid::from_bytes(uuid.to_bytes()), uuid)?;
    check_eq(uuid.to_bytes()[0], 0x12)?;
    check_eq(u128::from(uuid), EXAMPLE)?;
    check_eq(Uuid::from(EXAMPLE), uuid)?;
    check_eq(Uuid::from([0_u8; 16]), Uuid::default())
}
