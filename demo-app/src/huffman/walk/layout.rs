use super::{super::algorithm::Node, leaf_label};

/// The length of a connector's arrowhead, as the library draws it.
const ARROW_LENGTH: f64 = 10.0;
/// The clear space kept beyond an arrowhead, so its tip does not touch the box it points at.
const ARROW_CLEARANCE: f64 = 6.0;

/// The vertical gap one elbow connector needs between the two boxes it joins.
///
/// An elbow leaves a box, runs down half the gap, crosses sideways, and runs down the other half. That second half
/// holds the arrowhead and its clearance, and the first half is kept as long so the bend sits midway. So the gap is
/// twice the arrowhead plus its clearance.
pub(in crate::huffman) const LINK_GAP: f64 = 2.0 * (ARROW_LENGTH + ARROW_CLEARANCE);

/// The heights of the two kinds of box in a tree, which the browser measures, and what they make of the space between
/// one level and the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::huffman) struct Spacing {
    /// The height of a leaf or a branch.
    pub(in crate::huffman) box_height: f64,
    /// The height of the small box that labels a link `0` or `1`.
    pub(in crate::huffman) bit_height: f64,
}

impl Spacing {
    /// How far one level of the tree sits above the next, from the top of a parent to the top of its child.
    ///
    /// A link is drawn as two connectors, parent to bit label and bit label to child. Each needs a [`LINK_GAP`] of its
    /// own, so the gap between a parent and its child holds both of them and the label between.
    pub(in crate::huffman) fn level(&self) -> f64 {
        self.box_height + LINK_GAP + self.bit_height + LINK_GAP
    }
}

/// The width of one character of a box's text, with some to spare, and the space left on either side of the text. A
/// data node's box is its text plus 6 on each side. The scene centres each box on its measured width, so this only
/// decides how far apart the boxes are laid out.
const CHAR_WIDTH: f64 = 8.2;
const TEXT_PADDING: f64 = 16.0;
/// The narrowest a box can be.
const MIN_WIDTH: f64 = 36.0;
/// The space between one leaf and the next within a tree.
const LEAF_GAP: f64 = 8.0;
/// The extra space between one tree and the next.
const TREE_GAP: f64 = 22.0;

/// Where one node of the forest goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::huffman) struct Placed {
    /// The node's index in the tree's arena.
    pub(in crate::huffman) node: usize,
    /// The horizontal middle of the node, from the forest's left edge.
    pub(in crate::huffman) x: f64,
    /// The width of the node's box.
    pub(in crate::huffman) width: f64,
    /// How many levels the node sits above the leaves: 0 for a leaf, and one more than its taller child otherwise.
    pub(in crate::huffman) height: usize,
}

impl Placed {
    /// The top edge of the node's box, when the leaves' tops are on the line `leaf_y`, and `level` apart.
    pub(in crate::huffman) fn top(&self, leaf_y: f64, level: f64) -> f64 {
        leaf_y - self.height as f64 * level
    }
}

/// The width of `node`'s box: as wide as its text needs. A leaf reads as [`leaf_label`] does, and a branch as its
/// weight.
fn width_of(node: &Node) -> f64 {
    let chars = match *node {
        Node::Leaf { symbol, weight } => leaf_label(symbol, weight).chars().count(),
        Node::Branch { weight, .. } => weight.to_string().len(),
    };
    (chars as f64 * CHAR_WIDTH + TEXT_PADDING).max(MIN_WIDTH)
}

/// Places every node under `roots`, left to right, and returns them with the forest's total width.
///
/// The leaves are laid out in a row in the order a left-to-right walk meets them, and each branch is centred over its
/// two children. Every tree starts after the one before it, with a wider gap between trees than between leaves.
pub(in crate::huffman) fn layout(nodes: &[Node], roots: &[usize]) -> (Vec<Placed>, f64) {
    let mut placed = Vec::new();
    let mut next = 0.0;

    for (i, &root) in roots.iter().enumerate() {
        if i > 0 {
            next += TREE_GAP - LEAF_GAP;
        }
        place(nodes, root, &mut next, &mut placed);
    }
    (placed, (next - LEAF_GAP).max(0.0))
}

/// Places `index` and everything below it, and returns its horizontal middle and height.
fn place(nodes: &[Node], index: usize, next: &mut f64, placed: &mut Vec<Placed>) -> (f64, usize) {
    let width = width_of(&nodes[index]);
    let (x, height) = match nodes[index] {
        Node::Leaf { .. } => {
            let x = *next + width / 2.0;
            *next += width + LEAF_GAP;
            (x, 0)
        },
        Node::Branch { left, right, .. } => {
            let (left_x, left_height) = place(nodes, left, next, placed);
            let (right_x, right_height) = place(nodes, right, next, placed);
            ((left_x + right_x) / 2.0, left_height.max(right_height) + 1)
        },
    };
    placed.push(Placed { node: index, x, width, height });
    (x, height)
}
