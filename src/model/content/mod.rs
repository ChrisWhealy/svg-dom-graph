//! [`DataNodeContent`]: a node whose visible content is a grid of typed numeric values, not a plain text label. See
//! [`Scene::add_data_node`](crate::scene::Scene::add_data_node)
//! and [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with).
//!
//! [`UnaryOperator`]/[`BinaryOperator`] name a bitwise operation an operator node's own label describes. This crate
//! never evaluates one: a caller supplies the already-computed result, the same way it supplies every other data
//! node's own values. See [`Scene::add_unary_operator_node`](crate::scene::Scene::add_unary_operator_node) and
//! [`Scene::add_binary_operator_node`](crate::scene::Scene::add_binary_operator_node).
//!
//! This module contains pure data and formatting logic, with no DOM of its own. It is unit-tested with a plain `cargo
//! test`. This follows the same convention [`crate::geometry`] uses for its own DOM-free routing mathematics.
//! `scene::node` turns a [`DataNodeContent`] into actual `<rect>`/`<text>` elements. It also sizes the node's box to
//! fit them — see that module's own `draw_content_box`.
//!
//! # Grid shape
//!
//! For `n` values, [`DataNodeContent::shape`] fills the grid row-major. Any leftover slots in the last row stay blank.
//! [`GridLayout`] controls the arrangement. [`GridLayout::Automatic`] is the default, used unless
//! [`DataNodeContent::with_layout`] overrides it. `Automatic` picks between two rules:
//!
//! 1. Suppose some power of two, strictly between `1` and `n`, divides `n` evenly. Then [`best_power_of_two_rows`]
//!    picks the one giving the squarest grid as the row count. For example, `n = 8` renders as 2 rows of 4, not 3 rows
//!    of 3 with one slot left blank. `n = 20` renders as 4 rows of 5. This deliberately favours the row/column
//!    groupings common in memory dumps and register views — a power-of-two row count — over simply rendering everything
//!    "closest to square".
//! 2. Otherwise, no such power of two exists: `n` is odd, or `n` itself is `1` or `2`. Then `rows = ceil(sqrt(n))` and
//!    `cols = ceil(n / rows)` instead. This yields the closest-to-square shape available. It also prefers more rows
//!    over more columns when `n` is not a perfect square. So `n = 2` renders as two rows of one column each, not one
//!    row of two. And `n = 25` renders as a 5×5 square.
//!
//! `Automatic` counts *cells*, not physical size. It has no idea how wide each cell's own formatted text measures out
//! to. A `4 × 4` square of [`DataFormat::Binary`] `u64` cells is just as "square" to this rule as a `4 × 4` grid of
//! short [`DataFormat::Hexadecimal`] `u8` cells. Yet each binary cell is roughly 35 characters wide. `scene::node`'s
//! own `draw_content_box` measures each cell's real rendered width. So the binary grid renders far wider than it is
//! tall. [`GridLayout::MaxColumns`] directly caps that width, regardless of value count.
//! [`GridLayout::Columns`]/[`GridLayout::Rows`] instead hand full control to the caller.
//!
//! # Formatting
//!
//! Each value is formatted using its own type's byte representation. [`ByteOrder`] controls the byte order.
//! [`ByteOrder::BigEndian`] is the default, used unless [`DataNodeContent::with_byte_order`] overrides it. `BigEndian`
//! puts the most significant byte first, regardless of host endianness. So the displayed digits always read the same
//! way a human would write the number down. They also read the same on every host, not just whichever host built
//! the diagram:
//!
//! - [`DataFormat::Hexadecimal`]: every byte as two uppercase hex digits, space-separated, no `0x` prefix — e.g. `"F0
//!   E1 D2 C3 B4 A5 96 87"` for a `u64` under `BigEndian`.
//! - [`DataFormat::Binary`]: every byte as eight binary digits, space-separated. A further gap splits each byte's own
//!   upper and lower nybble — e.g. `"1111 0000"` for one byte. This lets a reader spot a nybble at a glance, rather
//!   than counting along an unbroken run of eight digits.
//! - [`DataFormat::Decimal`]: the whole value as one plain decimal number. Decimal has no natural byte boundary to
//!   split on, unlike hexadecimal and binary. So [`ByteOrder`] has no visible effect under `Decimal`: the same number
//!   reads the same regardless of which byte order produced it.
//!
//! [`ByteOrder::BigEndian`]'s determinism, independent of host endianness, suits register/value display. A reader there
//! expects the digits to read the same way a number is normally written down. A caller visualising an actual in-memory
//! byte layout wants [`ByteOrder::LittleEndian`] instead, to match it. There, byte order is a property of the data
//! being inspected, not a display preference.
//!
//! # Telling values apart
//!
//! A byte-group string alone does not say where one value ends and the next begins. `"00 11 22 33 44 55 66 77"` reads
//! the same whether it is one `u64`, two `u32`s, four `u16`s, or eight `u8`s. `scene::node::draw_content_box` avoids
//! leaning on whitespace to imply a boundary. Instead it gives each value its own inner box, coloured by its own type
//! (`NodeValues::type_color`, crate-private). A value's own width and boundaries then become a property of the box it
//! sits in, not something a reader must count bytes to infer. With two or more values, that box sits inside the node's
//! own unchanged, light blue box. With exactly one value, the inner box is dropped instead. The type colour applies
//! directly to the node's own box — see [`DataNodeContent::is_single_value`]'s own doc comment for why.
//!
//! # Colour is not the only way to distinguish a type
//!
//! `type_color` distinguishes datatypes visually, using colour. But colour alone is invisible to assistive technology,
//! and unreliable for a colour-blind reader. This crate offers no caller-facing way to map a colour back to a type name
//! either. So `scene::node::draw_content_box` also attaches the node's own type name — e.g. "u8"/"u16"/"u32"/"u64" — as
//! an SVG `<title>` on the node's own `<g>`. This gives a native browser tooltip when the mouse pointer hovers over any
//! child: the rect or the rendered digits alike.
//!
//! Neither clutters the rendered digits themselves: the type stays discoverable, not displayed. This is deliberately
//! not attached to each rect/text individually. A `<title>` names only its own direct parent, not a sibling. So a
//! `<title>` on a value's own rect would not produce a tooltip over that same value's own text — a sibling element, not
//! a descendant. And a `<title>` as a child of the `<text>` element itself would leak its own text into
//! `text.textContent`, corrupting the rendered digits read back from the DOM.
mod operator;
pub use operator::{BinaryOperator, UnaryOperator};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values a [`DataNodeContent`] displays. Each variant names the Rust integer type the values were captured as.
/// This determines how many bytes [`DataFormat::Hexadecimal`]/[`DataFormat::Binary`] split each value into. It also
/// determines which colour is assigned (`NodeValues::type_color`, crate-private).
///
/// Only the widths a caller is actually likely to inspect byte-by-byte are supported. A wider type — for example `u128`
/// — can be added the same way later, additively, if it turns out to be needed.
///
/// # Deliberately one width per node
///
/// A `NodeValues` holds exactly one variant. So every value in a given [`DataNodeContent`] shares the same integer
/// width. A node can display `[u32, u32, u32, u32]`, never `[u8, u16, u32, u64]`. This is a deliberate scope decision,
/// not a limitation to lift later. The intended abstraction here is "display an array/vector of homogeneous numeric
/// values" — a memory dump, a register bank, a typed buffer — not a general, per-cell-typed data inspector. Every value
/// in one node sharing the same `type_color` follows directly from that: colour identifies the node's own type, not
/// each individual cell's.
///
/// A heterogeneous node — `Vec<NodeValue>` with one width per value, à la a tagged-union cell type — is a legitimate,
/// larger feature in its own right. It is not an incremental change to this one. Building it later, should a real
/// caller need it, is expected to sit alongside this type rather than replace it.
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Every value, formatted per `format` and `byte_order`, in the same order they were supplied — one cell per value,
    /// not yet arranged into a grid (see [`DataNodeContent::shape`] for that).
    fn cell_strings(&self, format: DataFormat, byte_order: ByteOrder) -> Vec<String> {
        match self {
            Self::U8(v) => v
                .iter()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format))
                .collect(),
            Self::U16(v) => v
                .iter()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format))
                .collect(),
            Self::U32(v) => v
                .iter()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format))
                .collect(),
            Self::U64(v) => v
                .iter()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format))
                .collect(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A gentle pastel background colour identifies this content's own Rust type. It is deliberately soft, not a harsh
    /// primary. So a data node's colouring reads as a quiet label, not an alarm.
    ///
    /// Each width gets a distinct hue, chosen to stay visually distinct from `#eef4ff`. `#eef4ff` is the pale blue
    /// every ordinary node — and a multi-value data node's own outer box — already uses.
    pub(crate) fn type_color(&self) -> &'static str {
        match self {
            Self::U8(_) => "#fdebd3",  // pastel apricot
            Self::U16(_) => "#dcefdc", // pastel mint
            Self::U32(_) => "#e6dcf5", // pastel lavender
            Self::U64(_) => "#f5dce4", // pastel rose
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A short, human-readable name for this content's own Rust type ("u8"/"u16"/"u32"/"u64").
    ///
    /// [`type_color`](Self::type_color) is the only visual cue distinguishing one width from another. A caller who
    /// cannot distinguish colours has no other way to recover the type. Assistive technology does not perceive fill
    /// colour at all either. `scene::node` attaches this as the node's own `<g>`'s `<title>`, and as its `aria-label`.
    /// That way the type exists as text somewhere, not only as colour.
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::U64(_) => "u64",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The byte order `DataNodeContent::cells` splits each value into, for [`DataFormat::Hexadecimal`]/
/// [`DataFormat::Binary`]. See this module's own doc comment ("Formatting") for why `BigEndian` is the default, and
/// when a caller wants `LittleEndian` instead.
///
/// Has no visible effect under [`DataFormat::Decimal`]. A plain decimal number reads the same regardless of which byte
/// order produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ByteOrder {
    /// Most significant byte first, independent of host endianness. So the displayed digits always read the same way a
    /// human would write the number, on every host. The right choice for register/value display.
    #[default]
    BigEndian,
    /// Least significant byte first. The right choice when a value's own byte order is a property of the data being
    /// inspected — an actual in-memory layout — rather than a display preference.
    LittleEndian,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reorders `bytes` (already big-endian, from `to_be_bytes()`) per `order` — a no-op for `BigEndian`, reversed for
/// `LittleEndian`. A single-byte array (`N = 1`, i.e. `u8`) is unaffected either way: byte order is meaningless for
/// one byte.
fn order_bytes<const N: usize>(mut bytes: [u8; N], order: ByteOrder) -> [u8; N] {
    if order == ByteOrder::LittleEndian {
        bytes.reverse();
    }
    bytes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats one value's own already-ordered `bytes` (see [`order_bytes`]), or its `decimal` value directly for
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
/// `#[non_exhaustive]`, for the same reason as [`NodeValues`]. A plausible future addition — `Octal`, `Ascii`, ... —
/// should stay additive. It should not break a caller who exhaustively matched this already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFormat {
    Decimal,
    Hexadecimal,
    Binary,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`DataNodeContent`]'s values are arranged into a grid. See this module's own doc comment ("Grid shape") for
/// [`Automatic`](Self::Automatic)'s exact rule. That section also explains why `Automatic` can still render far from
/// physically square.
///
/// Build one directly — every variant's own field is mandatory. So there is no sensible all-default state besides
/// [`Automatic`](Self::Automatic) itself, which [`Default`] already provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GridLayout {
    /// The default: [`DataNodeContent::new`]'s own choice when [`DataNodeContent::with_layout`] is never called. Picks
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
    /// [`DataFormat::Binary`] `u64` values — that would otherwise render far wider than tall under `Automatic`'s
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid shape for `n` values, arranged per `layout`. See this module's own doc comment for the reasoning behind
/// [`GridLayout::Automatic`]'s two rules. See [`best_power_of_two_rows`] for how the first rule picks a row count.
///
/// Returns `(0, 0)` for `n = 0`. [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects an
/// empty [`DataNodeContent`] before this is ever reached. So that case has no real grid to compute anyway.
///
/// `layout`'s own `Columns`/`Rows`/`MaxColumns` fields must already be known non-zero.
/// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with) rejects a zero one with
/// [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout) first. This happens before `grid_shape` is ever
/// reached, exactly as it does for `n = 0`.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
/// The row count [`grid_shape`] prefers: the power of two, strictly between `1` and `n`, that evenly divides `n` and
/// gives the grid closest to square — i.e. the smallest `|cols - rows|`, where `cols = n / rows`.
///
/// `1` and `n` are excluded even when they would themselves be powers of two (e.g. `n = 8` could technically use `rows
/// = 1` or `rows = 8`): both describe a single row or a single column, not a real "grouping" of the kind this rule
/// exists to prefer — see this module's own doc comment.
///
/// Ties are broken toward the *smaller* candidate `rows` (so more, narrower rows lose to fewer, wider ones) — `n = 8`
/// has two equally square candidates, `rows = 2` (2×4) and `rows = 4` (4×2), and the wider `2×4` is the one actually
/// preferred. Candidates are checked in ascending order and only replaced by a strictly better score, which is what
/// gives the smaller candidate this priority on a tie.
///
/// Returns `None` if no such `rows` exists — every prime `n`, and `n = 1` or `n = 2`, whose only power-of-two divisor
/// is the trivial `1`.
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
/// A node's content: a set of typed numeric values, displayed as a grid as close to square as the value count allows —
/// see this module's own doc comment for the exact layout, formatting, and colouring rules.
///
/// Build one with [`DataNodeContent::new`]. Unlike [`NodeOptions`](crate::scene::NodeOptions)/
/// [`DragOptions`](crate::scene::DragOptions), there is no sensible all-default state to build one on top of — the
/// values are mandatory — so this is a plain constructor rather than a `default()` plus `with_*` builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataNodeContent {
    values: NodeValues,
    format: DataFormat,
    layout: GridLayout,
    byte_order: ByteOrder,
}

impl DataNodeContent {
    /// Builds a [`DataNodeContent`] displaying `values`, formatted as `format`. This arranges the grid per
    /// [`GridLayout::Automatic`] — see [`with_layout`](Self::with_layout) to override that. It orders bytes per
    /// [`ByteOrder::BigEndian`] — see [`with_byte_order`](Self::with_byte_order) to override that.
    ///
    /// An empty `values` is accepted here. This is the same deferred-validation convention
    /// [`DragOptions::with_bounds`](crate::scene::DragOptions::with_bounds)/
    /// [`NodeOptions::with_edge_anchors`](crate::scene::NodeOptions::with_edge_anchors) already follow. It is rejected
    /// instead by [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with), with
    /// [`Error::EmptyNodeContent`](crate::error::Error::EmptyNodeContent) — the only place that actually needs a grid
    /// to draw.
    #[must_use]
    pub fn new(values: NodeValues, format: DataFormat) -> Self {
        Self {
            values,
            format,
            layout: GridLayout::default(),
            byte_order: ByteOrder::default(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `layout` overriding [`GridLayout::Automatic`]'s own cell-count-only rule. See
    /// [`GridLayout`]'s own doc comment for what each variant does. See this module's own doc comment ("Grid shape")
    /// for why `Automatic` alone is not always enough.
    ///
    /// `layout`'s own `Columns`/`Rows`/`MaxColumns` value is accepted here even if `0`. This is the same
    /// deferred-validation convention `new`'s own doc comment describes. It is rejected instead by
    /// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with),
    /// with [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout).
    #[must_use]
    pub fn with_layout(mut self, layout: GridLayout) -> Self {
        self.layout = layout;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `byte_order` overriding [`ByteOrder::BigEndian`]'s own default. See [`ByteOrder`]'s own doc
    /// comment for what each variant does. See this module's own doc comment ("Formatting") for when `LittleEndian` is
    /// the right choice.
    #[must_use]
    pub fn with_byte_order(mut self, byte_order: ByteOrder) -> Self {
        self.byte_order = byte_order;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many values this holds.
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for exactly one value.
    ///
    /// A single value has no sibling to be told apart from. So `draw_content_box` skips the per-value inner box
    /// [`NodeValues::type_color`] would otherwise use. It applies that colour straight to the node's own single box
    /// instead. That gives one box, one colour, and no redundant box-within-a-box.
    pub(crate) fn is_single_value(&self) -> bool {
        self.values.len() == 1
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own [`GridLayout`] — `Automatic` unless [`with_layout`](Self::with_layout) overrode it.
    pub(crate) fn layout(&self) -> GridLayout {
        self.layout
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `(rows, cols)` grid this content renders as — see [`grid_shape`].
    pub(crate) fn shape(&self) -> (usize, usize) {
        grid_shape(self.values.len(), self.layout)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Every value's own formatted cell text, in the same order they were supplied. One string per value, ready for
    /// `draw_content_box` to place one at a time into the grid [`DataNodeContent::shape`] describes.
    pub(crate) fn cells(&self) -> Vec<String> {
        self.values.cell_strings(self.format, self.byte_order)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pastel colour identifying this content's own value type — see [`NodeValues::type_color`].
    pub(crate) fn type_color(&self) -> &'static str {
        self.values.type_color()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own type name ("u8"/"u16"/"u32"/"u64") — see [`NodeValues::type_name`].
    pub(crate) fn type_name(&self) -> &'static str {
        self.values.type_name()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
