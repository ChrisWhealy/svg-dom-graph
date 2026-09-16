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
