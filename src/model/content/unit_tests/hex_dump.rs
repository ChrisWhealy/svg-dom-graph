//! `DataFormat::HexDump`: what `hexdump -C` writes, one cell for each line, with its offset hidden unless asked for.

use super::super::hex_dump_format::{ascii_column, hex_column};
use super::super::*;
use super::support::{cells, check_eq};

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn dump(bytes: &[u8], format: HexDumpFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(bytes.to_vec()), DataFormat::HexDump(format))
}

/// A line without its trailing padding, which only keeps the cells the same width.
fn visible(line: &str) -> &str {
    line.trim_end()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_offset_is_hidden_unless_asked_for() -> Result<(), String> {
    check(!HexDumpFormat::default().shows_offset(), "hidden by default")?;
    check(!HexDumpFormat::new().shows_offset(), "hidden by new")?;
    check(HexDumpFormat::new().with_offset(true).shows_offset(), "shown when asked for")
}

#[test]
fn a_full_line_is_written_as_hexdump_c_writes_it_without_its_offset() -> Result<(), String> {
    // `printf '0123456789abcdef' | hexdump -C` gives
    // 00000000  30 31 32 33 34 35 36 37  38 39 61 62 63 64 65 66  |0123456789abcdef|
    let content = dump(b"0123456789abcdef", HexDumpFormat::new());
    check_eq(
        cells(&content),
        vec!["30 31 32 33 34 35 36 37  38 39 61 62 63 64 65 66  |0123456789abcdef|".to_owned()],
    )
}

#[test]
fn the_offset_when_shown_is_eight_lowercase_hex_digits_and_two_spaces() -> Result<(), String> {
    let content = dump(b"0123456789abcdef", HexDumpFormat::new().with_offset(true));
    let lines = cells(&content);
    check_eq(
        visible(&lines[0]),
        "00000000  30 31 32 33 34 35 36 37  38 39 61 62 63 64 65 66  |0123456789abcdef|",
    )?;
    // The line of the total length is not part of the dump unless it is asked for.
    check_eq(lines.len(), 1)
}

#[test]
fn the_total_length_line_is_shown_only_when_asked_for_and_needs_no_offsets() -> Result<(), String> {
    check(!HexDumpFormat::new().shows_total_length(), "hidden by default")?;
    for offsets in [false, true] {
        let format = HexDumpFormat::new().with_offset(offsets).with_total_length(true);
        check(format.shows_total_length(), "shown when asked for")?;
        let content = dump(b"0123456789abcdef", format);
        let lines = cells(&content);
        // `hexdump -C` ends with a line holding the total length.
        check_eq(visible(&lines[1]), "00000010")?;
        check_eq(lines.len(), 2)?;
        check_eq(content.selectable_len(), 16)?;
    }
    Ok(())
}

#[test]
fn a_short_last_line_keeps_the_characters_lined_up_and_non_printable_bytes_are_dots() -> Result<(), String> {
    // `printf 'Hello, World!\n' | hexdump -C` gives
    // 00000000  48 65 6c 6c 6f 2c 20 57  6f 72 6c 64 21 0a        |Hello, World!.|
    let content = dump(
        b"Hello, World!\n",
        HexDumpFormat::new().with_offset(true).with_total_length(true),
    );
    let lines = cells(&content);
    check_eq(
        visible(&lines[0]),
        "00000000  48 65 6c 6c 6f 2c 20 57  6f 72 6c 64 21 0a        |Hello, World!.|",
    )?;
    check_eq(visible(&lines[1]), "0000000e")
}

#[test]
fn a_dump_spans_a_line_for_every_sixteen_bytes() -> Result<(), String> {
    let bytes: Vec<u8> = (0..40).collect();
    let content = dump(&bytes, HexDumpFormat::new());
    let lines = cells(&content);
    check_eq(lines.len(), 3)?;
    check(
        lines[0].starts_with("00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  |................|"),
        &lines[0],
    )?;
    check(lines[2].starts_with("20 21 22 23 24 25 26 27 "), &lines[2])?;
    check(
        visible(&lines[2]).ends_with("|  !\"#$%&'|") || visible(&lines[2]).ends_with("| !\"#$%&'|"),
        &lines[2],
    )
}

#[test]
fn every_line_is_the_same_width_so_the_cells_line_up() -> Result<(), String> {
    for show in [false, true] {
        let content = dump(b"Hello, World! and some more", HexDumpFormat::new().with_offset(show));
        let mut widest = String::new();
        content.widest_cell_string(&mut widest);
        for line in cells(&content) {
            check_eq(line.chars().count(), widest.chars().count())?;
        }
    }
    Ok(())
}

#[test]
fn the_cells_are_one_column_of_lines_whatever_layout_is_asked_for() -> Result<(), String> {
    let bytes = vec![0u8; 50];
    let format = HexDumpFormat::new().with_offset(true).with_total_length(true);
    let content = dump(&bytes, format).with_layout(GridLayout::Columns(4));
    // 4 lines of bytes and the line of the length.
    check_eq(content.len(), 5)?;
    check_eq(content.shape(), (5, 1))
}

#[test]
fn one_line_is_still_a_screen_of_lines_and_not_a_single_value() -> Result<(), String> {
    let content = dump(b"abc", HexDumpFormat::new());
    check_eq(content.len(), 1)?;
    check(!content.is_single_value(), "a line of a dump is not a value that stands alone")
}

#[test]
fn no_bytes_make_no_cells() -> Result<(), String> {
    check_eq(
        dump(b"", HexDumpFormat::new().with_offset(true).with_total_length(true)).len(),
        0,
    )
}

#[test]
fn only_u8_values_are_a_valid_dump() -> Result<(), String> {
    check(dump(b"abc", HexDumpFormat::new()).hex_dump_is_valid(), "u8 is valid")?;
    let wide = DataNodeContent::new(NodeValues::U16(vec![1, 2]), DataFormat::HexDump(HexDumpFormat::new()));
    check(!wide.hex_dump_is_valid(), "u16 is not valid")?;
    let other = DataNodeContent::new(NodeValues::U16(vec![1, 2]), DataFormat::Hexadecimal);
    check(other.hex_dump_is_valid(), "another format has nothing to check")
}

#[test]
fn replacing_the_bytes_writes_only_the_lines_that_changed() -> Result<(), String> {
    let mut bytes = vec![0u8; 40];
    let mut content = dump(&bytes, HexDumpFormat::new());
    // Byte 20 is on the second line.
    bytes[20] = 0xff;
    let mut written = Vec::new();
    let mut scratch = String::new();
    let replaced = content
        .try_replace_values(NodeValues::U8(bytes), &mut scratch, |i, text| {
            written.push((i, text.to_owned()));
            Ok::<(), ()>(())
        })
        .map_err(|()| "update failed".to_owned())?;
    check(replaced, "same count of u8 values is accepted")?;
    check_eq(written.len(), 1)?;
    check_eq(written[0].0, 1)?;
    check(written[0].1.contains("ff"), &written[0].1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Selecting a byte
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Where byte `index` of the first line sits in `line`: the two characters its hex digits take, and the one its
/// character takes, as found by looking in the text itself.
fn found(line: &str, index: usize, format: HexDumpFormat) -> (String, char) {
    let hex = hex_column(index, format);
    let ascii = ascii_column(index, format);
    (line[hex..hex + 2].to_owned(), line.as_bytes()[ascii] as char)
}

#[test]
fn the_columns_of_a_byte_are_where_its_digits_and_its_character_are_written() -> Result<(), String> {
    let bytes: Vec<u8> = (0x41..0x51).collect();
    for show in [false, true] {
        let format = HexDumpFormat::new().with_offset(show);
        let content = dump(&bytes, format);
        let line = cells(&content).remove(0);
        for (index, &byte) in bytes.iter().enumerate() {
            check_eq(found(&line, index, format), (format!("{byte:02x}"), char::from(byte)))?;
        }
    }
    Ok(())
}

#[test]
fn a_hex_dump_is_selected_a_byte_at_a_time_and_not_a_line() -> Result<(), String> {
    let content = dump(&[0u8; 40], HexDumpFormat::new().with_offset(true).with_total_length(true));
    // 3 lines and the line of the length, but 40 bytes.
    check_eq(content.len(), 4)?;
    check_eq(content.selectable_len(), 40)?;
    check(
        content.resolve_selection(Selection::Cell(39)).is_some(),
        "the last byte can be selected",
    )?;
    check(
        content.resolve_selection(Selection::Cell(40)).is_none(),
        "one past the last cannot",
    )?;
    check(
        content.resolve_selection(Selection::Row { row: 0, col: None }).is_none(),
        "a row is not a byte",
    )?;
    check_eq(content.natural_selection(39), Some(Selection::Cell(39)))?;
    check_eq(content.natural_selection(40), None)?;
    check_eq(content.flat_index(&Selection::Cell(17)), Some(17))
}

#[test]
fn another_format_is_selected_by_its_cells() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Hexadecimal);
    check_eq(content.selectable_len(), 3)
}

#[test]
fn a_byte_is_found_with_its_line_and_its_place_in_the_line() -> Result<(), String> {
    let bytes: Vec<u8> = (0..40).collect();
    let content = dump(&bytes, HexDumpFormat::new());
    check_eq(content.hex_dump_byte(0), Some((0, 0, 0)))?;
    check_eq(content.hex_dump_byte(17), Some((17, 1, 1)))?;
    check_eq(content.hex_dump_byte(39), Some((39, 2, 7)))?;
    check_eq(content.hex_dump_byte(40), None)?;
    check_eq(
        DataNodeContent::new(NodeValues::U8(bytes), DataFormat::Hexadecimal).hex_dump_byte(0),
        None,
    )
}

#[test]
fn a_dump_is_a_terminal_screen_and_never_a_single_value() -> Result<(), String> {
    check_eq(dump(b"abc", HexDumpFormat::new()).type_colour(), "#000000")?;
    check_eq(dump(b"abc", HexDumpFormat::new()).ink("x"), "#e6e6e6")
}

#[test]
fn a_toolbar_row_is_only_asked_for_with_with_toolbar() -> Result<(), String> {
    check(!dump(b"abc", HexDumpFormat::new()).has_toolbar_row(), "none by default")?;
    check(dump(b"abc", HexDumpFormat::new()).with_toolbar().has_toolbar_row(), "asked for")
}

#[test]
fn identical_lines_are_all_written_and_never_collapsed_to_a_star() -> Result<(), String> {
    // `hexdump -C` would write the second and third of these as `*`. Each line is a cell, so each is written in full.
    let content = dump(&[0xAA; 64], HexDumpFormat::new());
    let lines = cells(&content);
    check_eq(lines.len(), 4)?;
    for line in &lines {
        check_eq(visible(line), visible(&lines[0]))?;
        check(!line.contains('*'), "a line is never a star")?;
    }
    Ok(())
}
