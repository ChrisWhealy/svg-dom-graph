//! `NodeValues::Text` and `DataFormat::Text`: each string as it is written, what makes text content valid, what a
//! screen reader is told for an empty cell, what a replacement may change, and the cell-width hint.

use super::super::*;
use super::support::{cells, check_eq};
use crate::test_support::check;

fn text(values: &[&str]) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::Text(values.iter().map(ToString::to_string).collect()),
        DataFormat::Text,
    )
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_string_is_written_as_it_is() -> Result<(), String> {
    let values = [
        "0",
        "\u{221E}",
        "\u{2014}",
        "A",
        "",
        "two  spaces",
        "\u{4E16}\u{754C}",
        "<b>&amp;</b>",
    ];
    check_eq(cells(&text(&values)), strings(&values))
}

#[test]
fn text_is_one_cell_for_each_string_and_is_laid_out_as_a_grid() -> Result<(), String> {
    let content = text(&["A", "B", "C", "D", "E", "F"]);
    check_eq(content.len(), 6)?;
    check_eq(content.shape(), (2, 3))?;
    check(!content.is_single_value(), "six strings are not a single value")?;
    check(text(&["only"]).is_single_value(), "one string is a single value")
}

#[test]
fn the_type_is_named_and_has_a_colour_of_its_own() -> Result<(), String> {
    let values = NodeValues::Text(strings(&["a"]));
    check_eq(values.type_name(), "text")?;
    let others = [
        NodeValues::U8(vec![0]),
        NodeValues::I64(vec![0]),
        NodeValues::F32(vec![0.0]),
        NodeValues::Bool(vec![true]),
        NodeValues::Uuid(vec![crate::scene::Uuid::from_u128(0)]),
    ];
    for other in others {
        check(
            other.type_colour() != values.type_colour(),
            &format!("{} shares the colour of text", other.type_name()),
        )?;
    }
    check(
        values.type_colour() != crate::colours::PLAIN_BOX_FILL,
        "text should not look like an ordinary node",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn text_values_and_the_text_format_go_together() -> Result<(), String> {
    check(text(&["a"]).text_is_valid(), "text values in the text format")?;
    check(
        !DataNodeContent::new(NodeValues::Text(strings(&["a"])), DataFormat::Decimal).text_is_valid(),
        "text values need the text format",
    )?;
    check(
        !DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Text).text_is_valid(),
        "the text format needs text values",
    )?;
    check(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).text_is_valid(),
        "content with no text is not checked",
    )
}

#[test]
fn a_string_with_a_control_character_is_invalid() -> Result<(), String> {
    for bad in ["a\nb", "a\rb", "tab\there", "nul\u{0}", "bell\u{7}", "\u{85}"] {
        check(!text(&["ok", bad]).text_is_valid(), &format!("{bad:?} should be refused"))?;
    }
    check(
        text(&["", " ", "\u{221E}", "\u{2014}"]).text_is_valid(),
        "empty, a space and symbols are fine",
    )
}

#[test]
fn a_hint_is_held_to_the_same_rule_as_a_cell() -> Result<(), String> {
    check(text(&["a"]).with_cell_width_hint("9999999999").text_is_valid(), "a plain hint")?;
    check(
        !text(&["a"]).with_cell_width_hint("9\n9").text_is_valid(),
        "a hint with a line break",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn an_empty_cell_is_read_out_as_empty_and_every_other_as_itself() -> Result<(), String> {
    let content = text(&["", "A"]);
    check_eq(content.spoken(""), "empty")?;
    check_eq(content.spoken("A"), "A")?;
    check_eq(content.spoken("\u{221E}"), "\u{221E}")
}

#[test]
fn text_is_drawn_in_the_ordinary_text_colour_and_keeps_its_spaces() -> Result<(), String> {
    let content = text(&["a  b"]);
    check_eq(content.ink("a  b"), crate::colours::TEXT_FILL)?;
    check(content.preserves_spaces(), "a run of spaces is drawn as it is")?;
    check(!content.keeps_spaces(), "a text node is still a grid, whose layout is checked")?;
    check(
        !DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).preserves_spaces(),
        "numbers collapse",
    )
}

#[test]
fn the_widest_string_to_start_from_is_the_one_with_the_most_characters() -> Result<(), String> {
    let mut out = String::from("stale");
    text(&["ab", "abcd", "abc"]).widest_cell_string(&mut out);
    check_eq(out.as_str(), "abcd")?;
    text(&[]).widest_cell_string(&mut out);
    check_eq(out.as_str(), "")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What replacing `content`'s values with `values` did: whether it was accepted, and every cell it rewrote.
fn replace(content: &mut DataNodeContent, values: &[&str]) -> (bool, Vec<(usize, String)>) {
    let mut written = Vec::new();
    let mut scratch = String::new();
    let accepted = content
        .try_replace_values(NodeValues::Text(strings(values)), &mut scratch, |i, text| -> Result<(), ()> {
            written.push((i, text.to_owned()));
            Ok(())
        })
        .expect("the callback never fails");
    (accepted, written)
}

#[test]
fn a_replacement_rewrites_only_the_strings_that_changed() -> Result<(), String> {
    let mut content = text(&["0", "\u{221E}", "\u{221E}"]);
    let (accepted, written) = replace(&mut content, &["0", "3", "\u{221E}"]);
    check(accepted, "same count, same kind")?;
    check_eq(written, vec![(1, "3".to_owned())])?;
    check_eq(cells(&content), strings(&["0", "3", "\u{221E}"]))
}

#[test]
fn a_replacement_that_changes_nothing_writes_nothing() -> Result<(), String> {
    let mut content = text(&["a", "b"]);
    let (accepted, written) = replace(&mut content, &["a", "b"]);
    check(accepted, "an identical replacement is accepted")?;
    check(written.is_empty(), "and writes nothing")
}

#[test]
fn a_replacement_may_be_empty_or_wider_than_the_original() -> Result<(), String> {
    let mut content = text(&["\u{221E}", "x"]);
    let (accepted, written) = replace(&mut content, &["1250", ""]);
    check(accepted, "any one-line string")?;
    check_eq(written, vec![(0, "1250".to_owned()), (1, String::new())])
}

#[test]
fn a_replacement_with_another_count_or_a_control_character_is_refused() -> Result<(), String> {
    let mut content = text(&["a", "b"]);
    let (accepted, written) = replace(&mut content, &["a"]);
    check(!accepted && written.is_empty(), "another count")?;
    let (accepted, written) = replace(&mut content, &["a", "b\nc"]);
    check(!accepted && written.is_empty(), "a line break")?;
    check_eq(cells(&content), strings(&["a", "b"]))
}

#[test]
fn text_cannot_replace_numbers_or_the_other_way_round() -> Result<(), String> {
    let mut numbers = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal);
    let mut scratch = String::new();
    let accepted = numbers
        .try_replace_values(NodeValues::Text(strings(&["1", "2"])), &mut scratch, |_, _| Ok::<(), ()>(()))
        .expect("never fails");
    check(!accepted, "text for numbers")?;
    let mut words = text(&["1", "2"]);
    let accepted = words
        .try_replace_values(NodeValues::U8(vec![1, 2]), &mut scratch, |_, _| Ok::<(), ()>(()))
        .expect("never fails");
    check(!accepted, "numbers for text")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_hint_adds_no_value_and_changes_no_cell() -> Result<(), String> {
    let plain = text(&["A", "B"]);
    let hinted = text(&["A", "B"]).with_cell_width_hint("9999999999");
    check_eq(cells(&hinted), cells(&plain))?;
    check_eq(hinted.len(), plain.len())?;
    check_eq(hinted.cell_width_hint(), Some("9999999999"))?;
    check_eq(plain.cell_width_hint(), None)?;
    check(hinted != plain, "a hint is part of what a node is drawn from")
}

#[test]
fn text_cells_are_those_of_text_content_only() -> Result<(), String> {
    check_eq(text(&["a", "b"]).text_cells(), Some(strings(&["a", "b"]).as_slice()))?;
    check_eq(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).text_cells(),
        None,
    )
}
