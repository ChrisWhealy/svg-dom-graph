//! The explanation under the diagram: that each step has one, and how it is formatted.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step explains itself in up to three paragraphs, and a merge says which side is which.
#[test]
fn each_step_explains_what_it_did_and_why() -> Result<(), String> {
    let tab = open()?;
    let counting = explanation(&tab)?;
    check(counting.contains("Each distinct byte is now a tree of one node"), &counting)?;
    check(counting.contains("rare bytes end up deepest"), &counting)?;

    click(&tab, "Next selection")?;
    let merge = explanation(&tab)?;
    check(merge.contains("The lighter goes left (0) and the other right (1)."), &merge)?;
    check(merge.ends_with("4 trees remain."), &merge)?;

    // The second merge joins 'b' and 'r', and the tree made by the first ties with 'r'.
    click(&tab, "Next selection")?;
    let tie = explanation(&tab)?;
    check(tie.contains("Another tree also weighs 2. Ties go to the tree made first"), &tie)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The explanation of a step is bold where it names the merge, and is not drawn with its Markdown markers showing.
#[test]
fn the_explanation_is_formatted_and_shows_no_markers() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next selection")?;
    let bold = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][1] \
                 .querySelectorAll('tspan[font-weight=bold]') ? [...[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][1] \
                 .querySelectorAll('tspan[font-weight=bold]')].map(t => t.textContent) : [])"
        ),
    )?;
    check_eq(bold.as_str(), "[\"Merge 1 of 4\"]")?;
    check(!explanation(&tab)?.contains("**"), "the markers of bold text are showing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A byte that Markdown reads as formatting is drawn as itself in the explanation, never as emphasis.
#[test]
fn a_byte_that_markdown_would_read_as_formatting_is_drawn_as_itself() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "a*b*c_d_e")?;
    // Counting, then merges until the star is named, which is the third of the lightest trees to meet.
    let mut seen = String::new();
    for position in 0..=8 {
        goto(&tab, position)?;
        seen.push_str(&explanation(&tab)?);
        seen.push('\n');
    }
    check(
        seen.contains("'*'") && seen.contains("'_'"),
        &format!("the bytes are not named as themselves: {seen}"),
    )?;
    check(!seen.contains('\\'), &format!("a backslash of an escape is showing: {seen}"))
}
