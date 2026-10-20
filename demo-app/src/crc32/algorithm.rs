//! CRC-32 as used by Ethernet, zip, gzip and PNG: the reflected polynomial `0xEDB88320`, a register that starts as all
//! ones, and a result that is the register with all its bits flipped. Worked out one byte at a time with the table of 256
//! entries, so that the demo can show each step. Nothing here touches the DOM or the library.

/// The polynomial `x^32 + x^26 + x^23 + x^22 + x^16 + x^12 + x^11 + x^10 + x^8 + x^7 + x^5 + x^4 + x^2 + x + 1`, with its
/// bits in reverse order, since the register is shifted right.
pub(super) const POLYNOMIAL: u32 = 0xEDB8_8320;

/// What the register holds before the first byte.
pub(super) const INITIAL: u32 = 0xFFFF_FFFF;

/// What the last register is XORed with to make the result.
pub(super) const FINAL_MASK: u32 = 0xFFFF_FFFF;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The register after a byte of value `byte` has been shifted through it, eight bits at a time: each time, shift right
/// by one, and if the bit that fell out was a one, XOR in the polynomial. This is the table's entry for `byte`.
const fn entry(byte: u8) -> u32 {
    let mut register = byte as u32;
    let mut bit = 0;
    while bit < 8 {
        register = if register & 1 == 1 { (register >> 1) ^ POLYNOMIAL } else { register >> 1 };
        bit += 1;
    }
    register
}

/// The table of the entry for each of the 256 values of a byte, made when the program is compiled.
pub(super) const TABLE: [u32; 256] = {
    let mut table = [0; 256];
    let mut index = 0;
    while index < 256 {
        table[index] = entry(index as u8);
        index += 1;
    }
    table
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happened when one input byte was shifted into the register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Step {
    /// The low byte of the register XORed with the input byte, which picks the entry.
    pub(super) index: u8,
    /// The table's entry at `index`.
    pub(super) entry: u32,
    /// The register afterwards: the old register shifted right by a byte, XORed with the entry.
    pub(super) register: u32,
}

/// Everything the demo shows about the CRC-32 of some bytes.
pub(super) struct Computation {
    /// One step for each input byte.
    pub(super) steps: Vec<Step>,
    /// The register after the last byte, before it is flipped. The initial value if there are no bytes.
    pub(super) register: u32,
    /// The CRC-32 of the bytes.
    pub(super) crc: u32,
}

/// Works out the CRC-32 of `bytes`, keeping every step.
pub(super) fn compute(bytes: &[u8]) -> Computation {
    let mut register = INITIAL;
    let steps = bytes
        .iter()
        .map(|&byte| {
            let index = ((register ^ u32::from(byte)) & 0xFF) as u8;
            let entry = TABLE[usize::from(index)];
            register = (register >> 8) ^ entry;
            Step { index, entry, register }
        })
        .collect();
    Computation {
        steps,
        register,
        crc: register ^ FINAL_MASK,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The CRC-32 of `bytes` worked out one bit at a time, with no table. The demo does not use it. The tests compare the
/// table with it.
#[cfg(test)]
pub(super) fn bitwise(bytes: &[u8]) -> u32 {
    let mut register = INITIAL;
    for &byte in bytes {
        register ^= u32::from(byte);
        for _ in 0..8 {
            register = if register & 1 == 1 { (register >> 1) ^ POLYNOMIAL } else { register >> 1 };
        }
    }
    register ^ FINAL_MASK
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the receiver concludes from the syndrome, which is the CRC that was sent XORed with the CRC of what arrived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Finding {
    /// The syndrome is zero: the data and the CRC agree.
    NoError,
    /// One bit of the data is wrong: bit `bit` of byte `byte`, where bit 0 is the least significant bit of the byte.
    Data { byte: usize, bit: u8 },
    /// One bit of the CRC itself was changed in transmission, and the data is right.
    Crc { bit: u8 },
    /// No single flipped bit explains the syndrome, so at least two bits are wrong and this cannot be corrected.
    Uncorrectable,
}

/// The syndrome of a single flipped bit: bit `bit` of byte `byte` of `len` bytes.
///
/// CRC is affine: the CRC of the data with one bit changed differs from the CRC of the data by a number that depends
/// only on **where** the bit is and how long the data is, and not on what the data holds. So it can be worked out from
/// data that is all zeros.
pub(super) fn syndrome_of_bit(len: usize, byte: usize, bit: u8) -> u32 {
    let zeros = vec![0_u8; len];
    let mut flipped = zeros.clone();
    flipped[byte] ^= 1 << bit;
    compute(&zeros).crc ^ compute(&flipped).crc
}

/// What `syndrome` says about `len` bytes of data that arrived with a CRC.
///
/// A single flipped bit of the data has a syndrome of its own, so the receiver can look it up and flip the bit back.
/// This works because the polynomial is chosen so that no two single-bit errors in data this short have the same
/// syndrome. A single flipped bit of the CRC itself leaves a syndrome of a single one. Anything else cannot be a single
/// flipped bit.
pub(super) fn locate(len: usize, syndrome: u32) -> Finding {
    if syndrome == 0 {
        return Finding::NoError;
    }
    if syndrome.count_ones() == 1 {
        return Finding::Crc {
            bit: syndrome.trailing_zeros() as u8,
        };
    }
    for byte in 0..len {
        for bit in 0..8 {
            if syndrome_of_bit(len, byte, bit) == syndrome {
                return Finding::Data { byte, bit };
            }
        }
    }
    Finding::Uncorrectable
}
