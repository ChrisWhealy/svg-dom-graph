//! `DataNodeContent`'s own basic properties: the default and overridden layout, the value count, whether it holds a
//! single value, and its type colour and name.

use super::super::*;
use super::support::{cells, check_eq};

#[test]
fn new_defaults_to_automatic_layout() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal);
    check_eq(content.layout(), GridLayout::Automatic)?;
    // Automatic's own 2x4 choice for 8 values — see grid_shape_of_eight_values_prefers_two_rows_of_four above.
    check_eq(content.shape(), (2, 4))
}

#[test]
fn with_layout_overrides_automatics_own_shape() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal)
        .with_layout(GridLayout::MaxColumns(3));
    check_eq(content.layout(), GridLayout::MaxColumns(3))?;
    check_eq(content.shape(), (3, 3))
}

#[test]
fn len_reports_the_value_count_regardless_of_width() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![1, 2, 3]), DataFormat::Decimal);
    check_eq(content.len(), 3)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   is_single_value / type_colour
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_single_value_is_reported_as_such() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    check_eq(content.is_single_value(), true)
}

#[test]
fn two_values_are_not_reported_as_a_single_value() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal);
    check_eq(content.is_single_value(), false)
}

#[test]
fn every_width_gets_a_distinct_type_colour() -> Result<(), String> {
    let colours = [
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Decimal).type_colour(),
    ];
    let mut unique = colours.to_vec();
    unique.sort_unstable();
    unique.dedup();
    check_eq(unique.len(), colours.len())
}

#[test]
fn type_name_matches_each_widths_own_rust_type() -> Result<(), String> {
    check_eq(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).type_name(),
        "u8",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal).type_name(),
        "u16",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal).type_name(),
        "u32",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Decimal).type_name(),
        "u64",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replaces `content`'s values with `values`, and returns whether it was accepted and each cell reported as changed.
fn replace(content: &mut DataNodeContent, values: NodeValues) -> (bool, Vec<(usize, String)>) {
    let mut changed = Vec::new();
    let mut scratch = String::new();
    let accepted = content
        .try_replace_values(values, &mut scratch, |i, text| -> Result<(), ()> {
            changed.push((i, text.to_owned()));
            Ok(())
        })
        .unwrap_or(false);
    (accepted, changed)
}

#[test]
fn replacing_values_reports_only_the_cells_that_changed() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    let (accepted, changed) = replace(&mut content, NodeValues::U8(vec![1, 9, 3, 7]));
    check_eq(accepted, true)?;
    check_eq(changed, vec![(1, "9".to_owned()), (3, "7".to_owned())])
}

#[test]
fn replacing_values_with_the_same_ones_reports_nothing() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U16(vec![10, 20, 30]), DataFormat::Hexadecimal);
    let (accepted, changed) = replace(&mut content, NodeValues::U16(vec![10, 20, 30]));
    check_eq(accepted, true)?;
    check_eq(changed, Vec::new())
}

#[test]
fn replaced_values_are_formatted_in_the_contents_own_format() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U8(vec![0, 0]), DataFormat::Hexadecimal);
    let (_, changed) = replace(&mut content, NodeValues::U8(vec![0, 255]));
    check_eq(changed, vec![(1, "FF".to_owned())])?;
    check_eq(cells(&content), vec!["00".to_owned(), "FF".to_owned()])
}

#[test]
fn an_incompatible_replacement_changes_and_reports_nothing() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal);
    for wrong in [NodeValues::U16(vec![5, 6]), NodeValues::U8(vec![5, 6, 7])] {
        let (accepted, changed) = replace(&mut content, wrong);
        check_eq(accepted, false)?;
        check_eq(changed, Vec::new())?;
    }
    check_eq(cells(&content), vec!["1".to_owned(), "2".to_owned()])
}

#[test]
fn replaced_plain_text_is_reported_whole_or_not_at_all() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U8(b"abc".to_vec()), DataFormat::PlainText);
    let (_, same) = replace(&mut content, NodeValues::U8(b"abc".to_vec()));
    check_eq(same, Vec::new())?;
    let (_, different) = replace(&mut content, NodeValues::U8(b"abd".to_vec()));
    check_eq(different, vec![(0, "abd".to_owned())])
}

#[test]
fn a_failed_update_keeps_the_old_values_so_a_retry_sees_every_cell_again() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    let mut scratch = String::new();
    let mut written = Vec::new();
    let failed = content.try_replace_values(NodeValues::U8(vec![9, 2, 8, 7]), &mut scratch, |i, text| {
        if i == 2 {
            return Err("write failed");
        }
        written.push(i);
        let _ = text;
        Ok(())
    });
    check_eq(failed, Err("write failed"))?;
    check_eq(written, vec![0])?;
    // The model still holds the old values, so none of the three changes is lost.
    check_eq(
        cells(&content),
        vec!["1".to_owned(), "2".to_owned(), "3".to_owned(), "4".to_owned()],
    )?;
    let (accepted, changed) = replace(&mut content, NodeValues::U8(vec![9, 2, 8, 7]));
    check_eq(accepted, true)?;
    check_eq(changed.iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![0, 2, 3])
}
