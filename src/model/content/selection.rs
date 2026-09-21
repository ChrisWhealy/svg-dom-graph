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
    /// Appends a short, human-readable description of this selection to `out`, suitable for a node's own
    /// `aria-label`.
    ///
    /// Writes nothing for [`Selection::None`], so the label reads exactly as it did before any selection was made.
    /// `Scene::set_selection` calls this into its own reused label buffer, first truncated back to the node's base
    /// description, then appends the current selection not only as a colour, but also as text, without allocating a
    /// fresh `String` on every call.
    ///
    /// Colour alone conveys nothing to assistive technology or a colour-blind reader — the same reasoning
    /// [`super::NodeValues::type_color`]'s own `<title>`/`aria-label` pairing already follows.
    pub(crate) fn describe_into(self, out: &mut String) {
        use std::fmt::Write as _;
        // `String`'s own `Write` impl only ever fails on allocation, which panics rather than returning `Err` —
        // the same reasoning `crate::geometry::elbow_path_into`'s own `write!` calls rely on.
        let _ = match self {
            Self::None => return,
            Self::Cell(i) => write!(out, ", cell {i} selected"),
            Self::Row { row, col: None } => write!(out, ", row {row} selected"),
            Self::Row { row, col: Some(col) } => write!(out, ", row {row} selected, column {col} focused"),
            Self::Column { col, row: None } => write!(out, ", column {col} selected"),
            Self::Column { col, row: Some(row) } => write!(out, ", column {col} selected, row {row} focused"),
        };
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A [`Selection::Row`]/[`Selection::Column`] already resolved against its own content's actual grid shape, in a form
/// [`Scene::set_selection`](crate::scene::Scene::set_selection) can walk and test cheaply.
///
/// Deliberately holds no list of member indices. A grid can hold arbitrarily many cells, so
/// [`for_each_index`](Self::for_each_index) walks a band's own members directly — a contiguous range for `Row`, a
/// `step_by(cols)` stride for `Column` — rather than scanning every cell in the grid and testing each one against
/// membership. [`contains`](Self::contains) is the `O(1)`-per-cell, allocation-free membership test
/// `Scene::set_selection` uses alongside that walk: to check whether an index it reaches while walking one band is
/// already covered by the other, so it is not visited twice — not, itself, how a band's own members are found.
///
/// [`contains`](Self::contains)/[`for_each_index`](Self::for_each_index) both trust their own caller to only ever
/// query a flat index — or, for `for_each_index`, a `len` bound — that names a real cell.
/// [`super::DataNodeContent::resolve_selection`] is the only place that builds a `ResolvedBand`.
///
/// `Scene::set_selection` is the only reader, and every index it ever queries either comes from
/// [`for_each_index`](Self::for_each_index) itself (already `< len`) or from a focus index
/// [`super::DataNodeContent::resolve_selection`] already validated. So neither method here ever needs to re-check
/// a query against the content's own value count itself.
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `f(i)` once for every flat index `i < len` this band contains, in ascending order — this band's own
    /// members, not a `0..len` scan tested one at a time via [`contains`](Self::contains).
    ///
    /// `len` clamps to the content's own actual value count, for the same reason [`contains`](Self::contains)'s own
    /// doc comment gives: a short last row/column can leave a nominal member past the real data.
    ///
    /// `Scene::set_selection` uses this to touch only the cells a changed band could plausibly have changed the
    /// category of — `O(row width)`/`O(column height)`, not `O(len)` — rather than testing every cell in the grid.
    pub(crate) fn for_each_index(self, len: usize, mut f: impl FnMut(usize)) {
        match self {
            Self::None => {},
            Self::Row { row, cols } => {
                for i in (row * cols)..((row + 1) * cols).min(len) {
                    f(i);
                }
            },
            Self::Column { col, cols } => {
                for i in (col..len).step_by(cols) {
                    f(i);
                }
            },
        }
    }
}
