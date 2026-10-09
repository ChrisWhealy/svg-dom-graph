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
}

impl LabellingStyle {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the label of element `index` into `out`, replacing what it held.
    pub(crate) fn label_into(self, index: usize, out: &mut String) {
        use std::fmt::Write;
        out.clear();
        match self {
            Self::Numeric => {
                let _ = write!(out, "{index}");
            },
            Self::Alphabetic => write_alphabetic(index, out),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The label of element `index`.
    #[cfg(test)]
    pub(crate) fn label(self, index: usize) -> String {
        let mut out = String::new();
        self.label_into(index, &mut out);
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
