//! `Scene::add_data_node`/`add_data_node_with`: a node whose visible content is a [`DataNodeContent`] grid, not a plain
//! text label.
//!
//! - [`rendering`] — a single value fills the whole box, several values each get their own coloured cell, and every
//!   cell shares one width.
//! - [`grid_layout`] — rows and columns, an incomplete last row, `GridLayout` overrides and column groups.
//! - [`accessibility`] — the type named as text, and each cell's own row and column.
//! - [`floats`] — `NodeValues::F32`/`F64` and `DataFormat::Float`: decimal places, significant digits, scientific
//!   notation, special values, IEEE-754 bits, and cells wide enough for the widest value.
//! - [`signed`] — `NodeValues::I8`/`I16`/`I32`/`I64`: a sign only when negative, two's complement bytes, and a type of
//!   their own.
//! - [`validation`] — empty content, a zero grid layout and a blank name are rejected before anything is drawn.
//! - [`dragging_and_connectors`] — dragging every cell, and ordinary connector routing, including `EdgeAnchors`.
//! - [`named`] — `Scene::add_named_data_node`: the labelled outer box and its anchoring.
//! - [`plain_text`] — `DataFormat::PlainText`.
//! - [`cell_rect`] — `Scene::cell_rect`.

mod accessibility;
mod cell_rect;
mod dragging_and_connectors;
mod floats;
mod grid_layout;
mod named;
mod plain_text;
mod rendering;
mod signed;
mod support;
mod validation;
