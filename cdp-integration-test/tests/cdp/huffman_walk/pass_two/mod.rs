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

mod unit_tests;
