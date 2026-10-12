//! `LabellingStyle` and `DataNodeContent::with_labels`/`with_labelling_style`: the labels, and when labelling is on.

use super::super::*;
use super::support::check_eq;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   LabellingStyle / with_labels
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_default_labelling_style_is_numeric() -> Result<(), String> {
    check_eq(LabellingStyle::default(), LabellingStyle::Numeric)
}

#[test]
fn numeric_labels_are_the_index() -> Result<(), String> {
    check_eq(
        [0, 7, 10, 123].map(|i| LabellingStyle::Numeric.label(i)),
        ["0", "7", "10", "123"].map(String::from),
    )
}

#[test]
fn alphabetic_labels_of_the_first_eight_are_a_to_h() -> Result<(), String> {
    let labels: Vec<String> = (0..8).map(|i| LabellingStyle::Alphabetic.label(i)).collect();
    check_eq(labels.concat(), "abcdefgh".to_owned())
}

#[test]
fn alphabetic_labels_gain_a_character_after_z() -> Result<(), String> {
    check_eq(
        [25, 26, 27, 51, 52, 701, 702].map(|i| LabellingStyle::Alphabetic.label(i)),
        ["z", "aa", "ab", "az", "ba", "zz", "aaa"].map(String::from),
    )
}

#[test]
fn alphabetic_labels_are_unique() -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for i in 0..2000 {
        if !seen.insert(LabellingStyle::Alphabetic.label(i)) {
            return Err(format!("index {i} repeats an earlier label"));
        }
    }
    Ok(())
}

#[test]
fn text_labels_are_the_strings_given() -> Result<(), String> {
    let style = LabellingStyle::Text(&["ms[n-3]", "ms[n-8]"]);
    check_eq([0, 1, 2].map(|i| style.label(i)), ["ms[n-3]", "ms[n-8]", ""].map(String::from))
}

#[test]
fn a_text_label_belongs_to_a_row_not_an_element() -> Result<(), String> {
    // In a grid of five columns, elements 0 to 4 are row 0, and elements 5 to 9 are row 1.
    let style = LabellingStyle::Text(&["0-4", "5-9"]);
    let mut out = String::new();
    let labels = [0, 4, 5, 9, 10].map(|i| {
        style.label_into(i, 5, &mut out);
        out.clone()
    });
    check_eq(labels, ["0-4", "0-4", "5-9", "5-9", ""].map(String::from))
}

#[test]
fn the_widest_text_label_is_found_in_any_row() -> Result<(), String> {
    let style = LabellingStyle::Text(&["a", "ms[n-14]", "bb"]);
    let mut out = String::new();
    style.widest_label_into(3, 1, &mut out);
    check_eq(out.as_str(), "ms[n-14]")
}

#[test]
fn a_text_label_for_a_row_the_grid_does_not_have_is_not_a_candidate_for_the_widest() -> Result<(), String> {
    let style = LabellingStyle::Text(&["a", "bb", "a label for a third row"]);
    let mut out = String::new();
    style.widest_label_into(2, 3, &mut out);
    check_eq(out.as_str(), "bb")
}

#[test]
fn the_widest_numeric_label_is_the_last_rows() -> Result<(), String> {
    let mut out = String::new();
    LabellingStyle::Numeric.widest_label_into(11, 1, &mut out);
    check_eq(out.as_str(), "10")
}

#[test]
fn labelling_is_off_unless_asked_for() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal);
    check_eq(content.labelling(), None)
}

#[test]
fn with_labels_turns_labelling_on_in_the_default_style() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal).with_labels();
    check_eq(content.labelling(), Some(LabellingStyle::Numeric))
}

#[test]
fn with_labelling_style_turns_labelling_on_in_that_style() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal)
        .with_labelling_style(LabellingStyle::Alphabetic);
    check_eq(content.labelling(), Some(LabellingStyle::Alphabetic))
}

#[test]
fn a_single_value_has_nothing_to_label() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).with_labels();
    check_eq(content.labelling(), None)
}

#[test]
fn alphabetic_labels_never_overflow_at_the_largest_index() -> Result<(), String> {
    let label = LabellingStyle::Alphabetic.label(usize::MAX);
    check_eq(label.is_empty() || !label.chars().all(|c| c.is_ascii_lowercase()), false)
}

#[test]
fn alphabetic_label_into_reuses_the_buffer_it_is_given() -> Result<(), String> {
    let mut out = String::with_capacity(16);
    let before = out.as_ptr();
    for index in [0, 25, 26, 701, 702, 100_000] {
        LabellingStyle::Alphabetic.label_into(index, 1, &mut out);
    }
    check_eq(out.as_ptr(), before)?;
    check_eq(out.capacity(), 16)
}
