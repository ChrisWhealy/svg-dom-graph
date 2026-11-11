//! Row labels drawn from `LabellingStyle::Text` stay inside their node, whatever script they are in.
//!
//! A label's width is not its character count: a CJK glyph is wider than a Latin one, and a combining mark has no
//! advance at all. The node's box is sized by measuring every label as rendered, so no label should ever spill out of
//! it. This asserts that observable contract in a real browser, against the real rendered geometry, instead of repeating
//! the width-selection algorithm. Which label is widest depends on the fonts the machine has, and nothing here assumes
//! it.

use crate::common::new_tab;

/// How far a label may sit outside the node's box, in pixels, before it counts as spilling out. A rendered bounding box
/// is rounded to the browser's own layout units, so an exact comparison would fail on rounding alone.
const TOLERANCE: f64 = 0.5;

mod unit_tests;
