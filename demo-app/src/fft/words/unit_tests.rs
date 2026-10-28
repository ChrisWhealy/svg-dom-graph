use super::{complex, inspector_title, needs_repair, pair_chosen, pair_option, ready, rolled_back, stepped, twiddle};
use crate::fft::algorithm::{Preset, trace};
use demo_common::complex::Complex;

fn impulse_pairs(stage: usize) -> Vec<crate::fft::algorithm::PairTrace> {
    trace(&Preset::Impulse.samples(8)).expect("valid").stage_pairs(stage).to_vec()
}

#[test]
fn a_complex_number_is_written_in_its_shortest_correct_form() {
    let r = std::f64::consts::FRAC_1_SQRT_2;
    assert_eq!(complex(Complex::ONE), "1");
    assert_eq!(complex(Complex::new(-1.0, 0.0)), "\u{2212}1");
    assert_eq!(complex(Complex::I), "i");
    assert_eq!(complex(Complex::new(0.0, -1.0)), "\u{2212}i");
    assert_eq!(complex(Complex::new(0.0, 0.5)), "0.5i");
    assert_eq!(complex(Complex::new(r, -r)), "0.707 \u{2212} 0.707i");
    assert_eq!(complex(Complex::new(-r, r)), "\u{2212}0.707 + 0.707i");
    assert_eq!(complex(Complex::new(2.0, 1.0)), "2 + i");
    assert_eq!(complex(Complex::ZERO), "0");
}

#[test]
fn a_part_that_rounds_to_zero_is_zero_and_never_minus_zero() {
    assert_eq!(complex(Complex::new(-1e-17, 1.0)), "i");
    assert_eq!(complex(Complex::new(1.0, -1e-17)), "1");
    assert_eq!(complex(Complex::new(-0.0, -0.0)), "0");
}

#[test]
fn the_twiddle_names_its_block_width_and_exponent() {
    let pairs = impulse_pairs(2);
    assert_eq!(twiddle(&pairs[0]), "W4^0 = 1");
    assert_eq!(twiddle(&pairs[1]), "W4^1 = \u{2212}i");
}

#[test]
fn the_title_previews_stage_one_before_the_first_stage() {
    assert_eq!(inspector_title(0, 3, 3, 4), "Preview: stage 1, pair 3");
    assert_eq!(inspector_title(2, 3, 3, 4), "Stage 2 of 3, pair 3 of 4");
}

#[test]
fn the_documented_unstarted_announcement_is_produced() {
    let pairs = impulse_pairs(1);
    assert_eq!(
        ready(8, 3, &pairs[2]),
        "FFT ready: N=8 samples of the summed waves. No stages completed. Previewing stage 1, pair 3, lanes 4 and 5."
    );
}

#[test]
fn the_documented_stage_announcement_is_produced() {
    let pairs = impulse_pairs(2);
    assert_eq!(
        stepped(2, 3, 3, 4, &pairs[2]),
        "Stage 2 of 3 complete; block width 4; pair 3 of 4; lanes 4 and 6; twiddle W4^0 = 1."
    );
}

#[test]
fn the_last_stage_says_that_it_is_the_transform() {
    let pairs = impulse_pairs(3);
    assert!(stepped(3, 3, 1, 4, &pairs[0]).contains("This is the transform"));
    assert!(!stepped(2, 3, 1, 4, &pairs[0]).contains("This is the transform"));
}

#[test]
fn a_pair_change_gives_the_numbers_of_the_butterfly() {
    let pairs = impulse_pairs(3);
    let said = pair_chosen(3, 2, 4, &pairs[1]);
    assert!(said.starts_with("Stage 3, pair 2 of 4: lanes 1 and 5;"));
    assert!(said.contains("W8^1 = 0.707 \u{2212} 0.707i"));
    assert!(said.contains("a = 0, b = 1,"));
    assert!(said.contains("sum = 0.707 \u{2212} 0.707i, difference = \u{2212}0.707 + 0.707i."));
    assert!(pair_chosen(0, 3, 4, &pairs[1]).starts_with("Preview of stage 1, pair 3"));
}

#[test]
fn an_option_of_the_pair_selector_is_numbered_from_one_and_names_its_lanes() {
    let pairs = impulse_pairs(3);
    assert_eq!(pair_option(2, &pairs[1]), "Pair 2 \u{00B7} lanes 1, 5");
}

#[test]
fn the_failure_banners_carry_the_error() {
    assert!(rolled_back("no room").ends_with("no room"));
    assert!(needs_repair("no room").ends_with("no room"));
}
