use super::{tidy, written};
use demo_common::complex::Complex;

#[test]
fn two_numbers_with_the_same_three_decimals_are_written_alike() {
    assert_eq!(written(Complex::new(0.5001, -0.5001)), written(Complex::new(0.4999, -0.4999)));
    assert_ne!(written(Complex::new(0.5, 0.0)), written(Complex::new(-0.5, 0.0)));
}

#[test]
fn a_part_that_shows_as_zero_is_a_plain_zero() {
    let tidied = tidy(Complex::new(1.0, -0.0));
    assert!(tidied.im.is_sign_positive(), "the sign of a zero is dropped");
    assert_eq!(tidy(Complex::new(-1e-17, 0.25)), Complex::new(0.0, 0.25));
    assert!(tidy(Complex::new(-1e-17, 0.25)).re.is_sign_positive());
}

#[test]
fn a_part_that_shows_as_something_is_kept() {
    assert_eq!(tidy(Complex::new(-0.001, 0.0005)), Complex::new(-0.001, 0.0005));
    let r = std::f64::consts::FRAC_1_SQRT_2;
    assert_eq!(tidy(Complex::new(r, -r)), Complex::new(r, -r));
}
