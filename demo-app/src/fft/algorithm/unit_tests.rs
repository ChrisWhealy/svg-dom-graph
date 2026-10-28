use super::{Error, FftTrace, PairTrace, Preset, SIZES, bit_reversal, first_informative_pair, pair_lanes, trace};
use demo_common::complex::Complex;

fn near(got: Complex, want: Complex, n: usize) -> bool {
    // The bound is 1e-12, scaled by the larger of 1, N and the value wanted.
    (got - want).norm() <= 1e-12 * want.norm().max(n as f64).max(1.0)
}

/// The discrete Fourier transform from its definition, `X[k] = Σ x[j]·e^(−2πijk/N)`. It shares nothing with the code
/// under test: not the roots of unity, not the butterfly. Each factor comes straight from `sin_cos`.
fn definition(input: &[Complex]) -> Vec<Complex> {
    let n = input.len();
    (0..n)
        .map(|k| {
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (j, x) in input.iter().enumerate() {
                let angle = -std::f64::consts::TAU * ((j * k) % n) as f64 / n as f64;
                let (s, c) = angle.sin_cos();
                re += x.re * c - x.im * s;
                im += x.re * s + x.im * c;
            }
            Complex::new(re, im)
        })
        .collect()
}

fn run(input: &[Complex]) -> FftTrace {
    trace(input).expect("a valid input")
}

fn last(trace: &FftTrace) -> &[Complex] {
    trace.states.last().expect("at least the input")
}

fn impulse(n: usize) -> Vec<Complex> {
    Preset::Impulse.samples(n)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_transform_matches_the_definition_for_every_preset_and_size() {
    for n in [2, 4, 8, 16, 32] {
        for preset in Preset::ALL {
            let input = preset.samples(n);
            let got = run(&input);
            let want = definition(&input);
            for (k, (g, w)) in last(&got).iter().zip(&want).enumerate() {
                assert!(near(*g, *w, n), "{preset:?}, n = {n}, bin {k}: {g:?} against {w:?}");
            }
        }
    }
}

#[test]
fn the_transform_of_an_all_zero_input_is_all_zero() {
    for n in [2, 8, 16] {
        let got = run(&vec![Complex::ZERO; n]);
        assert!(last(&got).iter().all(|z| *z == Complex::ZERO));
    }
}

#[test]
fn the_forward_transform_of_an_impulse_is_the_negative_angle_roots() {
    for n in [4, 8, 16] {
        let got = run(&impulse(n));
        for (k, z) in last(&got).iter().enumerate() {
            let angle = -std::f64::consts::TAU * k as f64 / n as f64;
            let (s, c) = angle.sin_cos();
            assert!(near(*z, Complex::new(c, s), n), "n = {n}, bin {k}");
        }
    }
}

#[test]
fn the_forward_transform_is_not_scaled() {
    // The constant 1 has all its energy in bin 0, and that bin is N, not 1.
    for n in [4, 8, 16] {
        let got = run(&Preset::Constant.samples(n));
        assert!(near(last(&got)[0], Complex::from_real(n as f64), n));
        assert!(last(&got)[1..].iter().all(|z| z.norm() < 1e-12));
    }
}

#[test]
fn the_cosine_peaks_in_bin_one_and_the_last_bin_with_half_of_n() {
    for n in [8, 16] {
        let got = run(&Preset::Cosine.samples(n));
        let peak = Complex::from_real(n as f64 / 2.0);
        for (k, z) in last(&got).iter().enumerate() {
            let want = if k == 1 || k == n - 1 { peak } else { Complex::ZERO };
            assert!(near(*z, want, n), "n = {n}, bin {k}: {z:?}");
        }
    }
}

#[test]
fn the_mixed_preset_has_every_sign_of_each_part() {
    let samples = Preset::Mixed.samples(16);
    assert!(samples.iter().any(|z| z.re > 0.0) && samples.iter().any(|z| z.re < 0.0));
    assert!(samples.iter().any(|z| z.im > 0.0) && samples.iter().any(|z| z.im < 0.0));
    assert!(samples.iter().all(|z| z.is_finite()));
}

#[test]
fn the_presets_have_the_stated_values() {
    assert_eq!(
        Preset::Impulse.samples(4),
        [Complex::ZERO, Complex::ONE, Complex::ZERO, Complex::ZERO]
    );
    assert!(Preset::Constant.samples(8).iter().all(|z| *z == Complex::ONE));
    // n = 0: 0.5·(0 − 2) − 0.25i·(0 − 3). n = 6: 0.5·(1 − 2) − 0.25i·(6 − 3).
    assert_eq!(Preset::Mixed.samples(8)[0], Complex::new(-1.0, 0.75));
    assert_eq!(Preset::Mixed.samples(8)[6], Complex::new(-0.5, -0.75));
}

#[test]
fn a_preset_is_found_again_by_its_key() {
    for preset in Preset::ALL {
        assert_eq!(Preset::from_key(preset.key()), Some(preset));
    }
    assert_eq!(Preset::from_key("zero"), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_eight_point_bit_reversal_is_the_known_permutation() {
    assert_eq!(bit_reversal(8), Ok(vec![0, 4, 2, 6, 1, 5, 3, 7]));
    assert_eq!(bit_reversal(2), Ok(vec![0, 1]));
    assert_eq!(bit_reversal(4), Ok(vec![0, 2, 1, 3]));
}

#[test]
fn the_bit_reversal_is_a_permutation_that_undoes_itself() {
    for n in [2, 4, 8, 16, 32, 64] {
        let order = bit_reversal(n).expect("a power of two");
        let mut seen = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..n).collect::<Vec<_>>(), "n = {n}");
        assert!(order.iter().enumerate().all(|(i, &j)| order[j] == i), "n = {n}");
    }
}

#[test]
fn lengths_that_cannot_be_transformed_are_refused_without_panicking() {
    for n in [0, 1, 3, 6, 12] {
        assert_eq!(bit_reversal(n), Err(Error::BadLength(n)));
        assert_eq!(trace(&vec![Complex::ONE; n]), Err(Error::BadLength(n)));
    }
}

#[test]
fn a_sample_that_is_not_finite_is_refused() {
    let mut input = impulse(8);
    input[5] = Complex::new(f64::NAN, 0.0);
    assert_eq!(trace(&input), Err(Error::NotFinite { index: 5 }));
    input[5] = Complex::new(0.0, f64::INFINITY);
    assert_eq!(trace(&input), Err(Error::NotFinite { index: 5 }));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_state_is_the_input_in_bit_reversed_order() {
    let input = Preset::Mixed.samples(8);
    let got = run(&input);
    assert_eq!(got.bit_reversed, [0, 4, 2, 6, 1, 5, 3, 7]);
    for (lane, &source) in got.bit_reversed.iter().enumerate() {
        assert_eq!(got.states[0][lane], input[source]);
    }
    assert_eq!(got.input, input);
}

#[test]
fn a_run_has_one_more_state_than_it_has_stages() {
    for (n, stages) in [(2, 1), (8, 3), (16, 4)] {
        let got = run(&Preset::Mixed.samples(n));
        assert_eq!(got.stages(), stages);
        assert_eq!(got.states.len(), stages + 1);
        assert_eq!(got.lanes(), n);
        assert!(got.states.iter().all(|state| state.len() == n));
    }
}

#[test]
fn every_stage_has_half_of_n_butterflies_that_share_no_lane() {
    for n in SIZES {
        let got = run(&Preset::Mixed.samples(n));
        for stage in 1..=got.stages() {
            let pairs = got.stage_pairs(stage);
            assert_eq!(pairs.len(), n / 2, "n = {n}, stage {stage}");
            let mut lanes: Vec<usize> = pairs.iter().flat_map(|p| [p.upper, p.lower]).collect();
            lanes.sort_unstable();
            assert_eq!(lanes, (0..n).collect::<Vec<_>>(), "n = {n}, stage {stage}");
        }
    }
}

#[test]
fn the_butterflies_are_listed_block_by_block_and_by_j_within_a_block() {
    // Stage 2 of 8: blocks of four, two butterflies each.
    assert_eq!(pair_lanes(8, 2), [(0, 2, 0, 0), (1, 3, 1, 0), (4, 6, 0, 4), (5, 7, 1, 4)]);
    // Stage 3 of 8: one block, four butterflies, 4 apart.
    assert_eq!(pair_lanes(8, 3), [(0, 4, 0, 0), (1, 5, 1, 0), (2, 6, 2, 0), (3, 7, 3, 0)]);
    // Stage 1: pairs of neighbours.
    assert_eq!(pair_lanes(8, 1), [(0, 1, 0, 0), (2, 3, 0, 2), (4, 5, 0, 4), (6, 7, 0, 6)]);
}

#[test]
fn the_recorded_butterflies_follow_the_pair_enumeration() {
    let got = run(&Preset::Mixed.samples(16));
    for stage in 1..=got.stages() {
        let lanes = pair_lanes(16, stage);
        for (pair, (upper, lower, j, start)) in got.stage_pairs(stage).iter().zip(lanes) {
            assert_eq!(
                (pair.stage, pair.upper, pair.lower, pair.j, pair.start),
                (stage, upper, lower, j, start)
            );
            assert_eq!(pair.block_width(), 1 << stage);
        }
    }
}

#[test]
fn a_butterfly_reads_the_state_before_its_stage_and_writes_the_state_after_it() {
    let got = run(&Preset::Mixed.samples(16));
    for stage in 1..=got.stages() {
        let (before, after) = (&got.states[stage - 1], &got.states[stage]);
        for pair in got.stage_pairs(stage) {
            assert_eq!(pair.a, before[pair.upper]);
            assert_eq!(pair.b, before[pair.lower]);
            assert!(near(after[pair.upper], pair.sum, 16));
            assert!(near(after[pair.lower], pair.difference, 16));
            assert!(near(pair.sum + pair.difference, pair.a.scale(2.0), 16));
        }
    }
}

#[test]
fn the_twiddle_factor_is_the_negative_angle_root_for_its_j_and_block() {
    let got = run(&impulse(16));
    for stage in 1..=got.stages() {
        for pair in got.stage_pairs(stage) {
            let m = pair.block_width();
            let (s, c) = (-std::f64::consts::TAU * pair.j as f64 / m as f64).sin_cos();
            assert!(near(pair.w, Complex::new(c, s), 16), "stage {stage}, j {}", pair.j);
            assert!(near(pair.product, pair.w * pair.b, 16));
            assert!((pair.w.norm() - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn the_twiddle_factor_depends_on_j_and_not_on_the_position_in_the_stage() {
    // Stage 2 of 8 has two blocks. Butterfly 2 (index) is the first of the second block, so its j is 0 again.
    let got = run(&impulse(8));
    let pairs = got.stage_pairs(2);
    assert_eq!(pairs.iter().map(|p| p.j).collect::<Vec<_>>(), [0, 1, 0, 1]);
    assert_eq!(pairs[2].w, Complex::ONE);
    assert!(near(pairs[3].w, Complex::new(0.0, -1.0), 8));
}

#[test]
fn stage_pairs_outside_the_run_are_empty() {
    let got = run(&impulse(8));
    assert!(got.stage_pairs(0).is_empty());
    assert!(got.stage_pairs(4).is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Appendix A of the design: the eight-point impulse, to three decimals.
#[test]
fn the_eight_point_impulse_has_the_documented_state_after_every_stage() {
    let got = run(&impulse(8));
    let real = |state: usize| got.states[state].iter().map(|z| z.re).collect::<Vec<_>>();
    assert_eq!(real(0), [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(real(1), [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]);
    assert_eq!(
        real(2).iter().map(|x| x.round()).collect::<Vec<_>>(),
        [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]
    );
    assert!(got.states[2].iter().all(|z| z.im.abs() < 1e-12));

    let r = std::f64::consts::FRAC_1_SQRT_2;
    let want = [
        Complex::new(1.0, 0.0),
        Complex::new(r, -r),
        Complex::new(0.0, -1.0),
        Complex::new(-r, -r),
        Complex::new(-1.0, 0.0),
        Complex::new(-r, r),
        Complex::new(0.0, 1.0),
        Complex::new(r, r),
    ];
    for (k, w) in want.iter().enumerate() {
        assert!(near(last(&got)[k], *w, 8), "bin {k}");
    }
}

#[test]
fn the_documented_stage_three_pair_has_the_documented_numbers() {
    let got = run(&impulse(8));
    let pair = got.stage_pairs(3)[1];
    assert_eq!((pair.start, pair.j, pair.upper, pair.lower), (0, 1, 1, 5));
    let r = std::f64::consts::FRAC_1_SQRT_2;
    assert_eq!(pair.a, Complex::ZERO);
    assert_eq!(pair.b, Complex::ONE);
    assert!(near(pair.w, Complex::new(r, -r), 8));
    assert!(near(pair.product, pair.w, 8));
    assert!(near(pair.sum, pair.w, 8));
    assert!(near(pair.difference, -pair.w, 8));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn pair_with(a: Complex, b: Complex) -> PairTrace {
    PairTrace {
        stage: 1,
        start: 0,
        j: 0,
        upper: 0,
        lower: 1,
        a,
        b,
        w: Complex::ONE,
        product: b,
        sum: a + b,
        difference: a - b,
    }
}

#[test]
fn the_default_pair_is_the_first_with_a_nonzero_input() {
    let z = Complex::ZERO;
    let one = Complex::ONE;
    let pairs = [pair_with(z, z), pair_with(z, z), pair_with(z, one), pair_with(one, one)];
    assert_eq!(first_informative_pair(&pairs), 2);
    let a_only = [pair_with(z, z), pair_with(Complex::new(0.0, -2.0), z)];
    assert_eq!(first_informative_pair(&a_only), 1);
}

#[test]
fn a_signed_zero_is_zero_when_choosing_the_default_pair() {
    let negative_zero = Complex::new(-0.0, -0.0);
    let pairs = [pair_with(negative_zero, negative_zero), pair_with(Complex::ONE, Complex::ZERO)];
    assert_eq!(first_informative_pair(&pairs), 1);
}

#[test]
fn the_default_pair_is_the_first_when_nothing_is_nonzero_or_everything_is() {
    let zeros = run(&[Complex::ZERO; 8]);
    for stage in 1..=3 {
        assert_eq!(first_informative_pair(zeros.stage_pairs(stage)), 0);
    }
    let constant = run(&Preset::Constant.samples(8));
    assert_eq!(first_informative_pair(constant.stage_pairs(1)), 0);
    assert_eq!(first_informative_pair(&[]), 0);
}

#[test]
fn the_default_pair_of_the_impulse_at_stage_one_holds_the_impulse() {
    let got = run(&impulse(8));
    let index = first_informative_pair(got.stage_pairs(1));
    assert_eq!(index, 2);
    let pair = got.stage_pairs(1)[index];
    assert_eq!((pair.upper, pair.lower), (4, 5));
}

#[test]
fn every_error_has_a_message() {
    for error in [
        Error::BadLength(3),
        Error::NotFinite { index: 1 },
        Error::Twiddles { order: 2 },
        Error::Disagreement { stage: 1, lane: 0 },
    ] {
        assert!(!error.to_string().is_empty());
    }
}
