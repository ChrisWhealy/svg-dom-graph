//! The 15 bits of format information: the error correction level and the mask, with a BCH code to protect them.

/// The two bits that say level L.
const LEVEL_L: u32 = 0b01;

/// The generator of the BCH code, which is x^10 + x^8 + x^5 + x^4 + x^2 + x + 1.
const GENERATOR: u32 = 0x537;

/// What the whole word is XORed with, so that it is never all zeros.
const XOR_MASK: u32 = 0x5412;

/// The number of bits in the BCH remainder, and so the shift of the five data bits above it.
const CHECK_BITS: u32 = 10;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 15 format bits for level L and `mask`, which is 0 to 7. Bit 0 is the least significant.
///
/// The five data bits are the level and then the mask. The remainder of those bits, shifted up ten places, divided by
/// the generator in GF(2) is the check, written after them. The whole word is then XORed with 0x5412.
pub(crate) fn format_word(mask: u8) -> u16 {
    let data = LEVEL_L << 3 | u32::from(mask & 0b111);
    let mut remainder = data << CHECK_BITS;
    for shift in (CHECK_BITS..CHECK_BITS + 5).rev() {
        if remainder >> shift & 1 == 1 {
            remainder ^= GENERATOR << (shift - CHECK_BITS);
        }
    }
    // The word is 15 bits, so it fits in 16.
    u16::try_from((data << CHECK_BITS | remainder) ^ XOR_MASK).unwrap_or(0)
}
