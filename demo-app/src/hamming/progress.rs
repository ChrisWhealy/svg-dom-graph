//! Showing how far the walk has got: the boxes it has not reached are drawn faint, and the box it has just reached is
//! marked. The bit that was flipped, and the bit that was put right, are marked too.

use super::{
    code::{CODEWORD_LEN, Transmission},
    walk::{newest, reached},
};
use crate::util::{all_if, mark_unreached, stringify};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, Selection},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many values the box at `index` of the path holds.
fn cells(index: usize) -> usize {
    match index {
        0 | 8 => 4,
        1 | 4 => 3,
        2 | 3 | 7 => CODEWORD_LEN,
        _ => 1,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the boxes of the path `nodes` that `position` has not reached faint, and marks the one it has just reached. A
/// box keeps its bits while it is faint, so a reader can see what is coming.
///
/// # Errors
///
/// Returns `Err` if a library call fails.
pub(super) fn show(scene: &Scene, nodes: &[NodeId], t: &Transmission, position: usize) -> Result<(), String> {
    for (index, &node) in nodes.iter().enumerate() {
        mark_unreached(scene, node, &all_if(!reached(index, position), cells(index)))?;
        // The whole of the newest box is marked, whether it holds one bit or several.
        let selection = match (newest(position) == Some(index), cells(index)) {
            (false, _) => Selection::None,
            (true, 1) => Selection::Cell(0),
            (true, _) => Selection::Row { row: 0, col: None },
        };
        scene.set_selection(node, selection).map_err(stringify)?;
    }
    // The bit that was flipped on the way, and the bit that the receiver flipped back. A highlighted cell is drawn at
    // full strength even in a box that is faint, so each is marked only once its box has been reached.
    if let Some(error) = t.error.filter(|_| reached(3, position)) {
        scene.set_secondary_selection(nodes[3], &[error - 1]).map_err(stringify)?;
    }
    if t.syndrome != 0 && reached(7, position) {
        scene
            .set_secondary_selection(nodes[7], &[usize::from(t.syndrome) - 1])
            .map_err(stringify)?;
    }
    Ok(())
}
