//! Internal unit tests for `model::content`: grid arithmetic, value formatting, the content's own properties,
//! selections, operator labels, text formats and element labels. None touches the DOM, so all run under a plain `cargo
//! test`.
//!
//! - [`grid_shape`] — `grid_shape` under `Automatic` and the fixed layouts, `GridLayout::is_valid`, and
//!   `best_power_of_two_rows`.
//! - [`formatting`] — hexadecimal, binary and decimal digits, `ByteOrder`, `for_each_cell_string` and
//!   `widest_cell_string`.
//! - [`floats`] — `F32`/`F64`: every `FloatFormat`, special values, IEEE-754 bits, and the width of a cell.
//! - [`wide`] — `U128`, `I128` and `Uuid`: the whole range, every `UuidFormat`, and the width of a cell.
//! - [`bit_values`] — `NodeValues::Bits`: validation, the digits of each format, and the width of a cell.
//! - [`bools_and_sizes`] — `Bool`, `Usize` and `Isize`: every `BoolFormat`, a Boolean's fixed width, and a pointer-
//!   sized integer's width.
//! - [`content`] — the default and overridden layout, the value count, a single value, and the type colour and name.
//! - [`selection`] — `resolve_selection` (including incomplete grids), `ResolvedBand::contains`,
//!   `Selection::describe_into`, and `natural_selection`/`flat_index`.
//! - [`operators`] — operator labels and commutativity.
//! - [`text_formats`] — `DataFormat::Ascii` and `DataFormat::PlainText`.
//! - [`hex_dump`] — `DataFormat::HexDump`: the lines of `hexdump -C`, the optional offset, and the cells.
//! - [`text_cells`] — `NodeValues::Text` and `DataFormat::Text`: the strings, what is valid, what is said for an empty cell,
//!   replacements, and the cell-width hint.
//! - [`labelling`] — `LabellingStyle` and `with_labels`/`with_labelling_style`.

mod bit_values;
mod bools_and_sizes;
mod content;
mod floats;
mod formatting;
mod grid_shape;
mod hex_dump;
mod labelling;
mod operators;
mod selection;
mod support;
mod text_cells;
mod text_formats;
mod wide;
