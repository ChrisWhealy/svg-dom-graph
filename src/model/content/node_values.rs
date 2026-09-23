use super::{ByteOrder, DataFormat, format_binary_into, format_decimal_into, format_hex_into};
use crate::colours::{TYPE_COLOUR_U8, TYPE_COLOUR_U16, TYPE_COLOUR_U32, TYPE_COLOUR_U64};

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
/// The one value among `values`, if any, that [`format`](DataFormat) is guaranteed to render widest — without
/// formatting any of them.
///
/// [`DataFormat::Hexadecimal`]/[`DataFormat::Binary`] give every value the same rendered width regardless of
/// magnitude — a fixed number of bytes' worth of digits, all `values` here sharing one integer width already (see
/// [`NodeValues`]'s own "Deliberately one width per node" doc section) — so the first value is exactly as
/// representative as any other. [`DataFormat::Decimal`]'s own width instead grows with magnitude, and these are all
/// unsigned, so the numerically largest value is always the widest one to render — found here by a plain
/// comparison, `Ord::max`, not by formatting every value just to compare the resulting text lengths.
fn pick_widest<T: Ord + Copy>(values: &[T], format: DataFormat) -> Option<T> {
    match format {
        DataFormat::Decimal => values.iter().max().copied(),
        DataFormat::Hexadecimal | DataFormat::Binary => values.first().copied(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values a [`super::DataNodeContent`] displays. Each variant names the Rust integer type the values were captured as.
/// This determines how many bytes [`super::DataFormat::Hexadecimal`]/[`super::DataFormat::Binary`] split each value into. It also
/// determines which colour is assigned (`super::NodeValues::type_colour`, crate-private).
///
/// Only the widths a caller is actually likely to inspect byte-by-byte are supported. A wider type — for example `u128`
/// — can be added the same way later, additively, if it turns out to be needed.
///
/// # Deliberately one width per node
///
/// A `NodeValues` holds exactly one variant. So every value in a given [`super::DataNodeContent`] shares the same integer
/// width. A node can display `[u32, u32, u32, u32]`, never `[u8, u16, u32, u64]`. This is a deliberate scope decision,
/// not a limitation to lift later. The intended abstraction here is "display an array/vector of homogeneous numeric
/// values" — a memory dump, a register bank, a typed buffer — not a general, per-cell-typed data inspector. Every value
/// in one node sharing the same `type_colour` follows directly from that: colour identifies the node's own type, not
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
    pub(super) fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::U64(v) => v.len(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `f(index, formatted)` once for every value, in order, formatted per `format` and `byte_order` — one
    /// cell per value, not yet arranged into a grid (see [`DataNodeContent::shape`] for that).
    ///
    /// `scratch` is cleared and reformatted into on every value, via [`format_decimal_into`]/[`format_hex_into`]/
    /// [`format_binary_into`], rather than allocating a fresh `String` per value the way collecting into a
    /// `Vec<String>` does. `f` borrows `scratch`'s own contents for the duration of one call only — the string is
    /// not valid, and must not be kept, past that call returning.
    ///
    /// Dispatches on `format` once, before the loop, not once per value: [`DataFormat::Decimal`] never computes
    /// `x.to_be_bytes()`/[`order_bytes`] at all, since [`format_decimal_into`] only ever needs the value's own
    /// numeric magnitude. Computing and byte-order-reordering a value's own bytes only to have `Decimal` discard
    /// them unused, on every value, is exactly the wasted work this avoids.
    pub(super) fn for_each_cell_string(
        &self,
        format: DataFormat,
        byte_order: ByteOrder,
        scratch: &mut String,
        mut f: impl FnMut(usize, &str),
    ) {
        match self {
            Self::U8(v) => match format {
                DataFormat::Decimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_decimal_into(u128::from(x), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Hexadecimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_hex_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Binary => {
                    for (i, &x) in v.iter().enumerate() {
                        format_binary_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
            },
            Self::U16(v) => match format {
                DataFormat::Decimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_decimal_into(u128::from(x), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Hexadecimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_hex_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Binary => {
                    for (i, &x) in v.iter().enumerate() {
                        format_binary_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
            },
            Self::U32(v) => match format {
                DataFormat::Decimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_decimal_into(u128::from(x), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Hexadecimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_hex_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Binary => {
                    for (i, &x) in v.iter().enumerate() {
                        format_binary_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
            },
            Self::U64(v) => match format {
                DataFormat::Decimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_decimal_into(u128::from(x), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Hexadecimal => {
                    for (i, &x) in v.iter().enumerate() {
                        format_hex_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
                DataFormat::Binary => {
                    for (i, &x) in v.iter().enumerate() {
                        format_binary_into(order_bytes(x.to_be_bytes(), byte_order), scratch);
                        f(i, scratch);
                    }
                },
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats into `out` whichever one value [`pick_widest`] identifies as guaranteed to render the widest cell
    /// under `format`/`byte_order` — clearing `out` first, then leaving it empty if this holds no values at all.
    ///
    /// A caller measuring the one rendered width every cell in a node shares (every cell uses one monospace font —
    /// see `scene::node::draw_content_box`'s own doc comment) needs exactly this one value's own text, not every
    /// value's. Finding it via [`pick_widest`] rather than [`for_each_cell_string`](Self::for_each_cell_string)
    /// means this formats one value, not every value, to answer that.
    pub(super) fn widest_cell_string(&self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
        out.clear();
        match self {
            Self::U8(v) => {
                if let Some(x) = pick_widest(v, format) {
                    match format {
                        DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                        DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                        DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    }
                }
            },
            Self::U16(v) => {
                if let Some(x) = pick_widest(v, format) {
                    match format {
                        DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                        DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                        DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    }
                }
            },
            Self::U32(v) => {
                if let Some(x) = pick_widest(v, format) {
                    match format {
                        DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                        DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                        DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    }
                }
            },
            Self::U64(v) => {
                if let Some(x) = pick_widest(v, format) {
                    match format {
                        DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                        DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                        DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    }
                }
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats the first value, per `format` and `byte_order`, into caller-owned `out` — `false`, leaving `out`
    /// untouched, if this holds no values at all.
    ///
    /// For a caller that already knows it holds exactly one value — an operator's own already-validated result,
    /// [`super::DataNodeContent::single_cell_string_into`]'s one caller — so it never needs
    /// [`for_each_cell_string`](Self::for_each_cell_string)'s per-value iteration just to reach the one string it
    /// would ever visit, nor allocate a fresh `String` to hold it: `draw_operator_box` already has its own
    /// construction-scratch buffer in hand, and reuses it for this too.
    pub(super) fn single_cell_string_into(&self, format: DataFormat, byte_order: ByteOrder, out: &mut String) -> bool {
        match self {
            Self::U8(v) => v.first().is_some_and(|&x| {
                match format {
                    DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                }
                true
            }),
            Self::U16(v) => v.first().is_some_and(|&x| {
                match format {
                    DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                }
                true
            }),
            Self::U32(v) => v.first().is_some_and(|&x| {
                match format {
                    DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                }
                true
            }),
            Self::U64(v) => v.first().is_some_and(|&x| {
                match format {
                    DataFormat::Decimal => format_decimal_into(u128::from(x), out),
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(x.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(x.to_be_bytes(), byte_order), out),
                }
                true
            }),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A gentle pastel background colour identifies this content's own Rust type. It is deliberately soft, not a harsh
    /// primary. So a data node's colouring reads as a quiet label, not an alarm.
    ///
    /// Each width gets a distinct hue, chosen to stay visually distinct from
    /// [`PLAIN_BOX_FILL`](crate::colours::PLAIN_BOX_FILL), the pale blue every ordinary node — and a multi-value
    /// data node's own outer box — already uses. See [`crate::colours`] for every shade this crate draws with,
    /// including these four.
    pub(crate) fn type_colour(&self) -> &'static str {
        match self {
            Self::U8(_) => TYPE_COLOUR_U8,
            Self::U16(_) => TYPE_COLOUR_U16,
            Self::U32(_) => TYPE_COLOUR_U32,
            Self::U64(_) => TYPE_COLOUR_U64,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A short, human-readable name for this content's own Rust type ("u8"/"u16"/"u32"/"u64").
    ///
    /// [`type_colour`](Self::type_colour) is the only visual cue distinguishing one width from another. A caller who
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
