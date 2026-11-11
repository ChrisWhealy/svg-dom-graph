//! A node can be dragged a small distance with no overlap involved, via real, CDP-driven mouse input.
//!
//! Uses the fixture's `solo` node (`#diagram > g.svg-dom-graph-content > g:nth-of-type(1)`), positioned far from every
//! other node so no overlap-resolution logic can be in play. This isolates whether ordinary dragging itself works under
//! a real mouse sequence, before `overlap_resolution.rs` layers the drop-onto-another-node case on top.

use crate::common::{drag, group_translate, new_tab};
use std::time::Duration;

mod unit_tests;
