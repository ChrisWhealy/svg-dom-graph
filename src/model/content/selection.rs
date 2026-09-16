// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which of a [`super::DataNodeContent`]'s own cells [`Scene::set_selection`](crate::scene::Scene::set_selection)
/// should highlight. Addressed either as one flat index, or, for a multi-row/column grid, as a whole row/column
/// plus an optional further cell within it.
///
/// A one-dimensional grid (a single row or a single column) only ever needs [`Cell`](Self::Cell) — there is no
/// separate "row" to highlight distinctly from the one element within it.
///
/// A two-dimensional grid can additionally highlight a whole [`Row`](Self::Row)/[`Column`](Self::Column) — the "we
/// are now processing this row/column" step of a data-flow walk. It can also highlight a further cell within it —
/// the "and specifically this element" step — rendered in a stronger colour that overrides the row/column's own.
///
/// `#[non_exhaustive]`: a plausible future addition — highlighting more than one row, say — should stay additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Selection {
    /// No selection: every cell renders in its own default `NodeValues::type_color`.
    #[default]
    None,
    /// Highlights the value at 0-based flat index `0`, addressing the whole content as one flat sequence regardless
    /// of how [`GridLayout`](super::GridLayout) arranges it into rows and columns.
    ///
    /// Renders in the same, stronger colour a [`Row`](Self::Row)/[`Column`](Self::Column)'s own optional cell gets —
    /// both name "this is the one specific element," not "this is the active group."
    Cell(usize),
    /// Highlights every cell in 0-based row `row`, and additionally the cell at column `col` within that row, in a
    /// stronger colour, if `col` is `Some`.
    Row { row: usize, col: Option<usize> },
    /// Highlights every cell in 0-based column `col`, and additionally the cell at row `row` within that column, in
    /// a stronger colour, if `row` is `Some`.
    Column { col: usize, row: Option<usize> },
}

impl Selection {
    /// A short, human-readable description of this selection, suitable for appending to a node's own
    /// `aria-label`.
    ///
    /// Empty for [`Selection::None`], so the label reads exactly as it did before any selection was made.
    /// `Scene::set_selection` uses this so the current selection is exposed as text, not only as colour.
    ///
    /// Colour alone conveys nothing to assistive technology or a colour-blind reader — the same reasoning
    /// [`super::NodeValues::type_color`]'s own `<title>`/`aria-label` pairing already follows.
    pub(crate) fn describe(self) -> String {
        match self {
            Self::None => String::new(),
            Self::Cell(i) => format!(", cell {i} selected"),
            Self::Row { row, col: None } => format!(", row {row} selected"),
            Self::Row { row, col: Some(col) } => format!(", row {row} selected, column {col} focused"),
            Self::Column { col, row: None } => format!(", column {col} selected"),
            Self::Column { col, row: Some(row) } => format!(", column {col} selected, row {row} focused"),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A [`Selection::Row`]/[`Selection::Column`] already resolved against its own content's actual grid shape, in a form
/// that [`Scene::set_selection`](crate::scene::Scene::set_selection) can test cheaply, one flat cell at a time.
///
/// Deliberately holds no list of member indices. A grid can hold arbitrarily many cells, and a "previous"/"next"
/// control re-tests every one of them on every step. [`contains`](Self::contains) instead tests membership
/// arithmetically, in `O(1)` per cell, with no allocation at all — a plain `row`/`col` and the grid's own `cols` are
/// enough to decide it.
///
/// [`contains`](Self::contains) trusts its own caller to only ever query a flat index that names a real cell.
/// [`super::DataNodeContent::resolve_selection`] is the only place that builds one.
///
/// `Scene::set_selection` is the only reader, and it walks exactly `content.len()` real cells, never a nominal
/// row/column position past them. So this never needs to re-check a query against the content's own value count itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedBand {
    /// No band: [`contains`](Self::contains) is `false` for every index.
    None,
    /// Every flat index `i` with `i / cols == row` belongs to this band.
    Row { row: usize, cols: usize },
    /// Every flat index `i` with `i % cols == col` belongs to this band.
    Column { col: usize, cols: usize },
}

impl ResolvedBand {
    /// Whether flat index `i` belongs to this band.
    pub(crate) fn contains(self, i: usize) -> bool {
        match self {
            Self::None => false,
            Self::Row { row, cols } => i / cols == row,
            Self::Column { col, cols } => i % cols == col,
        }
    }
}
