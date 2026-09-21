use super::{ByteOrder, DataFormat, format_value, format_value_into};

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
/// The values a [`super::DataNodeContent`] displays. Each variant names the Rust integer type the values were captured as.
/// This determines how many bytes [`super::DataFormat::Hexadecimal`]/[`super::DataFormat::Binary`] split each value into. It also
/// determines which colour is assigned (`super::NodeValues::type_color`, crate-private).
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
    /// `scratch` is cleared and reformatted into on every value, via [`format_value_into`], rather than allocating a
    /// fresh `String` per value the way collecting into a `Vec<String>` does. `f` borrows `scratch`'s own contents
    /// for the duration of one call only — the string is not valid, and must not be kept, past that call returning.
    pub(super) fn for_each_cell_string(
        &self,
        format: DataFormat,
        byte_order: ByteOrder,
        scratch: &mut String,
        mut f: impl FnMut(usize, &str),
    ) {
        match self {
            Self::U8(v) => {
                for (i, &x) in v.iter().enumerate() {
                    format_value_into(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format, scratch);
                    f(i, scratch);
                }
            },
            Self::U16(v) => {
                for (i, &x) in v.iter().enumerate() {
                    format_value_into(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format, scratch);
                    f(i, scratch);
                }
            },
            Self::U32(v) => {
                for (i, &x) in v.iter().enumerate() {
                    format_value_into(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format, scratch);
                    f(i, scratch);
                }
            },
            Self::U64(v) => {
                for (i, &x) in v.iter().enumerate() {
                    format_value_into(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format, scratch);
                    f(i, scratch);
                }
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The first value, formatted per `format` and `byte_order` — `None` if this holds no values at all.
    ///
    /// For a caller that already knows it holds exactly one value — an operator's own already-validated result,
    /// [`super::DataNodeContent::single_cell_string`]'s one caller — so it never needs
    /// [`for_each_cell_string`](Self::for_each_cell_string)'s per-value iteration just to reach the one string it
    /// would ever visit.
    pub(super) fn single_cell_string(&self, format: DataFormat, byte_order: ByteOrder) -> Option<String> {
        match self {
            Self::U8(v) => v
                .first()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format)),
            Self::U16(v) => v
                .first()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format)),
            Self::U32(v) => v
                .first()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format)),
            Self::U64(v) => v
                .first()
                .map(|&x| format_value(order_bytes(x.to_be_bytes(), byte_order), u128::from(x), format)),
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
