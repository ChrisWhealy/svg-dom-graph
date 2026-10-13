//! `NodeValues::Bool`, `Usize` and `Isize`: every `BoolFormat`, the marks and what is said for them, a Boolean's fixed
//! width, and the pointer-sized integers, whose width is that of the target.

use super::super::*;
use super::support::{cells, check_eq};
use crate::test_support::check;
use std::mem::size_of;

fn bools(values: Vec<bool>, format: DataFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(values), format)
}

fn widest(content: &DataNodeContent) -> String {
    let mut out = String::new();
    content.widest_cell_string(&mut out);
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Booleans
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_bool_format_writes_true_and_false_its_own_way() -> Result<(), String> {
    for (format, expected) in [
        (BoolFormat::TrueFalse, ["true", "false"]),
        (BoolFormat::Bit, ["1", "0"]),
        (BoolFormat::Icon, ["\u{2714}", "\u{2718}"]),
    ] {
        check_eq(
            cells(&bools(vec![true, false], DataFormat::Bool(format))),
            expected.iter().map(ToString::to_string).collect(),
        )?;
    }
    check_eq(BoolFormat::default(), BoolFormat::TrueFalse)
}

#[test]
fn the_other_formats_read_a_bool_as_the_byte_it_is_held_in() -> Result<(), String> {
    let texts = |format| cells(&bools(vec![true, false], format));
    check_eq(texts(DataFormat::Decimal), vec!["1".to_owned(), "0".to_owned()])?;
    check_eq(texts(DataFormat::Hexadecimal), vec!["01".to_owned(), "00".to_owned()])?;
    check_eq(texts(DataFormat::Binary), vec!["0000 0001".to_owned(), "0000 0000".to_owned()])
}

#[test]
fn a_bool_node_is_drawn_for_false_whichever_values_it_holds() -> Result<(), String> {
    for (format, widest_text) in [
        (DataFormat::Bool(BoolFormat::TrueFalse), "false"),
        (DataFormat::Bool(BoolFormat::Bit), "0"),
        (DataFormat::Bool(BoolFormat::Icon), "\u{2718}"),
        (DataFormat::Hexadecimal, "00"),
    ] {
        // Every value is `true`, and the node is still drawn wide enough to be changed to `false`.
        check_eq(widest(&bools(vec![true, true], format)), widest_text.to_owned())?;
    }
    check_eq(
        widest(&bools(Vec::new(), DataFormat::Bool(BoolFormat::TrueFalse))),
        String::new(),
    )
}

#[test]
fn a_bool_format_is_valid_only_for_bool_values() -> Result<(), String> {
    let format = DataFormat::Bool(BoolFormat::TrueFalse);
    check_eq(bools(vec![true], format).bool_format_is_valid(), true)?;
    for values in [NodeValues::U8(vec![1]), NodeValues::Usize(vec![1]), NodeValues::F32(vec![1.0])] {
        check_eq(DataNodeContent::new(values, format).bool_format_is_valid(), false)?;
    }
    check_eq(bools(vec![true], DataFormat::Hexadecimal).bool_format_is_valid(), true)?;
    check_eq(bools(vec![true], DataFormat::Decimal).holds_bools(), true)?;
    check_eq(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).holds_bools(),
        false,
    )
}

#[test]
fn only_an_icon_has_a_colour_of_its_own_and_is_spoken_as_a_word() -> Result<(), String> {
    let icon = bools(vec![true], DataFormat::Bool(BoolFormat::Icon));
    check_eq(icon.spoken("\u{2714}"), "true")?;
    check_eq(icon.spoken("\u{2718}"), "false")?;
    check(
        icon.ink("\u{2714}") != icon.ink("\u{2718}"),
        "a check mark and a cross have different colours",
    )?;
    check(
        icon.ink("\u{2714}") != crate::colours::TEXT_FILL && icon.ink("\u{2718}") != crate::colours::TEXT_FILL,
        "a mark is not drawn in the text colour",
    )?;
    // Words and bits are text, in the ordinary colour, and are read as they are written.
    let words = bools(vec![true], DataFormat::Bool(BoolFormat::TrueFalse));
    check_eq(words.spoken("true"), "true")?;
    check_eq(words.ink("true"), crate::colours::TEXT_FILL)?;
    // The same glyph in content that is not an icon is only text.
    check_eq(words.spoken("\u{2714}"), "\u{2714}")?;
    check_eq(words.ink("\u{2714}"), crate::colours::TEXT_FILL)
}

#[test]
fn only_icon_content_has_marks_to_measure() -> Result<(), String> {
    check_eq(
        bools(vec![true], DataFormat::Bool(BoolFormat::Icon)).bool_icon_marks(),
        Some(["\u{2714}", "\u{2718}"]),
    )?;
    check_eq(bools(vec![true], DataFormat::Bool(BoolFormat::Bit)).bool_icon_marks(), None)
}

#[test]
fn a_bool_is_never_equal_to_a_number_and_changes_only_when_it_flips() -> Result<(), String> {
    check(
        NodeValues::Bool(vec![true]) != NodeValues::U8(vec![1]),
        "a node showing a Boolean is not showing a number",
    )?;
    let mut content = bools(vec![true, false], DataFormat::Bool(BoolFormat::TrueFalse));
    let mut changed = Vec::new();
    let mut scratch = String::new();
    let accepted = content
        .try_replace_values(NodeValues::Bool(vec![true, true]), &mut scratch, |i, text| -> Result<(), ()> {
            changed.push((i, text.to_owned()));
            Ok(())
        })
        .unwrap_or(false);
    check_eq(accepted, true)?;
    check_eq(changed, vec![(1, "true".to_owned())])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   usize and isize
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_usize_reads_in_decimal_across_its_whole_range() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::Usize(vec![0, 1, usize::MAX]), DataFormat::Decimal);
    check_eq(cells(&content), vec!["0".to_owned(), "1".to_owned(), usize::MAX.to_string()])
}

#[test]
fn an_isize_shows_a_sign_only_when_it_is_negative() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::Isize(vec![0, 7, -7, isize::MAX, isize::MIN]), DataFormat::Decimal);
    check_eq(
        cells(&content),
        vec![
            "0".to_owned(),
            "7".to_owned(),
            "-7".to_owned(),
            isize::MAX.to_string(),
            isize::MIN.to_string(),
        ],
    )
}

/// A pointer-sized value is as many bytes as the target's pointer is, so this holds on a 32-bit and on a 64-bit target
/// alike.
#[test]
fn a_usize_is_as_many_bytes_as_a_pointer_in_hexadecimal_and_binary() -> Result<(), String> {
    let bytes = size_of::<usize>();
    let hex = cells(&DataNodeContent::new(NodeValues::Usize(vec![1]), DataFormat::Hexadecimal)).remove(0);
    check_eq(hex.len(), 3 * bytes - 1)?;
    check(hex.ends_with("01"), "the least significant byte is last")?;
    let binary = cells(&DataNodeContent::new(NodeValues::Usize(vec![1]), DataFormat::Binary)).remove(0);
    check_eq(binary.len(), 10 * bytes - 1)?;
    let little = DataNodeContent::new(NodeValues::Usize(vec![1]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check(
        cells(&little).remove(0).starts_with("01"),
        "the least significant byte is first",
    )
}

#[test]
fn an_isize_is_its_twos_complement_in_hexadecimal() -> Result<(), String> {
    let all_ff = vec!["FF"; size_of::<isize>()].join(" ");
    check_eq(
        cells(&DataNodeContent::new(NodeValues::Isize(vec![-1]), DataFormat::Hexadecimal)),
        vec![all_ff],
    )
}

#[test]
fn the_widest_pointer_sized_value_is_the_largest_or_the_most_negative() -> Result<(), String> {
    check_eq(
        widest(&DataNodeContent::new(
            NodeValues::Usize(vec![5, usize::MAX, 9]),
            DataFormat::Decimal,
        )),
        usize::MAX.to_string(),
    )?;
    // `isize::MIN` has a sign, so it is one character longer than `isize::MAX`.
    check_eq(
        widest(&DataNodeContent::new(
            NodeValues::Isize(vec![isize::MAX, 3, isize::MIN]),
            DataFormat::Decimal,
        )),
        isize::MIN.to_string(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Types
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_type_has_a_name_and_a_colour_of_its_own() -> Result<(), String> {
    let values = [
        NodeValues::U8(vec![1]),
        NodeValues::U16(vec![1]),
        NodeValues::U32(vec![1]),
        NodeValues::U64(vec![1]),
        NodeValues::U128(vec![1]),
        NodeValues::Usize(vec![1]),
        NodeValues::I8(vec![1]),
        NodeValues::I16(vec![1]),
        NodeValues::I32(vec![1]),
        NodeValues::I64(vec![1]),
        NodeValues::I128(vec![1]),
        NodeValues::Isize(vec![1]),
        NodeValues::F32(vec![1.0]),
        NodeValues::F64(vec![1.0]),
        NodeValues::Uuid(vec![Uuid::from_u128(1)]),
        NodeValues::Bool(vec![true]),
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
        (values[5].type_name(), values[11].type_name(), values[15].type_name()),
        ("usize", "isize", "bool"),
    )
}
