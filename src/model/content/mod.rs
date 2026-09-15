//! [`DataNodeContent`]: a node whose visible content is a grid of typed numeric values, rather than a plain text label —
//! see [`Scene::add_data_node`](crate::scene::Scene::add_data_node) and [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with).
//!
//! This module contains pure data and formatting logic, with no DOM of its own. It is unit-tested with a plain
//! `cargo test`, following the same convention used by [`crate::geometry`] for its own DOM-free routing mathematics.
//! The job of turning a [`DataNodeContent`] into actual `<rect>`/`<text>` elements and sizing the node's box belongs to
//! `scene::node` instead (see that module's own `draw_content_box`).
//!
//! # Grid shape
//!
//! For `n` values, [`DataNodeContent::shape`] fills the grid row-major, with any leftover slots in the last row left
//! blank, arranged according to [`GridLayout`] — [`GridLayout::Automatic`] (the default, used unless
//! [`DataNodeContent::with_layout`] overrides it) picks between two rules for the shape itself:
//!
//! 1. If some power of two, strictly between `1` and `n`, divides `n` evenly, [`best_power_of_two_rows`] picks the one
//!    giving the squarest grid as the row count.  For example, `n = 8` renders as 2 rows of 4, not 3 rows of 3 (with
//!    one slot left blank), and `n = 20` renders as 4 rows of 5.
//!    This is deliberately biased toward the row/column groupings most commonly seen in memory dumps and register views
//!    (a power-of-two row count), rather than simply rendering everything as "closest to square".
//! 2. Otherwise, if `n` is odd, or `n` itself is `1` or `2`, then no such power of two exists.
//!    So `rows = ceil(sqrt(n))`, `cols = ceil(n / rows)` instead, which yields the closest-to-square shape available.
//!    It also prefers more rows over more columns when `n` is not a perfect square: `n = 2` renders as two rows of one
//!    column each, not one row of two, and `n = 25` renders as a 5×5 square.
//!
//! `Automatic` counts *cells*, not physical size: it has no idea how wide each cell's own formatted text measures
//! out to. A `4 × 4` square of [`DataFormat::Binary`] `u64` cells (each roughly 35 characters wide) is just as
//! "square" to this rule as a `4 × 4` grid of short [`DataFormat::Hexadecimal`] `u8` cells, but renders dramatically
//! wider than it is tall once `scene::node`'s own `draw_content_box` measures each cell's real rendered width.
//! [`GridLayout::MaxColumns`] is the direct way to cap that width regardless of value count;
//! [`GridLayout::Columns`]/[`GridLayout::Rows`] hand full control to the caller instead.
//!
//! # Formatting
//!
//! Each value is formatted using its own type's big-endian byte representation (`to_be_bytes()` — most
//! significant byte first, regardless of host endianness, so the displayed digits always read the same way a
//! human would write the number down):
//!
//! - [`DataFormat::Hexadecimal`]: every byte as two uppercase hex digits, space-separated, no `0x` prefix — e.g.
//!   `"F0 E1 D2 C3 B4 A5 96 87"` for a `u64`.
//! - [`DataFormat::Binary`]: every byte as eight binary digits, space-separated, with a further gap splitting each
//!   byte's own upper and lower nybble — e.g. `"1111 0000"` for one byte, so a reader can pick out a nybble at a
//!   glance rather than counting along an unbroken run of eight digits.
//! - [`DataFormat::Decimal`]: the whole value as one plain decimal number — decimal has no natural byte boundary
//!   to split on, unlike hexadecimal and binary.
//!
//! # Telling values apart
//!
//! A byte-group string alone does not say where one value ends and the next begins — `"00 11 22 33 44 55 66 77"`
//! reads the same irrespective of whether it is one `u64`, two `u32`s, four `u16`s, or eight `u8`s. Rather than lean on
//! whitespace to imply a boundary, `scene::node::draw_content_box` gives each value its own, inner box, coloured by its
//! own type (`NodeValues::type_color`, crate-private). A value's own width and boundaries are then a property of the
//! box it sits in, not something a reader has to count bytes to infer. With two or more values that box sits inside the
//! node's own (unchanged, light blue) box; with exactly one value, [`DataNodeContent::is_single_value`]'s own doc comment
//! explains why the inner box is dropped and the type colour is applied directly.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values a [`DataNodeContent`] displays, each variant naming the Rust integer type they were captured as — this
/// determines how many bytes [`DataFormat::Hexadecimal`]/[`DataFormat::Binary`] split each value into, and which
/// colour it is assigned (`NodeValues::type_color`, crate-private).
///
/// Only the widths a caller is actually likely to want to inspect byte-by-byte are supported. A wider type (for
/// example `u128`) can be added the same way later, additively, if it turns out to be needed.
///
/// # Deliberately one width per node
///
/// A `NodeValues` holds exactly one variant, so every value in a given [`DataNodeContent`] shares the same integer
/// width — a node can display `[u32, u32, u32, u32]`, never `[u8, u16, u32, u64]`. This is a deliberate scope
/// decision, not a limitation to lift later: the intended abstraction here is "display an array/vector of
/// homogeneous numeric values" (a memory dump, a register bank, a typed buffer), not a general, per-cell-typed
/// data inspector. Every value in one node getting the same `type_color` follows directly from that: colour
/// identifies the node's own type, not each individual cell's.
///
/// A heterogeneous node (`Vec<NodeValue>` with one width per value, à la a tagged-union cell type) is a
/// legitimate, larger feature in its own right, not an incremental change to this one — building it later, should
/// a real caller need it, is expected to sit alongside this type rather than replace it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeValues {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
}

impl NodeValues {
    /// How many values this holds, regardless of width — what [`grid_shape`] arranges into a grid.
    fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::U64(v) => v.len(),
        }
    }

    /// Every value, formatted per `format`, in the same order they were supplied — one cell per value, not yet
    /// arranged into a grid (see [`DataNodeContent::shape`] for that).
    fn cell_strings(&self, format: DataFormat) -> Vec<String> {
        match self {
            Self::U8(v) => v
                .iter()
                .map(|&x| format_value(x.to_be_bytes(), u128::from(x), format))
                .collect(),
            Self::U16(v) => v
                .iter()
                .map(|&x| format_value(x.to_be_bytes(), u128::from(x), format))
                .collect(),
            Self::U32(v) => v
                .iter()
                .map(|&x| format_value(x.to_be_bytes(), u128::from(x), format))
                .collect(),
            Self::U64(v) => v
                .iter()
                .map(|&x| format_value(x.to_be_bytes(), u128::from(x), format))
                .collect(),
        }
    }

    /// A gentle pastel background colour identifying this content's own Rust type — deliberately soft rather than
    /// a harsh primary, so a data node's colouring reads as a quiet label, not an alarm.
    ///
    /// Each width gets a distinct hue, chosen to also stay visually distinct from `#eef4ff`, the pale blue every
    /// ordinary node (and a multi-value data node's own outer box) already uses.
    pub(crate) fn type_color(&self) -> &'static str {
        match self {
            Self::U8(_) => "#fdebd3",  // pastel apricot
            Self::U16(_) => "#dcefdc", // pastel mint
            Self::U32(_) => "#e6dcf5", // pastel lavender
            Self::U64(_) => "#f5dce4", // pastel rose
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats one value's already-big-endian `bytes`, or its `decimal` value directly for
/// [`DataFormat::Decimal`] — generic over the byte width so [`NodeValues::cell_strings`] needs one call site per
/// variant, not one formatting implementation per width.
fn format_value<const N: usize>(bytes: [u8; N], decimal: u128, format: DataFormat) -> String {
    match format {
        DataFormat::Decimal => decimal.to_string(),
        DataFormat::Hexadecimal => bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "),
        DataFormat::Binary => bytes
            .iter()
            .map(|b| format!("{:04b} {:04b}", b >> 4, b & 0x0F))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`DataNodeContent`] renders each value's digits. See this module's own doc comment for exactly what each
/// variant produces.
///
/// `#[non_exhaustive]` for the same reason as [`NodeValues`]: a plausible future addition (`Octal`, `Ascii`, ...)
/// should stay additive, not a source-breaking change for a caller who exhaustively matched this before it existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFormat {
    Decimal,
    Hexadecimal,
    Binary,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`DataNodeContent`]'s values are arranged into a grid — see this module's own doc comment ("Grid shape")
/// for [`Automatic`](Self::Automatic)'s exact rule, and why it can still render far from physically square.
///
/// Build one directly — every variant's own field is mandatory, so there is no sensible all-default state besides
/// [`Automatic`](Self::Automatic) itself, which [`Default`] already provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GridLayout {
    /// The default: [`DataNodeContent::new`]'s own choice when [`DataNodeContent::with_layout`] is never called.
    /// Picks a row count purely from the value *count* — see this module's own doc comment for the exact rule.
    #[default]
    Automatic,
    /// Exactly `n` columns; the row count is however many rows of `n` it takes to fit every value
    /// (`values.len().div_ceil(n)`).
    ///
    /// `n` must be `>= 1` — [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects
    /// `Columns(0)` with [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout), since a grid with no
    /// columns has nowhere to place any value.
    Columns(usize),
    /// Exactly `n` rows; the column count is however many columns of `n` it takes to fit every value
    /// (`values.len().div_ceil(n)`).
    ///
    /// `n` must be `>= 1` — rejected the same way as `Columns(0)`, for the same reason.
    Rows(usize),
    /// At most `n` columns: like [`Automatic`](Self::Automatic) in that a caller does not have to work out the row
    /// count by hand, but never wider than `n` columns regardless of how many values there are — the direct fix
    /// for a wide-celled grid (for example, [`DataFormat::Binary`] `u64` values) that would otherwise render far
    /// wider than tall under `Automatic`'s cell-count-only rule.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid shape for `n` values, arranged per `layout` — see this module's own doc comment for the reasoning
/// behind [`GridLayout::Automatic`]'s two rules, and [`best_power_of_two_rows`] for how the first one picks a row
/// count.
///
/// Returns `(0, 0)` for `n = 0` — [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects an
/// empty [`DataNodeContent`] before this is ever reached, so that case has no real grid to compute anyway.
///
/// `layout`'s own `Columns`/`Rows`/`MaxColumns` fields must already be known non-zero —
/// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects a zero one with
/// [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout) before this is ever reached, exactly as it
/// does for `n = 0`.
fn grid_shape(n: usize, layout: GridLayout) -> (usize, usize) {
    if n == 0 {
        return (0, 0);
    }
    match layout {
        GridLayout::Automatic => automatic_grid_shape(n),
        GridLayout::Columns(cols) => (n.div_ceil(cols), cols),
        GridLayout::Rows(rows) => (rows, n.div_ceil(rows)),
        GridLayout::MaxColumns(max_cols) => {
            let cols = max_cols.min(n);
            (n.div_ceil(cols), cols)
        },
    }
}

/// [`GridLayout::Automatic`]'s own shape rule — see this module's own doc comment for the reasoning behind both.
fn automatic_grid_shape(n: usize) -> (usize, usize) {
    if let Some(rows) = best_power_of_two_rows(n) {
        return (rows, n / rows);
    }
    // Fallback: n has no row count that is both a power of two and a non-trivial divisor of n (n is odd, or
    // n == 1) — the closest-to-square shape from `ceil(sqrt(n))` is the best available instead.
    let rows = (n as f64).sqrt().ceil() as usize;
    let cols = n.div_ceil(rows);
    (rows, cols)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The row count [`grid_shape`] prefers: the power of two, strictly between `1` and `n`, that evenly divides `n`
/// and gives the grid closest to square — i.e. the smallest `|cols - rows|`, where `cols = n / rows`.
///
/// `1` and `n` are excluded even when they would themselves be powers of two (e.g. `n = 8` could technically use
/// `rows = 1` or `rows = 8`): both describe a single row or a single column, not a real "grouping" of the kind
/// this rule exists to prefer — see this module's own doc comment.
///
/// Ties are broken toward the *smaller* candidate `rows` (so more, narrower rows lose to fewer, wider ones) —
/// `n = 8` has two equally square candidates, `rows = 2` (2×4) and `rows = 4` (4×2), and the wider `2×4` is the
/// one actually preferred. Candidates are checked in ascending order and only replaced by a strictly better
/// score, which is what gives the smaller candidate this priority on a tie.
///
/// Returns `None` if no such `rows` exists — every prime `n`, and `n = 1` or `n = 2`, whose only power-of-two
/// divisor is the trivial `1`.
fn best_power_of_two_rows(n: usize) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None; // (rows, squareness score)
    let mut candidate_rows: usize = 2;
    while candidate_rows < n {
        if n % candidate_rows == 0 {
            let cols = n / candidate_rows;
            let score = cols.abs_diff(candidate_rows);
            if best.is_none_or(|(_, best_score)| score < best_score) {
                best = Some((candidate_rows, score));
            }
        }
        candidate_rows = match candidate_rows.checked_mul(2) {
            Some(doubled) => doubled,
            None => break,
        };
    }
    best.map(|(rows, _)| rows)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's content: a set of typed numeric values, displayed as a grid as close to square as the value count
/// allows — see this module's own doc comment for the exact layout, formatting, and colouring rules.
///
/// Build one with [`DataNodeContent::new`]. Unlike [`NodeOptions`](crate::scene::NodeOptions)/
/// [`DragOptions`](crate::scene::DragOptions), there is no sensible all-default state to build one on top of — the
/// values are mandatory — so this is a plain constructor rather than a `default()` plus `with_*` builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataNodeContent {
    values: NodeValues,
    format: DataFormat,
    layout: GridLayout,
}

impl DataNodeContent {
    /// Builds a [`DataNodeContent`] displaying `values`, formatted as `format`, arranged per
    /// [`GridLayout::Automatic`] — see [`with_layout`](Self::with_layout) to override that.
    ///
    /// An empty `values` is accepted here — the same deferred-validation convention
    /// [`DragOptions::with_bounds`](crate::scene::DragOptions::with_bounds)/
    /// [`NodeOptions::with_edge_anchors`](crate::scene::NodeOptions::with_edge_anchors) already follow — but is rejected
    /// with [`Error::EmptyNodeContent`](crate::error::Error::EmptyNodeContent) by
    /// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with), which is the only place it would actually
    /// need a grid to draw.
    #[must_use]
    pub fn new(values: NodeValues, format: DataFormat) -> Self {
        Self {
            values,
            format,
            layout: GridLayout::default(),
        }
    }

    /// Returns `self` with `layout` overriding [`GridLayout::Automatic`]'s own cell-count-only rule — see
    /// [`GridLayout`]'s own doc comment for what each variant does, and this module's own doc comment ("Grid
    /// shape") for why `Automatic` alone is not always enough.
    ///
    /// `layout`'s own `Columns`/`Rows`/`MaxColumns` value is accepted here even if `0` — the same deferred-validation
    /// convention `new`'s own doc comment describes — but is rejected with
    /// [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout) by
    /// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with).
    #[must_use]
    pub fn with_layout(mut self, layout: GridLayout) -> Self {
        self.layout = layout;
        self
    }

    /// How many values this holds.
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    /// `true` for exactly one value.
    ///
    /// A single value has no sibling to be told apart from, so `draw_content_box` skips the per-value inner box
    /// [`NodeValues::type_color`] would otherwise use, and applies that colour straight to the node's own single
    /// box instead — one box, one colour, no redundant box-within-a-box.
    pub(crate) fn is_single_value(&self) -> bool {
        self.values.len() == 1
    }

    /// This content's own [`GridLayout`] — `Automatic` unless [`with_layout`](Self::with_layout) overrode it.
    pub(crate) fn layout(&self) -> GridLayout {
        self.layout
    }

    /// The `(rows, cols)` grid this content renders as — see [`grid_shape`].
    pub(crate) fn shape(&self) -> (usize, usize) {
        grid_shape(self.values.len(), self.layout)
    }

    /// Every value's own formatted cell text, in the same order they were supplied — one string per value, ready
    /// for `draw_content_box` to place one at a time into the grid [`DataNodeContent::shape`] describes.
    pub(crate) fn cells(&self) -> Vec<String> {
        self.values.cell_strings(self.format)
    }

    /// The pastel colour identifying this content's own value type — see [`NodeValues::type_color`].
    pub(crate) fn type_color(&self) -> &'static str {
        self.values.type_color()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
