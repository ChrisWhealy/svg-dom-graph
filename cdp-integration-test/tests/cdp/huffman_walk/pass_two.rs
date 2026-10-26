//! Pass two: packing the input into bytes with the canonical codes.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pass two
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// "abracadabra" has the canonical codes a=0 b=100 c=101 d=110 r=111, so it packs to the bytes 0x4E, 0xAC and 0x9C with
// a bit of padding. The 'a' that is the fourth input byte completes the first byte.

/// The texts that follow the box titled `heading`, for as long as `keep` accepts them.
fn after(tab: &Tab, heading: &str, keep: impl Fn(&str) -> bool) -> Result<Vec<String>, String> {
    let all = texts(tab)?;
    let start = all
        .iter()
        .position(|t| t == heading)
        .ok_or_else(|| format!("there is no {heading:?} box"))?;
    Ok(all[start + 1..].iter().take_while(|t| keep(t)).cloned().collect())
}

fn is_hex_byte(t: &str) -> bool {
    t.len() == 2 && t.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

/// The bytes written so far, as the two hex digits the hexadecimal node shows for each, in lower case. Before the
/// first, the scene says so in a plain box instead.
fn output(tab: &Tab) -> Result<Vec<String>, String> {
    if texts(tab)?.iter().any(|t| t == "Output bytes: none") {
        return Ok(Vec::new());
    }
    Ok(after(tab, "Output bytes", is_hex_byte)?
        .iter()
        .map(|b| b.to_lowercase())
        .collect())
}

/// What the accessible name of the node called `name` says is highlighted, such as `byte 0`, or `None` if nothing is.
fn highlight(tab: &Tab, name: &str) -> Result<Option<String>, String> {
    let label = text(
        tab,
        &format!(
            "([...document.querySelectorAll('{DIAGRAM} g[aria-label]')] \
             .find(g => g.getAttribute('aria-label').startsWith('{name}'))?.getAttribute('aria-label') ?? '')"
        ),
    )?;
    Ok(label.split_once("also highlighted: ").map(|(_, cell)| cell.to_owned()))
}

/// Whether the scene's title names pass two.
fn in_pass_two(tab: &Tab) -> Result<bool, String> {
    let all = texts(tab)?;
    Ok(all.iter().any(|t| t.contains("Phase 2")))
}

/// How many text input nodes the scene has.
fn text_inputs(tab: &Tab) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{DIAGRAM} input').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

/// The first position of pass two.
const PASS_TWO: usize = LAST + 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pass two opens with its own title, and the text box becomes an array of the input's bytes with the first marked.
#[test]
fn pass_two_replaces_the_text_box_with_the_input_bytes_and_marks_the_first() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    check(!in_pass_two(&tab)?, "pass one has the pass one title")?;
    check_eq(text_inputs(&tab)?, 1)?;

    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(PASS_TWO))?;
    check(in_pass_two(&tab)?, "pass two has the pass two title")?;
    check_eq(text_inputs(&tab)?, 0)?;
    check(
        explanation(&tab)?.starts_with("Byte 1 of 11: 'a' has the code 0"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, Vec::<String>::new())?;
    // The table is complete, and its row for 'a' is the one ringed.
    check_eq(table(&tab)?.remove(0), row("a", "5", "0", "1", "0"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping across the boundary, in both directions, visits every position once, and the scene follows.
#[test]
fn stepping_across_the_boundary_skips_and_repeats_no_position() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST - 1)?;
    for expected in LAST..=PASS_TWO + 1 {
        click(&tab, "Next selection")?;
        check_eq(position(&tab)?, Some(expected))?;
        check_eq(in_pass_two(&tab)?, expected >= PASS_TWO)?;
    }
    for expected in (LAST - 1..=PASS_TWO).rev() {
        click(&tab, "Previous selection")?;
        check_eq(position(&tab)?, Some(expected))?;
        check_eq(in_pass_two(&tab)?, expected >= PASS_TWO)?;
        check_eq(text_inputs(&tab)?, usize::from(expected < PASS_TWO))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step writes a byte only when eight bits are ready, and marks exactly the bytes it wrote.
#[test]
fn a_step_marks_exactly_the_bytes_it_wrote() -> Result<(), String> {
    let tab = open()?;
    // The first three input bytes leave 7 bits pending, and the fourth completes 0100 1110, which is 4e.
    goto(&tab, PASS_TWO + 2)?;
    check_eq(output(&tab)?, Vec::<String>::new())?;
    goto(&tab, PASS_TWO + 3)?;
    check_eq(output(&tab)?, vec!["4e".to_owned()])?;
    check_eq(highlight(&tab, "Output bytes")?, Some("cell 0".to_owned()))?;
    // The next step writes nothing, so the byte is no longer marked.
    goto(&tab, PASS_TWO + 4)?;
    check_eq(output(&tab)?, vec!["4e".to_owned()])?;
    check_eq(highlight(&tab, "Output bytes")?, None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The flush pads the last byte with zeros, compares its size with the input's.
#[test]
fn the_flush_pads_the_last_byte_and_shows_the_result_and_its_size() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, FLUSH)?;
    check(
        explanation(&tab)?.starts_with("Flush: 7 pending bits cannot fill a byte, so 1 zero bit pad it out: 10011100."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, ["4e", "ac", "9c"].map(str::to_owned).to_vec())?;

    let summary = markdown(&tab, 2)?;
    check(summary.contains("Original: 11 bytes (88 bits)"), &summary)?;
    check(
        summary.contains("Encoded: 3 bytes, which is 23 bits of code and 1 of padding"),
        &summary,
    )?;
    check(summary.contains("3.67 to 1"), &summary)?;
    check(summary.contains("27.3%"), &summary)?;

    // Next does nothing at the end: the walk clamps.
    button(&tab, "Next selection")?.click().map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(position(&tab)?, Some(FLUSH))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text whose codes fill whole bytes needs no padding, and the flush writes no byte of its own.
#[test]
fn a_byte_aligned_output_needs_no_padding() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "aaaaaaaa")?;
    // 3 positions of pass one, 8 of packing, and the flush.
    goto(&tab, 11)?;
    check(
        explanation(&tab)?.contains("no padding is needed"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, vec!["00".to_owned()])?;
    check(
        markdown(&tab, 2)?.contains("which is 8 bits of code and 0 of padding"),
        "the summary counts no padding",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every byte of a multi-byte character is its own cell of the input dump.
#[test]
fn the_input_dump_has_a_cell_for_each_byte_not_each_character() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "é")?;
    // 5 positions of pass one: counting, 1 merge, the lengths and 2 codes.
    goto(&tab, 5)?;
    // The hex dump tells the two bytes apart, even though neither is printable on its own.
    let lines = after(&tab, "Input bytes", |t| t.contains('|'))?;
    check(
        lines.iter().any(|l| l.to_lowercase().contains("c3 a9")),
        &format!("the dump does not show c3 a9: {lines:?}"),
    )?;
    check(
        explanation(&tab)?.starts_with("Byte 1 of 2: '0xC3'"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A one-byte text still shows its whole summary, inside the diagram.
#[test]
fn a_short_text_still_shows_its_summary_inside_the_diagram() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "a")?;
    // 3 positions of pass one, 1 of packing, and the flush.
    goto(&tab, 4)?;
    let summary = markdown(&tab, 2)?;
    check(summary.contains("Original: 1 byte (8 bits)"), &summary)?;
    let inside = text(
        &tab,
        &format!(
            "(() => {{ const d = document.querySelector('{DIAGRAM}').getBoundingClientRect(); \
             const t = [...document.querySelectorAll('{DIAGRAM} text')].find(t => t.textContent.startsWith('Compression')); \
             if (!t) return 'missing'; const r = t.getBoundingClientRect(); \
             return r.bottom <= d.bottom && r.right <= d.right ? 'inside' : 'outside'; }})()"
        ),
    )?;
    check_eq(inside.as_str(), "inside")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The keyboard focus stays on the "Next selection" button as the walk crosses from pass one into pass two.
#[test]
fn keyboard_focus_survives_the_change_of_pass() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    button(&tab, "Next selection")?.focus().map_err(|e| format!("{e}"))?;
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(PASS_TWO))?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? ''")?,
        "Next selection".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Next 8" and "Previous 8" move eight positions, across the change of pass, and stop at the ends.
#[test]
fn the_eight_step_buttons_move_eight_positions_across_both_passes() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next 8")?;
    check_eq(position(&tab)?, Some(8))?;
    click(&tab, "Next 8")?;
    check_eq(position(&tab)?, Some(16))?;
    check(in_pass_two(&tab)?, "position 16 is in pass two")?;
    click(&tab, "Previous 8")?;
    check_eq(position(&tab)?, Some(8))?;
    check(!in_pass_two(&tab)?, "position 8 is in pass one")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A dump without offsets marks the input byte a step packs over the glyphs it names, in every place of a line and on
/// more than one line. Identical lines are never collapsed, so a line is always where the byte's index says it is.
#[test]
fn the_input_dump_marks_the_byte_a_step_packs_over_its_own_glyphs() -> Result<(), String> {
    use crate::common::{ascii_column, dump_marks, hex_column};
    // 48 input bytes make three lines of the dump, and pack one at a time, so every place in a line is marked.
    let tab = open_default()?;
    set_text(&tab, &"abcdefgh".repeat(6))?;
    let scope = format!("{DIAGRAM} g[aria-label^=\"Input bytes\"]");
    let (mut hex_places, mut ascii_places, mut lines) = (
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    );
    // Walk to the end, looking at the marks after every step of pass two.
    for _ in 0..200 {
        click(&tab, "Next selection")?;
        if !in_pass_two(&tab)? {
            continue;
        }
        for mark in dump_marks(&tab, &scope)? {
            check(mark.over_its_glyphs, &format!("{mark:?} is not over its own glyphs"))?;
            lines.insert(mark.line);
            if mark.text.len() == 2 {
                let place = (0..16).find(|&p| hex_column(p, false) == mark.column);
                hex_places.insert(place.ok_or(format!("{mark:?} is not on a byte's hex digits"))?);
            } else {
                let place = (0..16).find(|&p| ascii_column(p, false) == mark.column);
                ascii_places.insert(place.ok_or(format!("{mark:?} is not on a byte's character"))?);
            }
        }
        // The flush is the last step, and the toolbar cannot step past it.
        if explanation(&tab)?.starts_with("Flush") {
            break;
        }
    }
    // Every place in a line is written at some step, including both sides of the gap, and more than one line.
    check_eq(hex_places.len(), 16)?;
    check_eq(ascii_places.len(), 16)?;
    check(lines.len() >= 2, &format!("only lines {lines:?} were marked"))
}
