use super::{Block, Span, Style};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How wide a character is and how big the body text is, as measured in the browser.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Metrics {
    /// The width of one character of body text, which is monospace.
    pub(crate) char_width: f64,
    /// The font size of body text.
    pub(crate) font_size: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One line of text, placed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    /// The top of the line, from the top of the content.
    pub(crate) top: f64,
    /// The height of the line.
    pub(crate) height: f64,
    /// The font size of the text on it.
    pub(crate) size: f64,
    /// How far the text starts from the left edge of the content.
    pub(crate) indent: f64,
    /// A bullet or number drawn to the left of the first line of a list item, in the space `indent` leaves.
    pub(crate) marker: Option<String>,
    pub(crate) spans: Vec<Span>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every line of a document, placed, and the height they take.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub(crate) lines: Vec<Line>,
    /// The bottom of the last line.
    pub(crate) height: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Line height as a multiple of font size.
const LINE_HEIGHT: f64 = 1.5;
/// The font size of each heading level, as a multiple of the body size.
const HEADING_SCALE: [f64; 3] = [1.5, 1.25, 1.1];
/// The gap between blocks, as a multiple of the body size, and the extra gap above a heading, as a multiple of its own.
const BLOCK_GAP: f64 = 0.6;
const HEADING_GAP: f64 = 0.5;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A word: the pieces of one or more runs with no whitespace between them. `**bo**ld` is one word of two pieces.
type Word = Vec<Span>;

/// The whitespace before a word.
///
/// Between words of prose it is one space, however much whitespace there was. Inside a code span it is exactly what was
/// written, since a gap in code is information: `A  B` is not `A B`. Either is dropped where the line breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Gap {
    text: String,
    /// Whether any of it was written inside a code span.
    code: bool,
}

impl Gap {
    fn width(&self) -> usize {
        self.text.chars().count()
    }
}

/// A word, and the gap before it. The first word of a text has none.
struct Item {
    gap: Option<Gap>,
    word: Word,
}

/// The whitespace seen since the last word, until the next word says what it comes to.
#[derive(Default)]
struct PendingGap {
    text: String,
    code: bool,
    /// Whether the last of it was a prose space, so that more prose whitespace next to it adds nothing.
    ends_in_prose_space: bool,
}

impl PendingGap {
    fn add(&mut self, whitespace: &str, in_code: bool) {
        if in_code {
            // A tab is drawn as one space, so that every character takes one column.
            self.text.extend(whitespace.chars().map(|c| if c == '\t' { ' ' } else { c }));
            self.code = true;
            self.ends_in_prose_space = false;
        } else if !self.ends_in_prose_space {
            self.text.push(' ');
            self.ends_in_prose_space = true;
        }
    }
}

/// Splits `spans` into words, each with the whitespace before it. Whitespace before the first word, and after the last,
/// is dropped.
fn words(spans: &[Span]) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let mut gap = PendingGap::default();
    let mut in_word = false;
    for span in spans {
        for (is_space, piece) in split_runs(&span.text) {
            if is_space {
                gap.add(piece, span.style.code);
                in_word = false;
                continue;
            }
            if !in_word {
                let pending = std::mem::take(&mut gap);
                let before = (!items.is_empty() && !pending.text.is_empty()).then_some(Gap {
                    text: pending.text,
                    code: pending.code,
                });
                items.push(Item { gap: before, word: Vec::new() });
                in_word = true;
            }
            if let Some(item) = items.last_mut() {
                item.word.push(Span {
                    text: piece.to_owned(),
                    style: span.style,
                });
            }
        }
    }
    items
}

/// `text` as alternating runs of whitespace and non-whitespace, each flagged with whether it is whitespace.
fn split_runs(text: &str) -> Vec<(bool, &str)> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut current: Option<bool> = None;
    for (i, c) in text.char_indices() {
        let is_space = c.is_whitespace();
        match current {
            Some(was) if was != is_space => {
                runs.push((was, &text[start..i]));
                start = i;
                current = Some(is_space);
            },
            None => current = Some(is_space),
            _ => {},
        }
    }
    if let Some(was) = current {
        runs.push((was, &text[start..]));
    }
    runs
}

fn word_len(word: &Word) -> usize {
    word.iter().map(|s| s.text.chars().count()).sum()
}

/// Appends `text` drawn in `style`, joining it to the last run if that is drawn the same way.
fn push_run(line: &mut Vec<Span>, text: &str, style: Style) {
    match line.last_mut() {
        Some(last) if last.style == style => last.text.push_str(text),
        _ => line.push(Span { text: text.to_owned(), style }),
    }
}

/// Appends `word` to `line`.
fn push_word(line: &mut Vec<Span>, word: &Word) {
    for piece in word {
        push_run(line, &piece.text, piece.style);
    }
}

/// Appends `gap` to `line`, between a word that ends in `before` and one that starts in `after`. The gap is drawn in
/// the style the two share, or plain if they share none. A gap that was written in code is drawn as code too, so that
/// its spaces belong to the code.
fn push_gap(line: &mut Vec<Span>, gap: &Gap, before: Style, after: Style) {
    let shared = if before == after { before } else { Style::default() };
    let style = Style {
        code: gap.code && before.code && after.code,
        ..shared
    };
    push_run(line, &gap.text, style);
}

/// Breaks `word`, which is longer than `cols`, into pieces of at most `cols` characters each.
fn split_word(word: &Word, cols: usize) -> Vec<Word> {
    let mut pieces: Vec<Word> = vec![Vec::new()];
    let mut used = 0;
    for span in word {
        let mut rest = span.text.as_str();
        while !rest.is_empty() {
            if used == cols {
                pieces.push(Vec::new());
                used = 0;
            }
            let take = rest.chars().take(cols - used).map(char::len_utf8).sum::<usize>();
            if let Some(piece) = pieces.last_mut() {
                piece.push(Span {
                    text: rest[..take].to_owned(),
                    style: span.style,
                });
            }
            used += rest[..take].chars().count();
            rest = &rest[take..];
        }
    }
    pieces
}

/// Wraps `spans` into lines of at most `cols` characters, breaking between words. A word longer than a whole line is
/// split.
///
/// The gap between two words is one space in prose, however much whitespace was written, and exactly what was written
/// inside a code span. Its width counts toward the line. Where a line breaks, the gap there is dropped, and so is any
/// whitespace at the start or the end of a line. A `cols` of 0 is treated as 1.
pub(crate) fn wrap(spans: &[Span], cols: usize) -> Vec<Vec<Span>> {
    let cols = cols.max(1);
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    let mut used = 0;

    for Item { gap, word } in words(spans) {
        let pieces = if word_len(&word) > cols { split_word(&word, cols) } else { vec![word] };
        // Only a word's first piece has the gap before it. Each piece after is a full line and starts a new one.
        let mut gap = gap;
        for piece in pieces {
            let len = word_len(&piece);
            let gap_width = gap.as_ref().map_or(0, Gap::width);
            if used > 0 && used + gap_width + len > cols {
                lines.push(std::mem::take(&mut line));
                used = 0;
            }
            if let (true, Some(gap)) = (used > 0, gap.take()) {
                let before = line.last().map(|s| s.style).unwrap_or_default();
                let after = piece.first().map(|s| s.style).unwrap_or_default();
                push_gap(&mut line, &gap, before, after);
                used += gap.width();
            }
            gap = None;
            push_word(&mut line, &piece);
            used += len;
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The marker of item `index` of a list that starts at `start`, or a bullet if it is not numbered.
fn list_marker(start: Option<u32>, index: usize) -> String {
    start.map_or_else(|| "•".to_owned(), |n| format!("{}.", u64::from(n) + index as u64))
}

/// The marker, and a space after it, are the hanging indent every line of the item keeps.
fn hanging_indent(marker: &str, metrics: Metrics) -> f64 {
    (marker.chars().count() + 1) as f64 * metrics.char_width
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The narrowest column `blocks` fit in without a list marker running into its own text: the widest hanging indent of
/// any list item, plus room for one character of text. It is 0 if there is no list.
pub(crate) fn minimum_width(blocks: &[Block], metrics: Metrics) -> f64 {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::List { start, items } => (0..items.len())
                .map(|i| hanging_indent(&list_marker(*start, i), metrics) + metrics.char_width)
                .reduce(f64::max),
            _ => None,
        })
        .fold(0.0, f64::max)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Places `blocks` in a column `width` wide. `width` is the width of the text itself, with no padding.
pub(crate) fn layout(blocks: &[Block], width: f64, metrics: Metrics) -> Layout {
    let mut lines: Vec<Line> = Vec::new();
    let mut top = 0.0;
    let body = metrics.font_size;

    // Adds the wrapped lines of one block. `gap_before` is the space above the first, ignored at the very top.
    let mut add = |size: f64, gap_before: f64, indent: f64, marker: Option<String>, spans: &[Span], bold: bool| {
        if !lines.is_empty() {
            top += gap_before;
        }
        let char_width = metrics.char_width * size / body;
        let cols = ((width - indent) / char_width).floor().max(1.0) as usize;
        let mut first = true;
        for mut wrapped in wrap(spans, cols) {
            if bold {
                wrapped.iter_mut().for_each(|s| s.style.bold = true);
            }
            let height = size * LINE_HEIGHT;
            lines.push(Line {
                top,
                height,
                size,
                indent,
                marker: if first { marker.clone() } else { None },
                spans: wrapped,
            });
            top += height;
            first = false;
        }
    };

    for block in blocks {
        match block {
            Block::Heading { level, spans } => {
                let size = body * HEADING_SCALE[usize::from(level.clamp(&1, &3) - 1)];
                add(size, body * BLOCK_GAP + size * HEADING_GAP, 0.0, None, spans, true);
            },
            Block::Paragraph { lines: paragraph } => {
                for (i, spans) in paragraph.iter().enumerate() {
                    // A hard break starts a new line with no extra space.
                    add(body, if i == 0 { body * BLOCK_GAP } else { 0.0 }, 0.0, None, spans, false);
                }
            },
            Block::List { start, items } => {
                for (i, item) in items.iter().enumerate() {
                    let marker = list_marker(*start, i);
                    let indent = hanging_indent(&marker, metrics);
                    add(
                        body,
                        if i == 0 { body * BLOCK_GAP } else { 0.0 },
                        indent,
                        Some(marker),
                        item,
                        false,
                    );
                }
            },
        }
    }

    Layout { lines, height: top }
}
