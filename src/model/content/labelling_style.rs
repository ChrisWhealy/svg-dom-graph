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
            Self::Alphabetic => {
                // Fills the digits least significant first. One more than the index makes it a number counted from 1.
                let mut rest = index + 1;
                while rest > 0 {
                    rest -= 1;
                    out.push(char::from(b'a' + (rest % 26) as u8));
                    rest /= 26;
                }
                let reversed: String = out.chars().rev().collect();
                *out = reversed;
            },
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
