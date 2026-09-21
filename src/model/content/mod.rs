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
mod byte_order;
mod data_format;
mod data_node_content;
mod grid_layout;
mod node_values;
mod operator;
mod selection;

pub use byte_order::ByteOrder;
pub use data_format::DataFormat;
pub use data_node_content::DataNodeContent;
pub use grid_layout::GridLayout;
pub use node_values::NodeValues;
pub use operator::{BinaryOperator, UnaryOperator};
pub(crate) use selection::ResolvedBand;
pub use selection::Selection;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats one value's own already-ordered `bytes` (see [`order_bytes`]), or its `decimal` value directly for
/// [`DataFormat::Decimal`] — generic over the byte width so [`NodeValues::single_cell_string`] needs one call site per
/// variant, not one formatting implementation per width.
///
/// A thin, single-value wrapper around [`format_value_into`] — see that function's own doc comment for a caller
/// formatting more than one value, which should reuse one buffer across all of them instead of allocating a fresh
/// `String` per call the way this one always does.
fn format_value<const N: usize>(bytes: [u8; N], decimal: u128, format: DataFormat) -> String {
    let mut out = String::new();
    format_value_into(bytes, decimal, format, &mut out);
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Same as [`format_value`], but formats into caller-owned `out` instead of returning a fresh `String`.
///
/// Clears `out` first, then writes into it — the same reused-buffer shape [`crate::geometry::elbow_path_into`] already
/// uses for a per-frame `d` attribute. [`NodeValues::for_each_cell_string`] reuses one `out` across every value in a
/// [`super::DataNodeContent`], so formatting `n` values costs at most one buffer growth, amortised across the whole
/// node, rather than `n` separate heap allocations the way collecting into a `Vec<String>` would.
fn format_value_into<const N: usize>(bytes: [u8; N], decimal: u128, format: DataFormat, out: &mut String) {
    use std::fmt::Write as _;
    out.clear();

    match format {
        DataFormat::Decimal => {
            let _ = write!(out, "{decimal}");
        },
        DataFormat::Hexadecimal => {
            // "XX" per byte, plus one separating space between each pair of bytes.
            out.reserve(3 * N - 1);
            for (i, b) in bytes.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                let _ = write!(out, "{b:02X}");
            }
        },
        DataFormat::Binary => {
            // "hhhh llll" per byte (nybble, space, nybble), plus one separating space between each pair of bytes.
            out.reserve(10 * N - 1);
            for (i, b) in bytes.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                let _ = write!(out, "{:04b} {:04b}", b >> 4, b & 0x0F);
            }
        },
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
#[cfg(test)]
mod unit_tests;
