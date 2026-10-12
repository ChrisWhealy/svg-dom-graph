// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`super::DataNodeContent::with_labelling_style`] names a grid's own elements. A label is drawn outside the cells,
/// in the grid box's own left padding, beside the first cell of each row.
///
/// `#[non_exhaustive]`, for the same reason as [`super::DataFormat`]. A plausible future addition — `Roman`, ... —
/// should stay additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LabellingStyle {
    /// The element's own index: `0`, `1`, `2`, and so on. The default, used by
    /// [`with_labels`](super::DataNodeContent::with_labels).
    #[default]
    Numeric,
    /// Lowercase letters, using as many characters as the index needs: `a` to `z`, then `aa`, `ab`, and so on, as a
    /// spreadsheet names its columns. Element `0` is `a`. Element `25` is `z`. Element `26` is `aa`.
    ///
    /// This is base 26 with no zero digit, so `a` never acts as a leading zero. Without that, `a` would be both `0` and
    /// the empty prefix of `aa`, and two elements could share a label.
    Alphabetic,
    /// Text chosen by the caller: element `i` is labelled with the `i`th string. An element past the end of the list
    /// has an empty label. Meant for a grid whose elements have names of their own, such as the operands of a formula.
    ///
    /// The list is a `'static` slice, so that this style stays [`Copy`] like the others. Consequently, a label that
    /// needs to be built at run time cannot be used here.
    Text(&'static [&'static str]),
}

impl LabellingStyle {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the label of element `index` into `out`, replacing what it held.
    pub(crate) fn label_into(self, index: usize, cols: usize, out: &mut String) {
        out.clear();
        self.append_label(index, cols, out);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Appends the label of element `index` to `out`, keeping what it held. Allocates only if `out` must grow.
    pub(crate) fn append_label(self, index: usize, cols: usize, out: &mut String) {
        use std::fmt::Write;
        match self {
            Self::Numeric => {
                let _ = write!(out, "{index}");
            },
            Self::Alphabetic => write_alphabetic(index, out),
            Self::Text(labels) => out.push_str(labels.get(index / cols.max(1)).copied().unwrap_or("")),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the longest label a grid of `rows` rows of `cols` columns draws into `out`, replacing what it held. A
    /// label is drawn beside the first element of each row. A numeric or alphabetic label never gets shorter as its
    /// index grows, so the last row's own is the longest. A text label has no such order, so every row's own is
    /// compared, and the first of the longest wins.
    pub(crate) fn widest_label_into(self, rows: usize, cols: usize, out: &mut String) {
        match self {
            Self::Text(labels) => {
                let widest = labels.iter().take(rows).fold("", |widest, label| {
                    if label.chars().count() > widest.chars().count() { label } else { widest }
                });
                out.clear();
                out.push_str(widest);
            },
            _ => self.label_into(rows.saturating_sub(1) * cols, cols, out),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The label of element `index`.
    #[cfg(test)]
    pub(crate) fn label(self, index: usize) -> String {
        let mut out = String::new();
        self.label_into(index, 1, &mut out);
        out
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Appends the spreadsheet-style letters for `index` to `out`, most significant first: `a` to `z`, then `aa`, `ab`, and
/// so on. The leading letters are written before the last one, so nothing is built backwards and then reversed, and
/// `out` allocates only if it must grow. The recursion is at most 14 calls deep for a 64-bit `usize`. Counting from `0`
/// with `- 1` on the leading part avoids `index + 1`, which could overflow.
fn write_alphabetic(index: usize, out: &mut String) {
    if index >= 26 {
        write_alphabetic(index / 26 - 1, out);
    }
    out.push(char::from(b'a' + (index % 26) as u8));
}
