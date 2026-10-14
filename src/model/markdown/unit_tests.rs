//! The Markdown subset: what each construct parses to, what is left literal, and how text wraps and stacks.

use super::{layout::wrap, *};

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

const PLAIN: Style = Style {
    bold: false,
    italic: false,
    code: false,
};
const BOLD: Style = Style { bold: true, ..PLAIN };
const ITALIC: Style = Style { italic: true, ..PLAIN };
const CODE: Style = Style { code: true, ..PLAIN };

fn span(text: &str, style: Style) -> Span {
    Span { text: text.to_owned(), style }
}

/// `text` as `(text, style)` pairs, which read better in a failure than `Span`s do.
fn runs(text: &str) -> Vec<(String, Style)> {
    parse_inline(text).into_iter().map(|s| (s.text, s.style)).collect()
}

fn run(text: &str, style: Style) -> (String, Style) {
    (text.to_owned(), style)
}

/// The text of every line `wrap` makes of `text` at `cols` columns.
fn wrapped(text: &str, cols: usize) -> Vec<String> {
    wrap(&parse_inline(text), cols)
        .into_iter()
        .map(|line| line.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Inline
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn plain_text_is_one_plain_run() -> Result<(), String> {
    check_eq(runs("just some words"), vec![run("just some words", PLAIN)])?;
    check_eq(runs(""), vec![])
}

#[test]
fn bold_italic_and_both_are_recognised_in_either_marker() -> Result<(), String> {
    check_eq(runs("a **b** c"), vec![run("a ", PLAIN), run("b", BOLD), run(" c", PLAIN)])?;
    check_eq(runs("a __b__ c"), vec![run("a ", PLAIN), run("b", BOLD), run(" c", PLAIN)])?;
    check_eq(runs("a *b* c"), vec![run("a ", PLAIN), run("b", ITALIC), run(" c", PLAIN)])?;
    check_eq(runs("a _b_ c"), vec![run("a ", PLAIN), run("b", ITALIC), run(" c", PLAIN)])?;
    check_eq(
        runs("***both***"),
        vec![run(
            "both",
            Style {
                bold: true,
                italic: true,
                code: false,
            },
        )],
    )
}

#[test]
fn inline_code_keeps_its_markers_literal() -> Result<(), String> {
    check_eq(
        runs("use `a * b` here"),
        vec![run("use ", PLAIN), run("a * b", CODE), run(" here", PLAIN)],
    )?;
    // Two backticks let the code hold one.
    check_eq(runs("``a ` b``"), vec![run("a ` b", CODE)])?;
    // One space of padding either side is dropped, so code can start or end with a backtick.
    check_eq(runs("`` `x` ``"), vec![run("`x`", CODE)])
}

#[test]
fn emphasis_nests_when_the_markers_differ_in_length() -> Result<(), String> {
    let both = Style {
        bold: true,
        italic: true,
        code: false,
    };
    check_eq(
        runs("**bold with *italic* inside**"),
        vec![run("bold with ", BOLD), run("italic", both), run(" inside", BOLD)],
    )?;
    check_eq(
        runs("*italic with **bold** inside*"),
        vec![run("italic with ", ITALIC), run("bold", both), run(" inside", ITALIC)],
    )
}

#[test]
fn a_marker_with_no_closer_is_literal() -> Result<(), String> {
    check_eq(runs("**not closed"), vec![run("**not closed", PLAIN)])?;
    check_eq(runs("a * b"), vec![run("a * b", PLAIN)])?;
    // Both stars are surrounded by spaces, so neither opens nor closes.
    check_eq(runs("2 * 3 * 4"), vec![run("2 * 3 * 4", PLAIN)])?;
    check_eq(runs("`open"), vec![run("`open", PLAIN)])?;
    check_eq(runs("****"), vec![run("****", PLAIN)])
}

#[test]
fn emphasis_markers_inside_inline_code_do_not_close_the_emphasis_around_it() -> Result<(), String> {
    let both = Style {
        bold: true,
        italic: false,
        code: true,
    };
    // The `**` inside the code is code, so the bold run goes on past it to the real closer.
    check_eq(
        runs("**before `**` after**"),
        vec![run("before ", BOLD), run("**", both), run(" after", BOLD)],
    )?;
    let italic_code = Style {
        italic: true,
        code: true,
        bold: false,
    };
    check_eq(
        runs("*a `*` b*"),
        vec![run("a ", ITALIC), run("*", italic_code), run(" b", ITALIC)],
    )?;
    check_eq(
        runs("_a `_` b_"),
        vec![run("a ", ITALIC), run("_", italic_code), run(" b", ITALIC)],
    )?;
    // Two backticks hold a single one, and the markers inside are still code.
    check_eq(
        runs("**x ``a ` ** b`` y**"),
        vec![run("x ", BOLD), run("a ` ** b", both), run(" y", BOLD)],
    )
}

#[test]
fn a_backtick_with_no_closer_does_not_hide_the_emphasis_closer_after_it() -> Result<(), String> {
    // The lone backtick is literal, so the `**` after it still closes the bold run.
    check_eq(runs("**a ` b**"), vec![run("a ` b", BOLD)])
}

#[test]
fn an_underscore_inside_a_word_is_left_alone() -> Result<(), String> {
    check_eq(runs("snake_case_name"), vec![run("snake_case_name", PLAIN)])?;
    check_eq(runs("(_word_)"), vec![run("(", PLAIN), run("word", ITALIC), run(")", PLAIN)])
}

#[test]
fn a_backslash_makes_the_next_punctuation_literal() -> Result<(), String> {
    check_eq(runs(r"\*not italic\*"), vec![run("*not italic*", PLAIN)])?;
    check_eq(runs(r"a \# b"), vec![run("a # b", PLAIN)])?;
    check_eq(runs(r"back\\slash"), vec![run(r"back\slash", PLAIN)])?;
    // Before anything that is not punctuation, a backslash is only a backslash.
    check_eq(runs(r"\q"), vec![run(r"\q", PLAIN)])
}

#[test]
fn a_closer_after_whitespace_does_not_close() -> Result<(), String> {
    check_eq(runs("*a *b"), vec![run("*a *b", PLAIN)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Blocks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_blank_line_ends_a_paragraph_and_its_lines_join_with_a_space() -> Result<(), String> {
    let blocks = parse("one\ntwo\n\nthree");
    check_eq(
        blocks,
        vec![
            Block::Paragraph {
                lines: vec![vec![span("one two", PLAIN)]],
            },
            Block::Paragraph {
                lines: vec![vec![span("three", PLAIN)]],
            },
        ],
    )
}

#[test]
fn two_trailing_spaces_or_a_backslash_is_a_hard_break() -> Result<(), String> {
    for source in ["one  \ntwo", "one\\\ntwo"] {
        check_eq(
            parse(source),
            vec![Block::Paragraph {
                lines: vec![vec![span("one", PLAIN)], vec![span("two", PLAIN)]],
            }],
        )?;
    }
    Ok(())
}

#[test]
fn headings_have_a_level_of_one_to_three() -> Result<(), String> {
    let level = |source: &str| match parse(source).as_slice() {
        [Block::Heading { level, .. }] => Some(*level),
        _ => None,
    };
    check_eq(level("# One"), Some(1))?;
    check_eq(level("## Two"), Some(2))?;
    check_eq(level("### Three"), Some(3))?;
    // Deeper headings are drawn as the deepest there is.
    check_eq(level("###### Six"), Some(3))?;
    // A hash with no space after it is text.
    check_eq(level("#hashtag"), None)
}

#[test]
fn a_heading_drops_its_closing_hashes_and_keeps_its_inline_styles() -> Result<(), String> {
    check_eq(
        parse("## A *big* one ##"),
        vec![Block::Heading {
            level: 2,
            spans: vec![span("A ", PLAIN), span("big", ITALIC), span(" one", PLAIN)],
        }],
    )
}

#[test]
fn a_bulleted_list_takes_any_of_three_markers() -> Result<(), String> {
    check_eq(
        parse("- a\n* b\n+ c"),
        vec![Block::List {
            start: None,
            items: vec![vec![span("a", PLAIN)], vec![span("b", PLAIN)], vec![span("c", PLAIN)]],
        }],
    )
}

#[test]
fn a_numbered_list_counts_up_from_its_first_number() -> Result<(), String> {
    check_eq(
        parse("3. c\n4) d"),
        vec![Block::List {
            start: Some(3),
            items: vec![vec![span("c", PLAIN)], vec![span("d", PLAIN)]],
        }],
    )?;
    check_eq(plain_text(&parse("3. c\n4) d")), "3. c\n4. d".to_owned())
}

#[test]
fn an_indented_line_carries_on_the_list_item_before_it() -> Result<(), String> {
    check_eq(
        parse("- one\n  and more\n- two"),
        vec![Block::List {
            start: None,
            items: vec![vec![span("one and more", PLAIN)], vec![span("two", PLAIN)]],
        }],
    )
}

#[test]
fn a_list_can_interrupt_a_paragraph_and_the_other_kind_of_list_starts_a_new_one() -> Result<(), String> {
    let blocks = parse("intro\n- a\n1. b");
    check_eq(blocks.len(), 3)?;
    check(matches!(blocks[0], Block::Paragraph { .. }), "the intro is a paragraph")?;
    check(matches!(blocks[1], Block::List { start: None, .. }), "a bulleted list")?;
    check(matches!(blocks[2], Block::List { start: Some(1), .. }), "then a numbered one")
}

#[test]
fn a_star_that_opens_emphasis_is_not_a_bullet() -> Result<(), String> {
    check_eq(
        parse("**bold** start"),
        vec![Block::Paragraph {
            lines: vec![vec![span("bold", BOLD), span(" start", PLAIN)]],
        }],
    )
}

#[test]
fn windows_line_endings_and_blank_input_are_handled() -> Result<(), String> {
    check_eq(parse("a\r\n\r\nb").len(), 2)?;
    check_eq(parse(""), vec![])?;
    check_eq(parse("   \n\n  "), vec![])
}

#[test]
fn plain_text_has_no_markers_and_one_line_per_heading_line_or_item() -> Result<(), String> {
    let blocks = parse("# Title\n\nSome **bold** text.\n\n- one\n- two");
    check_eq(plain_text(&blocks), "Title\nSome bold text.\n• one\n• two".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Wrapping
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn text_wraps_between_words_and_no_line_is_longer_than_the_columns() -> Result<(), String> {
    let lines = wrapped("the quick brown fox jumps over the lazy dog", 16);
    check_eq(
        lines,
        ["the quick brown", "fox jumps over", "the lazy dog"]
            .map(str::to_owned)
            .to_vec(),
    )?;
    for line in wrapped("the quick brown fox jumps over the lazy dog", 11) {
        check(line.chars().count() <= 11, &format!("{line:?} is too long"))?;
    }
    Ok(())
}

#[test]
fn a_word_longer_than_a_line_is_split() -> Result<(), String> {
    check_eq(wrapped("abcdefghij", 4), ["abcd", "efgh", "ij"].map(str::to_owned).to_vec())?;
    check_eq(wrapped("ab abcdefgh", 4), ["ab", "abcd", "efgh"].map(str::to_owned).to_vec())
}

#[test]
fn a_word_made_of_several_styled_runs_is_not_broken_between_them() -> Result<(), String> {
    // `**bo**ld` is one word, so it moves to the next line whole.
    check_eq(wrapped("aaaa **bo**ld", 8), ["aaaa", "bold"].map(str::to_owned).to_vec())
}

#[test]
fn wrapping_keeps_each_runs_style() -> Result<(), String> {
    let lines = wrap(&parse_inline("plain **bold** end"), 80);
    check_eq(lines.len(), 1)?;
    check_eq(
        lines[0].clone(),
        vec![span("plain ", PLAIN), span("bold", BOLD), span(" end", PLAIN)],
    )
}

#[test]
fn zero_columns_and_empty_text_do_not_loop_or_panic() -> Result<(), String> {
    check_eq(wrapped("ab", 0), ["a", "b"].map(str::to_owned).to_vec())?;
    check_eq(wrapped("", 10), vec![String::new()])?;
    check_eq(wrapped("   ", 10), vec![String::new()])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const METRICS: Metrics = Metrics {
    char_width: 8.0,
    font_size: 14.0,
};

#[test]
fn lines_stack_from_the_top_and_the_height_is_the_bottom_of_the_last() -> Result<(), String> {
    let laid = layout(&parse("one\n\ntwo"), 400.0, METRICS);
    check_eq(laid.lines.len(), 2)?;
    check_eq(laid.lines[0].top, 0.0)?;
    check(
        laid.lines[1].top > laid.lines[0].top + laid.lines[0].height,
        "a gap separates paragraphs",
    )?;
    let last = &laid.lines[1];
    check_eq(laid.height, last.top + last.height)
}

#[test]
fn a_hard_break_adds_no_gap() -> Result<(), String> {
    let laid = layout(&parse("one  \ntwo"), 400.0, METRICS);
    check_eq(laid.lines[1].top, laid.lines[0].top + laid.lines[0].height)
}

#[test]
fn a_heading_is_larger_and_bold() -> Result<(), String> {
    let laid = layout(&parse("# Big\n\nsmall"), 400.0, METRICS);
    check(laid.lines[0].size > laid.lines[1].size, "the heading is larger")?;
    check(laid.lines[0].spans.iter().all(|s| s.style.bold), "the heading is bold")?;
    check_eq(laid.lines[1].size, 14.0)
}

#[test]
fn wrapping_uses_a_narrower_character_count_for_a_larger_heading() -> Result<(), String> {
    // At 14 pixels a character is 8 wide, so 80 pixels is 10 columns. A level 1 heading is 1.5 times as large, so its
    // characters are 12 wide, and 80 pixels is 6 columns.
    let body = layout(&parse("aaaa bbbb cccc"), 80.0, METRICS);
    let heading = layout(&parse("# aaaa bbbb cccc"), 80.0, METRICS);
    check_eq(body.lines.len(), 2)?;
    check_eq(heading.lines.len(), 3)
}

#[test]
fn a_list_item_hangs_its_continuation_lines_under_its_text() -> Result<(), String> {
    let laid = layout(&parse("- aaaa bbbb cccc dddd"), 80.0, METRICS);
    check(laid.lines.len() > 1, "the item wraps")?;
    check_eq(laid.lines[0].marker.clone(), Some("•".to_owned()))?;
    check_eq(laid.lines[1].marker.clone(), None)?;
    // The marker and a space are two columns of 8 pixels.
    check_eq(laid.lines[0].indent, 16.0)?;
    check_eq(laid.lines[1].indent, 16.0)?;
    // The text has 80 - 16 = 64 pixels, which is 8 columns.
    check(
        laid.lines
            .iter()
            .all(|l| l.spans.iter().map(|s| s.text.chars().count()).sum::<usize>() <= 8),
        "within 8 columns",
    )
}

#[test]
fn numbered_items_are_numbered_and_the_indent_follows_the_marker_width() -> Result<(), String> {
    let laid = layout(&parse("9. a\n10. b"), 400.0, METRICS);
    check_eq(laid.lines[0].marker.clone(), Some("9.".to_owned()))?;
    check_eq(laid.lines[1].marker.clone(), Some("10.".to_owned()))?;
    check_eq(laid.lines[0].indent, 24.0)?;
    check_eq(laid.lines[1].indent, 32.0)
}

#[test]
fn an_empty_document_has_no_lines_and_no_height() -> Result<(), String> {
    let laid = layout(&parse(""), 400.0, METRICS);
    check_eq((laid.lines.len(), laid.height), (0, 0.0))
}

#[test]
fn a_width_too_narrow_for_one_character_still_makes_progress() -> Result<(), String> {
    let laid = layout(&parse("abc"), 1.0, METRICS);
    check_eq(laid.lines.len(), 3)
}
