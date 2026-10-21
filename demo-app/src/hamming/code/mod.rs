//! The Hamming(7,4) code in plain Rust: four data bits and three parity bits make a seven-bit codeword, from which one
//! flipped bit can be found and put right. Nothing here touches the DOM or the library.
//!
//! The bits of a codeword are numbered 1 to 7 from the left. The parity bits are at the positions that are powers of two:
//! 1, 2 and 4. The data bits fill the others: 3, 5, 6 and 7. A parity bit covers every position whose number has the same
//! bit set: position 1 (binary 001) covers 1, 3, 5, 7. Position 2 (010) covers 2, 3, 6, 7. Position 4 (100) covers 4, 5,
//! 6, 7. Each makes the number of 1s among the positions it covers even.

/// The positions, counting from 1, of the three parity bits.
pub(super) const PARITY_POSITIONS: [usize; 3] = [1, 2, 4];
/// The positions of the four data bits, in the order of the data.
pub(super) const DATA_POSITIONS: [usize; 4] = [3, 5, 6, 7];
/// How many bits a codeword has.
pub(super) const CODEWORD_LEN: usize = 7;

/// The positions that the parity bit at `PARITY_POSITIONS[check]` covers: those with that bit set in their number.
pub(super) fn covered(check: usize) -> Vec<usize> {
    let mask = PARITY_POSITIONS[check];
    (1..=CODEWORD_LEN).filter(|position| position & mask != 0).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happens to four data bits on the way from the sender to the receiver, and what the receiver makes of it.
///
/// Every bit is `0` or `1`. A codeword is indexed from 0, so position `n` is at index `n - 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Transmission {
    pub(super) data: [u8; 4],
    /// The parity bits p1, p2 and p4.
    pub(super) parity: [u8; 3],
    pub(super) codeword: [u8; CODEWORD_LEN],
    /// The position, from 1, of the bit flipped on the way, if one was.
    pub(super) error: Option<usize>,
    pub(super) received: [u8; CODEWORD_LEN],
    /// The receiver's checks c1, c2 and c4: each parity group worked out again over the bits that arrived.
    pub(super) checks: [u8; 3],
    /// The checks read as a binary number, c4 first. It is the position of the flipped bit, or 0 if none.
    pub(super) syndrome: u8,
    pub(super) corrected: [u8; CODEWORD_LEN],
    pub(super) recovered: [u8; 4],
}

impl Transmission {
    /// What arrives when `data` is sent as a codeword and the bit at position `error` is flipped on the way.
    ///
    /// An `error` that is not a position from 1 to 7 is ignored.
    pub(super) fn new(data: [u8; 4], error: Option<usize>) -> Self {
        let mut codeword = [0; CODEWORD_LEN];
        for (bit, position) in data.iter().zip(DATA_POSITIONS) {
            codeword[position - 1] = *bit;
        }
        let mut parity = [0; 3];
        for (check, bit) in parity.iter_mut().enumerate() {
            // The parity bit is still 0 in the codeword, so including its own position changes nothing.
            *bit = xor_over(&codeword, &covered(check));
            codeword[PARITY_POSITIONS[check] - 1] = *bit;
        }

        let error = error.filter(|position| (1..=CODEWORD_LEN).contains(position));
        let mut received = codeword;
        if let Some(position) = error {
            received[position - 1] ^= 1;
        }

        let mut checks = [0; 3];
        for (check, bit) in checks.iter_mut().enumerate() {
            *bit = xor_over(&received, &covered(check));
        }
        let syndrome = checks[0] | (checks[1] << 1) | (checks[2] << 2);
        let mut corrected = received;
        if syndrome != 0 {
            corrected[usize::from(syndrome) - 1] ^= 1;
        }
        let recovered = DATA_POSITIONS.map(|position| corrected[position - 1]);
        Self {
            data,
            parity,
            codeword,
            error,
            received,
            checks,
            syndrome,
            corrected,
            recovered,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The XOR of the bits of `word` at `positions`, which count from 1.
pub(super) fn xor_over(word: &[u8; CODEWORD_LEN], positions: &[usize]) -> u8 {
    positions.iter().fold(0, |sum, position| sum ^ word[position - 1])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
