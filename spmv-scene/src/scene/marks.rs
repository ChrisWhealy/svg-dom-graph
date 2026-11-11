//! What a step marks: the windows as they were, so that the cells that differ can be told, and the cells marked.

/// What a step marks: the windows as they were, so that the cells that differ can be told, and the cells marked.
#[derive(Default)]
pub(super) struct Marks {
    pub(super) prev_x: Vec<i64>,
    pub(super) prev_y: Vec<i64>,
    pub(super) changed: Vec<web_sys::Element>,
}
