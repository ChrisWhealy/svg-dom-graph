use super::{MAX_CODE_LEN, Tree};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A bit sequence of 1 to [`MAX_CODE_LEN`] bits, held in the low `len` bits of `bits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Code {
    pub(in crate::huffman) bits: u32,
    pub(in crate::huffman) len: u8,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The code for one byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct Entry {
    pub(in crate::huffman) symbol: u8,
    pub(in crate::huffman) code: Code,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every byte's code, in canonical order: shortest first, and by byte value within one length.
pub(in crate::huffman) struct Codebook {
    pub(in crate::huffman) entries: Vec<Entry>,
    by_symbol: [Option<Code>; 256],
}

impl Codebook {
    /// A codebook from hand-chosen entries, so that a test can use code lengths no real input produces.
    #[cfg(test)]
    pub(in crate::huffman) fn from_entries(entries: Vec<Entry>) -> Self {
        let mut by_symbol = [None; 256];
        entries.iter().for_each(|e| by_symbol[e.symbol as usize] = Some(e.code));
        Codebook { entries, by_symbol }
    }

    pub(in crate::huffman) fn code_for(&self, symbol: u8) -> Option<Code> {
        self.by_symbol[symbol as usize]
    }
}

/// Assigns canonical codes to the lengths read from `tree`. `None` if any length exceeds [`MAX_CODE_LEN`].
///
/// The first code is all zeros. Each next code is the previous one plus 1, shifted left by however many bits longer it
/// is. A decoder can repeat this from the lengths alone, so only the lengths ever need sending.
pub(in crate::huffman) fn codebook(tree: &Tree) -> Option<Codebook> {
    let mut lengths = tree.leaf_depths();
    if lengths.iter().any(|&(_, len)| len > MAX_CODE_LEN) {
        return None;
    }
    lengths.sort_unstable_by_key(|&(symbol, len)| (len, symbol));

    let mut entries = Vec::with_capacity(lengths.len());
    let mut by_symbol = [None; 256];
    // One wider than a code, because the last code of a complete set is all ones and the step past it must not wrap.
    let mut next = 0u64;
    let mut prev_len = lengths[0].1;

    for (symbol, len) in lengths {
        next <<= len - prev_len;
        prev_len = len;

        let code = Code {
            bits: u32::try_from(next).ok()?,
            len,
        };
        entries.push(Entry { symbol, code });
        by_symbol[symbol as usize] = Some(code);
        next += 1;
    }

    Some(Codebook { entries, by_symbol })
}
