use std::{cmp::Reverse, collections::BinaryHeap};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One distinct byte and how many times it occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Frequency {
    pub(in crate::huffman) symbol: u8,
    pub(in crate::huffman) count: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Counts each distinct byte in `input`, in byte order.
pub(in crate::huffman) fn frequencies(input: &[u8]) -> Vec<Frequency> {
    let mut counts = [0u32; 256];
    input.iter().for_each(|&b| counts[b as usize] += 1);

    counts
        .iter()
        .enumerate()
        .filter(|&(_, &count)| count > 0)
        .map(|(symbol, &count)| Frequency { symbol: symbol as u8, count })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::huffman) enum Node {
    /// A byte, with its count.
    Leaf { symbol: u8, weight: u32 },
    /// Two subtrees, named by their index in [`Tree::nodes`]. `weight` is the sum of theirs.
    Branch { left: usize, right: usize, weight: u32 },
}

impl Node {
    pub(in crate::huffman) fn weight(&self) -> u32 {
        match *self {
            Node::Leaf { weight, .. } | Node::Branch { weight, .. } => weight,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of tree building: the two lightest nodes are removed and replaced by their parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Merge {
    /// The lighter of the two, which becomes the left child.
    pub(in crate::huffman) left: usize,
    /// The heavier of the two, which becomes the right child.
    pub(in crate::huffman) right: usize,
    /// The new node.
    pub(in crate::huffman) parent: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The finished tree, held as an arena so that a merge can name nodes by index.
///
/// The leaves come first, in byte order. Each merge then appends one branch, so the root is always the last node.
pub(in crate::huffman) struct Tree {
    pub(in crate::huffman) nodes: Vec<Node>,
    /// The merges in the order they happened. Empty when only one distinct byte occurs.
    pub(in crate::huffman) merges: Vec<Merge>,
}

impl Tree {
    pub(in crate::huffman) fn root(&self) -> usize {
        self.nodes.len() - 1
    }

    /// The depth of each leaf, as `(symbol, depth)` in byte order.
    ///
    /// A tree with a single leaf has depth 0, which would give that byte an empty code. It is given length 1 instead.
    pub(in crate::huffman) fn leaf_depths(&self) -> Vec<(u8, u8)> {
        let mut depths = Vec::new();
        let mut pending = vec![(self.root(), 0u8)];

        while let Some((index, depth)) = pending.pop() {
            match self.nodes[index] {
                Node::Leaf { symbol, .. } => depths.push((symbol, depth.max(1))),
                Node::Branch { left, right, .. } => {
                    pending.push((left, depth + 1));
                    pending.push((right, depth + 1));
                },
            }
        }

        depths.sort_unstable();
        depths
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the tree by repeatedly merging the two lightest nodes.
///
/// Ties on weight go to the node created first, so the same input always gives the same tree. `frequencies` must not be
/// empty.
pub(in crate::huffman) fn tree(frequencies: &[Frequency]) -> Tree {
    let mut nodes: Vec<Node> = frequencies
        .iter()
        .map(|f| Node::Leaf {
            symbol: f.symbol,
            weight: f.count,
        })
        .collect();
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> =
        nodes.iter().enumerate().map(|(i, n)| Reverse((n.weight(), i))).collect();
    let mut merges = Vec::new();

    while heap.len() > 1 {
        let Reverse((w_left, left)) = heap.pop().expect("two or more entries");
        let Reverse((w_right, right)) = heap.pop().expect("two or more entries");

        let parent = nodes.len();
        nodes.push(Node::Branch {
            left,
            right,
            weight: w_left + w_right,
        });
        heap.push(Reverse((w_left + w_right, parent)));
        merges.push(Merge { left, right, parent });
    }

    Tree { nodes, merges }
}
