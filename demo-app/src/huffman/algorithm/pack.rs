use super::{Code, Codebook, MAX_CODE_LEN};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happened when one byte's code was appended to the accumulator.
///
/// The accumulator holds only the pending bits, which are those not yet written out. Between steps there are always
/// fewer than 8 of them. The code itself can be much longer than 8 bits, so one step can complete several bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::huffman) struct PackStep {
    /// The input byte.
    pub(in crate::huffman) symbol: u8,
    /// Its code.
    pub(in crate::huffman) code: Code,
    /// The pending bits before the append, right-aligned.
    pub(in crate::huffman) acc_before: u64,
    /// How many of those bits are valid.
    pub(in crate::huffman) nbits_before: u8,
    /// The whole bytes completed by this append, in output order. Empty when the code fits in the pending bits.
    pub(in crate::huffman) emitted: Vec<u8>,
    /// The pending bits after the completed bytes are removed, right-aligned.
    pub(in crate::huffman) acc_after: u64,
    /// How many of those bits are valid.
    pub(in crate::huffman) nbits_after: u8,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The finished output of the second pass.
pub(in crate::huffman) struct Packed {
    /// One step per input byte.
    pub(in crate::huffman) steps: Vec<PackStep>,
    /// The output. Its last byte is padded with zeros if the bit total is not a multiple of 8.
    pub(in crate::huffman) bytes: Vec<u8>,
    /// The number of meaningful bits, so a decoder can ignore the padding.
    pub(in crate::huffman) bit_len: usize,
}

impl Packed {
    /// Zero bits added to the last byte, 0 to 7.
    pub(in crate::huffman) fn padding(&self) -> u8 {
        (self.bytes.len() * 8 - self.bit_len) as u8
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replaces each byte of `input` with its code and packs the codes most significant bit first.
///
/// Appending is a shift and an OR, not a loop over bits. Draining whole bytes keeps at most 7 bits pending, and a code
/// of up to [`MAX_CODE_LEN`] bits then still fits the 64-bit accumulator. `None` if a byte has no code.
pub(in crate::huffman) fn pack(input: &[u8], codebook: &Codebook) -> Option<Packed> {
    let mut acc = 0u64;
    let mut nbits = 0u8;
    let mut bytes = Vec::new();
    let mut steps = Vec::with_capacity(input.len());
    let mut bit_len = 0usize;

    for &symbol in input {
        let code = codebook.code_for(symbol)?;
        let (acc_before, nbits_before) = (acc, nbits);

        acc = acc << code.len | u64::from(code.bits);
        nbits += code.len;
        bit_len += usize::from(code.len);

        let mut emitted = Vec::new();
        while nbits >= 8 {
            nbits -= 8;
            emitted.push((acc >> nbits) as u8);
        }
        acc &= (1u64 << nbits) - 1;
        bytes.extend_from_slice(&emitted);

        steps.push(PackStep {
            symbol,
            code,
            acc_before,
            nbits_before,
            emitted,
            acc_after: acc,
            nbits_after: nbits,
        });
    }

    if nbits > 0 {
        bytes.push((acc << (8 - nbits)) as u8);
    }
    Some(Packed { steps, bytes, bit_len })
}

const _: () = assert!(
    MAX_CODE_LEN as u32 + 7 <= u64::BITS,
    "a full code plus 7 pending bits must fit the accumulator"
);
