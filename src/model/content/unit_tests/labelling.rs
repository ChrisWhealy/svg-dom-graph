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
