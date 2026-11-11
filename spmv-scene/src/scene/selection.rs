//! A node that is selected: a column, which feeds rows, or a row, which is fed by columns.

/// A node that is selected: a column, which feeds rows, or a row, which is fed by columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Column(usize),
    Row(usize),
}

impl Selection {
    /// The tag a node carries, which is what an event is traced back to a node by: `x7` for column 7 and `y3` for row
    /// 3.
    #[must_use]
    pub fn tag(self) -> String {
        match self {
            Self::Column(j) => format!("x{j}"),
            Self::Row(i) => format!("y{i}"),
        }
    }

    /// The selection that a tag names, if it names one.
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let (kind, number) = tag.split_at_checked(1)?;
        let n: usize = number.parse().ok()?;
        match kind {
            "x" => Some(Self::Column(n)),
            "y" => Some(Self::Row(n)),
            _ => None,
        }
    }
}
