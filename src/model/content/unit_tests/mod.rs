//! Internal unit tests for `model::content`: grid arithmetic, value formatting, the content's own properties,
//! selections, operator labels, text formats and element labels. None touches the DOM, so all run under a plain `cargo
//! test`.
//!
//! - [`grid_shape`] — `grid_shape` under `Automatic` and the fixed layouts, `GridLayout::is_valid`, and
//!   `best_power_of_two_rows`.
//! - [`formatting`] — hexadecimal, binary and decimal digits, `ByteOrder`, `for_each_cell_string` and
//!   `widest_cell_string`.
//! - [`content`] — the default and overridden layout, the value count, a single value, and the type colour and name.
//! - [`selection`] — `resolve_selection` (including incomplete grids), `ResolvedBand::contains`,
//!   `Selection::describe_into`, and `natural_selection`/`flat_index`.
//! - [`operators`] — operator labels and commutativity.
//! - [`text_formats`] — `DataFormat::Ascii` and `DataFormat::PlainText`.
//! - [`labelling`] — `LabellingStyle` and `with_labels`/`with_labelling_style`.

mod content;
mod formatting;
mod grid_shape;
mod labelling;
mod operators;
mod selection;
mod support;
mod text_formats;
