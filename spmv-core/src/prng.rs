//! SplitMix64 and the derivation of a named stream from a seed, as in `perf-harness` and `service-sim-engine`. It is written out
//! here so that no dependency can change what a seed means, and it uses only 64-bit integer arithmetic.

/// A SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`, without modulo bias. A draw in the short last interval of the 64-bit range is thrown away.
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

    /// A number in `lo..=hi`.
    pub fn between(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
}

/// The stream `n` of `seed`.
#[must_use]
pub fn stream(seed: u64, n: u64) -> SplitMix64 {
    let mut base = SplitMix64::new(seed ^ n.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    SplitMix64::new(base.next_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference outputs for the seed 1234567, which `perf-harness` and `service-sim-engine` fix in the same way.
    #[test]
    fn the_generator_gives_the_reference_sequence() {
        let mut prng = SplitMix64::new(1_234_567);
        let want = [
            6_457_827_717_110_365_317u64,
            3_203_168_211_198_807_973,
            9_817_491_932_198_370_423,
            4_593_380_528_125_082_431,
            16_408_922_859_458_223_821,
        ];
        for (index, expected) in want.into_iter().enumerate() {
            assert_eq!(prng.next_u64(), expected, "output {index}");
        }
    }

    #[test]
    fn streams_are_distinct_and_repeatable() {
        let (mut a, mut b, mut c) = (stream(5, 1), stream(5, 1), stream(5, 2));
        let (x, y, z) = (a.next_u64(), b.next_u64(), c.next_u64());
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn below_stays_in_range_and_between_is_inclusive() {
        let mut p = stream(9, 3);
        for _ in 0..1000 {
            assert!(p.below(7) < 7);
            let v = p.between(3, 5);
            assert!((3..=5).contains(&v));
        }
    }
}
