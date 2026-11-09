use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first outputs of SplitMix64 for the seed 1234567, as published for the reference algorithm. A second
/// implementation (Python) gave the same five numbers, so the constants are not just this code agreeing with itself.
#[test]
fn the_generator_gives_the_reference_sequence() -> Result<(), String> {
    let mut prng = SplitMix64::new(1234567);
    let expected = [
        6457827717110365317u64,
        3203168211198807973,
        9817491932198370423,
        4593380528125082431,
        16408922859458223821,
    ];
    for (index, want) in expected.into_iter().enumerate() {
        let got = prng.next_u64();
        if got != want {
            return Err(format!("output {index}: expected {want}, got {got}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn derived_streams_have_fixed_first_outputs_and_differ_from_each_other() -> Result<(), String> {
    let cases = [
        (42u64, 0u64, 6332618229526065668u64),
        (42, 1, 7575206361173662766),
        (0, 0x1000, 18400474137010943219),
    ];
    for (seed, stream, want) in cases {
        let got = SplitMix64::derive(seed, stream).next_u64();
        if got != want {
            return Err(format!("derive({seed}, {stream}): expected {want}, got {got}"));
        }
    }
    let mut first = SplitMix64::derive(7, 0);
    let mut second = SplitMix64::derive(7, 1);
    if first.next_u64() == second.next_u64() {
        return Err("two streams of one seed began alike".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn below_stays_in_range_and_reaches_every_value() -> Result<(), String> {
    let mut prng = SplitMix64::new(99);
    let mut seen = [false; 7];
    for _ in 0..1000 {
        let value = prng.below(7);
        if value >= 7 {
            return Err(format!("below(7) gave {value}"));
        }
        seen[value as usize] = true;
    }
    if seen.contains(&false) {
        return Err(format!("some value of 0..7 never came up: {seen:?}"));
    }
    if SplitMix64::new(1).below(1) != 0 {
        return Err("below(1) must be 0".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With n = 2^63 + 1 about half of all draws fall in the discarded tail, so this exercises the redraw.
#[test]
fn below_throws_away_the_biased_tail_and_draws_again() -> Result<(), String> {
    let n = (1u64 << 63) + 1;
    let mut under_test = SplitMix64::new(5);
    let mut reference = SplitMix64::new(5);
    let limit = u64::MAX - (u64::MAX % n);
    for _ in 0..200 {
        let mut expected = None;
        while expected.is_none() {
            let draw = reference.next_u64();
            if draw < limit {
                expected = Some(draw % n);
            }
        }
        let got = under_test.below(n);
        if Some(got) != expected {
            return Err(format!("expected {expected:?}, got {got}"));
        }
    }
    Ok(())
}
