//! Editing the text: the walk starts again, and text that cannot be encoded leaves the diagram alone.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Editing the text starts the walk again, from the new text.
#[test]
fn editing_the_text_starts_the_walk_again_from_the_new_text() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 7)?;

    set_text(&tab, "aab")?;
    check_eq(position(&tab)?, Some(0))?;
    check_eq(status(&tab)?, "3 bytes, 2 distinct, 1 merges.".to_owned())?;
    check_eq(forest(&tab)?, (2, 0))?;
    check_eq(table(&tab)?, vec![row("a", "2", "–", "–", "–"), row("b", "1", "–", "–", "–")])?;

    // 1 counting step, 1 merge, 1 lengths step, 2 codes.
    goto(&tab, 4)?;
    check_eq(table(&tab)?, vec![row("a", "2", "1", "1", "0"), row("b", "1", "0", "1", "1")])?;
    check_eq(position(&tab)?, Some(4))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text that cannot be encoded says why, and leaves the diagram as it was.
#[test]
fn text_that_cannot_be_encoded_leaves_the_diagram_alone() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    let before = (explanation(&tab)?, table(&tab)?);

    set_text(&tab, "")?;
    check(
        status(&tab)?.starts_with("Nothing to encode."),
        &format!("status {:?}", status(&tab)?),
    )?;
    check_eq((explanation(&tab)?, table(&tab)?), before.clone())?;

    set_text(&tab, &"x".repeat(129))?;
    check(
        status(&tab)?.starts_with("129 bytes is too long. The limit is 128."),
        &format!("status {:?}", status(&tab)?),
    )?;
    check_eq((explanation(&tab)?, table(&tab)?), before)?;
    check_eq(position(&tab)?, Some(2))?;
    // The box still holds what was typed, so that it can be corrected.
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.len(),
        129,
    )?;

    // The limit is in bytes: 65 two-byte characters are 130 bytes.
    set_text(&tab, &"é".repeat(65))?;
    check(
        status(&tab)?.starts_with("130 bytes is too long. The limit is 128."),
        &format!("status {:?}", status(&tab)?),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A character of several bytes is counted as its separate bytes, each shown in hexadecimal.
#[test]
fn a_multi_byte_character_is_counted_as_its_separate_bytes() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "é")?;
    check_eq(status(&tab)?, "2 bytes, 2 distinct, 1 merges.".to_owned())?;
    check_eq(
        table(&tab)?,
        vec![row("0xA9", "1", "–", "–", "–"), row("0xC3", "1", "–", "–", "–")],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One distinct byte has nothing to merge, and is given a 1-bit code.
#[test]
fn one_distinct_byte_is_given_a_one_bit_code() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "aaaa")?;
    check_eq(status(&tab)?, "4 bytes, 1 distinct, 0 merges.".to_owned())?;
    goto(&tab, 2)?;
    check_eq(table(&tab)?, vec![row("a", "4", "(root)", "1", "0")])?;
    // The route to the only byte is that byte's own box.
    check_eq(marked(&tab)?, vec!["a x4".to_owned()])
}
