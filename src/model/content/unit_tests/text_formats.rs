//! The text formats: `DataFormat::Ascii` and `DataFormat::PlainText`.

use super::super::*;
use super::support::{cells, check_eq};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   DataFormat::Ascii
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn ascii_shows_a_printable_byte_as_itself_a_space_as_a_visible_mark_and_anything_else_as_a_dot() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(b"Hi !~\x00\x7F\xFF".to_vec()), DataFormat::Ascii);
    check_eq(
        cells(&content),
        ["H", "i", "\u{2423}", "!", "~", "\u{B7}", "\u{B7}", "\u{B7}"]
            .map(str::to_owned)
            .to_vec(),
    )
}

#[test]
fn ascii_shows_a_wider_value_as_its_bytes_in_byte_order_with_no_separator() -> Result<(), String> {
    let value = u32::from_be_bytes(*b"abcd");
    let big = DataNodeContent::new(NodeValues::U32(vec![value]), DataFormat::Ascii);
    check_eq(cells(&big), vec!["abcd".to_owned()])?;
    let little =
        DataNodeContent::new(NodeValues::U32(vec![value]), DataFormat::Ascii).with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&little), vec!["dcba".to_owned()])
}

#[test]
fn ascii_widest_cell_has_the_same_character_count_as_every_other() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0x4142, 0x0020, 0x7A7A]), DataFormat::Ascii);
    let mut widest = String::new();
    content.widest_cell_string(&mut widest);
    for cell in cells(&content) {
        check_eq(cell.chars().count(), widest.chars().count())?;
    }
    Ok(())
}

#[test]
fn plain_text_is_one_cell_holding_the_whole_string_with_real_spaces() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(b"Hello, world".to_vec()), DataFormat::PlainText);
    check_eq(cells(&content), vec!["Hello, world".to_owned()])?;
    check_eq(content.len(), 1)?;
    check_eq(content.shape(), (1, 1))?;
    check_eq(content.type_name(), "text")
}

#[test]
fn plain_text_accepts_only_printable_ascii_bytes() -> Result<(), String> {
    let ok = DataNodeContent::new(NodeValues::U8(b" ~".to_vec()), DataFormat::PlainText);
    check_eq(ok.plain_text_is_valid(), true)?;
    for bad in [vec![0x1F], vec![0x7F], vec![0xFF]] {
        let content = DataNodeContent::new(NodeValues::U8(bad), DataFormat::PlainText);
        check_eq(content.plain_text_is_valid(), false)?;
    }
    let wide = DataNodeContent::new(NodeValues::U16(vec![0x4142]), DataFormat::PlainText);
    check_eq(wide.plain_text_is_valid(), false)
}

#[test]
fn empty_plain_text_has_no_cells() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(Vec::new()), DataFormat::PlainText);
    check_eq(content.len(), 0)?;
    check_eq(cells(&content), Vec::<String>::new())
}
