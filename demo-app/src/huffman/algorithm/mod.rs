//! Huffman encoding (canonical form) in plain Rust, split into the same named pieces the demo draws.
//!
//! Pass one counts byte frequencies, builds the tree by repeatedly merging the two lightest nodes, and reads a code
//! length for each byte from its depth. Canonical assignment then replaces the tree's own bit patterns, because a
//! decoder can rebuild canonical codes from the lengths alone. Pass two looks up each byte's code and packs it into the
//! output.
//!
//! # Scope
//!
//! Symbols are bytes, so non-ASCII text is encoded as its UTF-8 bytes. Input is limited to [`MAX_INPUT_LEN`] bytes.
//! That limit keeps every code well inside [`MAX_CODE_LEN`], so no length limiting is needed.

mod codes;
mod pack;
mod tree;

#[allow(unused_imports)] // `Entry` is for the walk module
pub(super) use codes::{Code, Codebook, Entry, codebook};
pub(super) use pack::{PackStep, Packed, pack};
#[allow(unused_imports)] // `Merge` is named by the tests
pub(super) use tree::{Frequency, Merge, Node, Tree, frequencies, tree};

/// The longest input accepted, in bytes.
pub(super) const MAX_INPUT_LEN: usize = 128;
/// The longest code the packer can hold, in bits.
pub(super) const MAX_CODE_LEN: u8 = 32;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything one encoding goes through, from the input text to the output bytes.
pub(super) struct Trace {
    /// How often each distinct byte occurs, in byte order.
    pub(super) frequencies: Vec<Frequency>,
    /// The tree built from those frequencies, with the merges that built it.
    pub(super) tree: Tree,
    /// The canonical code for each distinct byte.
    pub(super) codebook: Codebook,
    /// The second pass: the packed output and the step that produced each part of it.
    pub(super) packed: Packed,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes `input`, keeping every step. `None` if `input` is empty or longer than [`MAX_INPUT_LEN`].
pub(super) fn trace(input: &[u8]) -> Option<Trace> {
    if input.is_empty() || input.len() > MAX_INPUT_LEN {
        return None;
    }

    let frequencies = frequencies(input);
    let tree = tree(&frequencies);
    let codebook = codebook(&tree)?;
    let packed = pack(input, &codebook)?;
    Some(Trace {
        frequencies,
        tree,
        codebook,
        packed,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
