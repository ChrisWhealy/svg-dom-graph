//! The pseudo-random number generator that every draw of a scenario comes from.
//!
//! It is SplitMix64 with the harness's derivation of a named stream from a seed, and it is written out here, as it is in
//! `perf-harness`, so that no dependency can change what a seed means. It uses only 64-bit integer arithmetic, so it gives
//! the same numbers on a 32-bit platform.

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

    /// The state, which is all that a checkpoint needs to carry the generator on.
    #[must_use]
    pub fn state(&self) -> u64 {
        self.state
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

/// The stream `n` of `seed`: the stream number is mixed into the seed, and the first output of that seeds the generator.
#[must_use]
pub fn stream(seed: u64, n: u64) -> SplitMix64 {
    let mut base = SplitMix64::new(seed ^ n.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    SplitMix64::new(base.next_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first outputs for the seed 1234567, as published for the reference algorithm, and as `perf-harness` fixes them.
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
    fn derived_streams_have_the_first_outputs_that_perf_harness_fixes() {
        for (seed, n, want) in [
            (42u64, 0u64, 6_332_618_229_526_065_668u64),
            (42, 1, 7_575_206_361_173_662_766),
            (0, 0x1000, 18_400_474_137_010_943_219),
        ] {
            assert_eq!(stream(seed, n).next_u64(), want, "stream {n} of seed {seed}");
        }
    }

    #[test]
    fn below_stays_in_range_and_between_is_inclusive() {
        let mut prng = stream(9, 3);
        let mut seen = [false; 5];
        for _ in 0..500 {
            let v = prng.between(3, 7);
            assert!((3..=7).contains(&v));
            seen[usize::try_from(v - 3).unwrap()] = true;
        }
        assert!(seen.iter().all(|s| *s), "every value of 3..=7 turns up");
    }
}
