//! `Scene::add_data_node`/`add_data_node_with`: a node whose visible content is a [`DataNodeContent`] grid, not a plain
//! text label.
//!
//! - [`rendering`] — a single value fills the whole box, several values each get their own coloured cell, and every
//!   cell shares one width.
//! - [`grid_layout`] — rows and columns, an incomplete last row, `GridLayout` overrides and column groups.
//! - [`accessibility`] — the type named as text, and each cell's own row and column.
//! - [`validation`] — empty content, a zero grid layout and a blank name are rejected before anything is drawn.
//! - [`dragging_and_connectors`] — dragging every cell, and ordinary connector routing, including `EdgeAnchors`.
//! - [`named`] — `Scene::add_named_data_node`: the labelled outer box and its anchoring.
//! - [`plain_text`] — `DataFormat::PlainText`.
//! - [`cell_rect`] — `Scene::cell_rect`.

mod accessibility;
mod cell_rect;
mod dragging_and_connectors;
mod grid_layout;
mod named;
mod plain_text;
mod rendering;
mod support;
mod validation;
