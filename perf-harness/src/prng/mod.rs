//! The pseudo-random number generator that every workload draws from.
//!
//! It is SplitMix64, written out here and not taken from a crate, so that no dependency upgrade can change what a seed means.
//! It uses only 64-bit integer arithmetic, so it gives the same numbers on every platform, including a 32-bit one.

/// A SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A generator for one named stream of a seed, so that unrelated choices do not share a sequence.
    ///
    /// The stream number is mixed into the seed, and the first output of the result seeds the generator that is returned.
    pub fn derive(seed: u64, stream: u64) -> Self {
        let mut base = Self::new(seed ^ stream.wrapping_mul(0xD6E8_FEB8_6659_FD93));
        Self::new(base.next_u64())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A number in `0..n`, without modulo bias.
    ///
    /// Draws that fall in the short last interval of the 64-bit range are thrown away and drawn again.
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "below(0) has no answer");
        let limit = u64::MAX - (u64::MAX % n);
        loop {
            let draw = self.next_u64();
            if draw < limit {
                return draw % n;
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
