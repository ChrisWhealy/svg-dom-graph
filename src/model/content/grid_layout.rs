// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`super::DataNodeContent`]'s values are arranged into a grid. See this module's own doc comment ("Grid shape") for
/// [`Automatic`](Self::Automatic)'s exact rule. That section also explains why `Automatic` can still render far from
/// physically square.
///
/// Build one directly — every variant's own field is mandatory. So there is no sensible all-default state besides
/// [`Automatic`](Self::Automatic) itself, which [`Default`] already provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GridLayout {
    /// The default: [`super::DataNodeContent::new`]'s own choice when [`super::DataNodeContent::with_layout`] is never called. Picks
    /// a row count purely from the value *count* — see this module's own doc comment for the exact rule.
    #[default]
    Automatic,
    /// Exactly `n` columns; the row count is however many rows of `n` it takes to fit every
    /// value (`values.len().div_ceil(n)`).
    ///
    /// `n` must be `>= 1`. [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects `Columns(0)`
    /// with [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout). A grid with no columns has nowhere to
    /// place any value.
    Columns(usize),
    /// Exactly `n` rows; the column count is however many columns of `n` it takes to fit every
    /// value (`values.len().div_ceil(n)`).
    ///
    /// `n` must be `>= 1` — rejected the same way as `Columns(0)`, for the same reason.
    Rows(usize),
    /// At most `n` columns. Like [`Automatic`](Self::Automatic), a caller need not work out the row count by hand. But
    /// it never exceeds `n` columns, regardless of value count. This directly fixes a wide-celled grid — for example,
    /// [`super::DataFormat::Binary`] `u64` values — that would otherwise render far wider than tall under `Automatic`'s
    /// cell-count-only rule.
    ///
    /// `n` must be `>= 1` — rejected the same way as `Columns(0)`, for the same reason.
    MaxColumns(usize),
}

impl GridLayout {
    /// `false` for a `Columns`/`Rows`/`MaxColumns` wrapping `0` — such a grid has no columns (or rows) to place any
    /// value in. `Automatic` is always valid.
    pub(crate) fn is_valid(self) -> bool {
        !matches!(self, Self::Columns(0) | Self::Rows(0) | Self::MaxColumns(0))
    }
}
