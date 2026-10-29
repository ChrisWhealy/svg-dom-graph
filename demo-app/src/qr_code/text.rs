//! Text in the scene. SVG does not wrap text, so a block is a fixed number of lines, and each line is a `<tspan>` that
//! is written when the text changes. The number of elements never depends on the text.

use super::slot::{Slot, Target, make};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Splits `text` into lines of at most `width` characters, breaking only between words. A word longer than a line is
/// kept whole on a line of its own.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            },
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a block of text goes, and how much of it there is room for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockLayout {
    pub(crate) x: f64,
    /// The baseline of the first line.
    pub(crate) y: f64,
    pub(crate) line_height: f64,
    pub(crate) max_lines: usize,
    /// The widest a line may be, in characters.
    pub(crate) width: usize,
    pub(crate) bold: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A block of wrapped text with room for `max_lines` lines.
pub(crate) struct TextBlock {
    lines: Vec<Slot>,
    width: usize,
}

impl TextBlock {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds the block to `layer`, empty.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be made or added.
    pub(crate) fn new(
        document: &web_sys::Document,
        layer: &web_sys::Element,
        layout: BlockLayout,
    ) -> Result<Self, String> {
        let text = make(
            document,
            "text",
            &[
                ("x", &layout.x.to_string()),
                ("y", &layout.y.to_string()),
                ("font-size", "14"),
                ("font-weight", if layout.bold { "bold" } else { "normal" }),
                ("fill", "#1a1a1a"),
                ("style", "user-select: none;"),
            ],
        )?;
        let mut lines = Vec::with_capacity(layout.max_lines);
        for index in 0..layout.max_lines {
            let dy = if index == 0 { 0.0 } else { layout.line_height };
            let line = make(document, "tspan", &[("x", &layout.x.to_string()), ("dy", &dy.to_string())])?;
            text.append_child(&line).map_err(|e| format!("could not add a line: {e:?}"))?;
            lines.push(Slot::new(&line, Target::Text));
        }
        layer
            .append_child(&text)
            .map_err(|e| format!("could not add a text block: {e:?}"))?;
        Ok(Self { lines, width: layout.width })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes `text` into the block, and returns how many lines changed. Lines the text does not reach are emptied.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the text needs more lines than the block has, rather than cutting it short, or if the DOM
    /// refuses a write.
    pub(crate) fn set(&self, text: &str) -> Result<usize, String> {
        let wrapped = wrap(text, self.width);
        if wrapped.len() > self.lines.len() {
            return Err(format!(
                "the text needs {} lines and the block has {}",
                wrapped.len(),
                self.lines.len()
            ));
        }
        let mut writes = 0;
        for (index, slot) in self.lines.iter().enumerate() {
            writes += usize::from(slot.set(wrapped.get(index).map_or("", String::as_str))?);
        }
        Ok(writes)
    }
}
