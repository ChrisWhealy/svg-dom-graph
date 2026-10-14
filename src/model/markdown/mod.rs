//! A small subset of Markdown, parsed and laid out without any DOM, for a node that explains something in prose.
//!
//! This is deliberately not a CommonMark implementation, and this crate does not depend on one. A Markdown crate such
//! as `comrak` is a general parser that produces HTML or a syntax tree. This crate needs neither. It draws SVG `<text>`
//! it lays out itself, in a monospace font, so the output must be lines of styled runs and not markup. The subset
//! below is all a node needs, and every unsupported construct simply reads as the characters it was typed with.
//!
//! # What is supported
//!
//! Blocks:
//!
//! - A **paragraph** is one or more lines. A blank line ends it, and the lines inside it join with a single space. A
//!   line that ends in two spaces or a backslash is a hard break, which starts a new line inside the same paragraph.
//! - A **heading** is one to three `#` followed by a space. Four or more `#` are drawn as a level 3 heading.
//! - A **list** is a run of lines that start with `-`, `*` or `+` and a space, or with a number, a `.` or `)`, and a
//!   space. A line indented by two spaces or more after an item continues that item, even if it starts with a marker.
//!   So a nested list is drawn as text in its parent's item. A numbered list counts up from its first number.
//!
//! Inline:
//!
//! - `**bold**` and `__bold__`, `*italic*` and `_italic_`, and `***both***`.
//! - `` `code` `` and ``` ``code with a ` in it`` ```.
//! - A backslash before any ASCII punctuation makes that character literal.
//!
//! An emphasis marker with no matching closing one is literal text. Emphasis nests inside emphasis only when the
//! markers differ in length: `**bold with *italic* inside**` works, and `**bold *italic***` does not. An underscore
//! only opens or closes emphasis at the edge of a word, so `snake_case_name` is left alone.
//!
//! # What is not supported
//!
//! Links and images, block quotes, code blocks, tables, rules, HTML, and nested lists. They are drawn literally.
//!
//! # Layout
//!
//! [`layout()`] wraps every line at a width given in user-space units, using one measured character width. Under a
//! monospace font a bold or italic run is as wide as a regular one, so every run's width is its character count times
//! that one number. A character is counted as one column, whatever it is, so text that is wider than a column, such as
//! some East Asian characters, can overrun its line.
//!
//! Between two words of prose, however much whitespace was written, one space is drawn. Whitespace inside a code span
//! is kept exactly as written, since a gap in code is information, and its width counts toward the line. A tab in code
//! is drawn as one space. Where a line breaks, the whitespace there is dropped, as it is at either end of a line.

mod inline;
mod layout;

pub(crate) use inline::parse_inline;
pub(crate) use layout::{Layout, Line, Metrics, layout};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a run of text is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    /// Inline code, drawn in a different colour.
    pub(crate) code: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A run of text that is all drawn the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) style: Style,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One block of a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    /// A heading, at level 1 to 3. Always drawn bold.
    Heading { level: u8, spans: Vec<Span> },
    /// A paragraph, as the lines the author broke it into with hard breaks. Each line wraps on its own.
    Paragraph { lines: Vec<Vec<Span>> },
    /// A list. `start` is the number of the first item of a numbered list, and `None` for a bulleted one.
    List { start: Option<u32>, items: Vec<Vec<Span>> },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What kind of list a line of text starts, with the text after its marker.
struct Marker<'a> {
    start: Option<u32>,
    rest: &'a str,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `line` as a list item, if it starts with a list marker followed by a space.
fn list_marker(line: &str) -> Option<Marker<'_>> {
    let line = line.trim_start();
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(bullet) {
            return Some(Marker { start: None, rest });
        }
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    // Nine digits at most, so that the number always fits a `u32`.
    if (1..=9).contains(&digits) {
        let rest = &line[digits..];
        let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
        let rest = rest.strip_prefix(' ')?;
        return Some(Marker {
            start: line[..digits].parse().ok(),
            rest,
        });
    }
    None
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many spaces of indent make a line carry on the list item above it. A marker indented by less is a sibling item.
const CONTINUATION_INDENT: usize = 2;

/// The indent of `line`, in spaces. A tab counts as four.
fn indent(line: &str) -> usize {
    line.chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// The level and text of `line` if it is a heading.
fn heading(line: &str) -> Option<(u8, &str)> {
    let line = line.trim_start();
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    // A closing run of `#` is decoration.
    let text = rest.trim().trim_end_matches('#').trim_end();
    Some((hashes.min(3) as u8, text))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the parser has gathered since the last block ended.
#[derive(Default)]
struct Pending {
    /// The lines of a paragraph, as the pieces between hard breaks. The last one is still being added to.
    paragraph: Vec<String>,
    /// A numbered list's first number, or `None` for a bulleted one, and the text of every item so far.
    list: Option<(Option<u32>, Vec<String>)>,
}

impl Pending {
    fn flush(&mut self, blocks: &mut Vec<Block>) {
        if !self.paragraph.is_empty() {
            let lines = std::mem::take(&mut self.paragraph);
            blocks.push(Block::Paragraph {
                lines: lines.iter().map(|l| parse_inline(l)).collect(),
            });
        }
        if let Some((start, items)) = self.list.take() {
            blocks.push(Block::List {
                start,
                items: items.iter().map(|i| parse_inline(i)).collect(),
            });
        }
    }

    /// Adds `text` to the paragraph, joining it to the piece before it with a space unless that piece ended in a break.
    fn add_to_paragraph(&mut self, text: &str, hard_break: bool, after_break: &mut bool) {
        match self.paragraph.last_mut() {
            Some(last) if !*after_break => {
                last.push(' ');
                last.push_str(text);
            },
            _ => self.paragraph.push(text.to_owned()),
        }
        *after_break = hard_break;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses `source` into blocks. Never fails: whatever is not understood is kept as the text it was written as.
pub(crate) fn parse(source: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut pending = Pending::default();
    let mut after_break = false;

    for raw in source.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let line = raw.trim();

        if line.is_empty() {
            pending.flush(&mut blocks);
            after_break = false;
        } else if pending.list.is_some() && indent(raw) >= CONTINUATION_INDENT {
            // An indented line carries on the item before it, whatever it starts with. This is decided before a
            // marker or a `#` is looked for, so an indented marker is text and not a nested list, which is not
            // supported.
            if let Some((_, items)) = &mut pending.list {
                if let Some(last) = items.last_mut() {
                    last.push(' ');
                    last.push_str(line);
                }
            }
        } else if let Some((level, text)) = heading(raw) {
            pending.flush(&mut blocks);
            blocks.push(Block::Heading {
                level,
                spans: parse_inline(text),
            });
            after_break = false;
        } else if let Some(marker) = list_marker(raw) {
            // A list may interrupt a paragraph, and a different kind of list starts a new one.
            if pending
                .list
                .as_ref()
                .is_some_and(|(start, _)| start.is_some() != marker.start.is_some())
            {
                pending.flush(&mut blocks);
            }
            if !pending.paragraph.is_empty() {
                pending.flush(&mut blocks);
            }
            let (_, items) = pending.list.get_or_insert_with(|| (marker.start, Vec::new()));
            items.push(marker.rest.trim().to_owned());
        } else if raw.starts_with([' ', '\t']) && pending.list.is_some() {
            // A line indented by a single space that is not a marker carries on the item too.
            if let Some((_, items)) = &mut pending.list {
                if let Some(last) = items.last_mut() {
                    last.push(' ');
                    last.push_str(line);
                }
            }
        } else {
            if pending.list.is_some() {
                pending.flush(&mut blocks);
            }
            let hard_break = raw.ends_with("  ") || raw.ends_with('\\');
            let text = line.strip_suffix('\\').map_or(line, str::trim_end);
            pending.add_to_paragraph(text, hard_break, &mut after_break);
        }
    }
    pending.flush(&mut blocks);
    blocks
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text of `spans`, without any styling.
fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `blocks` as plain text: one line for each heading, paragraph line and list item, with a `•` or a number before
/// each list item. This is what assistive technology and a copy of the node's own text should read.
pub(crate) fn plain_text(blocks: &[Block]) -> String {
    let mut lines = Vec::new();
    for block in blocks {
        match block {
            Block::Heading { spans, .. } => lines.push(spans_text(spans)),
            Block::Paragraph { lines: paragraph } => lines.extend(paragraph.iter().map(|l| spans_text(l))),
            Block::List { start, items } => {
                for (i, item) in items.iter().enumerate() {
                    let marker = start.map_or_else(|| "•".to_owned(), |n| format!("{}.", u64::from(n) + i as u64));
                    lines.push(format!("{marker} {}", spans_text(item)));
                }
            },
        }
    }
    lines.join("\n")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
