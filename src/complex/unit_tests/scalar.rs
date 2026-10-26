//! The scalar tests, written once and run for both precisions.

macro_rules! scalar_tests {
    ($module:ident, $name:ident, $float:ident, $tol:expr) => {
        mod $module {
            use crate::complex::{RootOfUnityError, $name};
            use std::mem::{align_of, size_of};

            fn root(k: usize, n: usize) -> $name {
                $name::unit_root(k, n).expect("a root of a non-zero order")
            }

            fn roots(n: usize, count: usize) -> Vec<$name> {
                $name::unit_roots(n, count).expect("roots of a non-zero order")
            }

            fn close(a: $name, b: $name) -> bool {
                (a - b).norm() <= $tol
            }

            #[test]
            fn a_complex_number_is_two_packed_floats() {
                assert_eq!(size_of::<$name>(), 2 * size_of::<$float>());
                assert_eq!(align_of::<$name>(), align_of::<$float>());

                let numbers = [$name::new(1.0, 2.0), $name::new(3.0, 4.0)];
                // SAFETY: `$name` is `repr(C)` with two `$float` fields and no padding, so two of them are four
                // `$float`s.
                let flat: &[$float] = unsafe { std::slice::from_raw_parts(numbers.as_ptr().cast(), 4) };
                assert_eq!(flat, &[1.0, 2.0, 3.0, 4.0]);
            }

            #[test]
            fn addition_subtraction_and_negation() {
                let (a, b) = ($name::new(1.0, 2.0), $name::new(-3.0, 0.5));

                assert_eq!(a + b, $name::new(-2.0, 2.5));
                assert_eq!(a - b, $name::new(4.0, 1.5));
                assert_eq!(-a, $name::new(-1.0, -2.0));
                assert_eq!(a.sum_and_difference(b), (a + b, a - b));
            }

            #[test]
            fn multiplication_follows_i_squared_is_minus_one() {
                assert_eq!($name::I * $name::I, $name::new(-1.0, 0.0));
                assert_eq!($name::new(1.0, 2.0) * $name::new(3.0, 4.0), $name::new(-5.0, 10.0));
                assert_eq!($name::new(1.0, 2.0) * $name::ONE, $name::new(1.0, 2.0));
            }

            #[test]
            fn scaling_by_a_real_matches_the_full_product() {
                let z = $name::new(1.5, -2.5);

                assert_eq!(z * 2.0, z * $name::from_real(2.0));
                assert_eq!(2.0 * z, z * 2.0);
                assert_eq!(z / 2.0, $name::new(0.75, -1.25));
            }

            #[test]
            fn a_quarter_turn_needs_no_multiplication_and_matches_the_product() {
                let z = $name::new(3.0, -4.0);

                assert_eq!(z.mul_i(), z * $name::I);
                assert_eq!(z.mul_neg_i(), z * -$name::I);
                assert_eq!(z.mul_i().mul_neg_i(), z);
            }

            #[test]
            fn division_undoes_multiplication() {
                let (a, b) = ($name::new(1.0, 2.0), $name::new(3.0, -4.0));

                assert!(close(a * b / b, a));
                // Both branches of Smith's algorithm: the divisor's larger part is the real one, then the imaginary
                // one.
                assert!(close(a / $name::new(5.0, 1.0) * $name::new(5.0, 1.0), a));
                assert!(close(a / $name::new(1.0, 5.0) * $name::new(1.0, 5.0), a));
            }

            #[test]
            fn division_does_not_overflow_when_the_plain_formula_would() {
                let big = $name::new($float::MAX / 2.0, $float::MAX / 2.0);
                let quotient = big / big;

                assert!(quotient.is_finite());
                assert!(close(quotient, $name::ONE));
            }

            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
            //   Division at the ends of the range
            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

            const MAX: $float = $float::MAX;
            /// The smallest positive number there is, which is subnormal.
            fn smallest() -> $float {
                $float::from_bits(1)
            }

            #[test]
            fn a_number_divided_by_itself_is_one_at_every_size_including_the_largest() {
                for factor in [1.0, 0.9, 0.5, 0.25] {
                    let big = MAX * factor;
                    let z = $name::new(big, big);

                    assert_eq!(z / z, $name::ONE, "factor {factor}");
                    assert_eq!($name::new(big, 0.0) / $name::new(big, 0.0), $name::ONE);
                    assert_eq!($name::new(0.0, big) / $name::new(0.0, big), $name::ONE);
                    assert_eq!($name::new(big, -big) / $name::new(big, -big), $name::ONE);
                }
            }

            #[test]
            fn a_number_divided_by_itself_is_one_at_the_smallest_sizes() {
                for tiny in [
                    smallest(),
                    smallest() * 3.0,
                    $float::MIN_POSITIVE / 2.0,
                    $float::MIN_POSITIVE,
                    $float::MIN_POSITIVE * 3.0,
                ] {
                    let z = $name::new(tiny, tiny);

                    assert_eq!(z / z, $name::ONE, "tiny {tiny:e}");
                    assert_eq!($name::new(tiny, 0.0) / $name::new(tiny, 0.0), $name::ONE);
                    assert_eq!($name::new(0.0, tiny) / $name::new(0.0, tiny), $name::ONE);
                }
            }

            #[test]
            fn smiths_algorithm_alone_overflows_where_the_division_does_not() {
                // This is the case that the scaling is for: both parts and the divisor near the largest number. The
                // denominator and the numerator both overflow to infinity, and infinity over infinity is `NaN`.
                let big = MAX * 0.9;
                let alone = $name::divide_smith(big, big, big, big);

                assert!(alone.re.is_nan(), "Smith's algorithm alone gave {alone:?}");
                assert_eq!($name::new(big, big) / $name::new(big, big), $name::ONE);
            }

            #[test]
            fn the_quotient_of_large_numbers_that_is_large_is_found_exactly() {
                // (MAX + MAX i) / (1 + i) is MAX, because the sum of two MAXes over two is MAX. The sum overflows, and the
                // quotient does not.
                let quotient = $name::new(MAX, MAX) / $name::new(1.0, 1.0);

                assert_eq!(quotient, $name::new(MAX, 0.0));
            }

            #[test]
            fn the_quotient_of_large_numbers_with_different_parts_is_right() {
                // (1 + 0.5i) / (0.5 + i) = 0.8 - 0.6i, whatever size both are.
                let quotient = $name::new(MAX, MAX / 2.0) / $name::new(MAX / 2.0, MAX);

                assert!(close(quotient, $name::new(0.8, -0.6)), "{quotient}");
                let tiny = smallest() * 1024.0;
                let quotient = $name::new(tiny, tiny / 2.0) / $name::new(tiny / 2.0, tiny);
                assert!(close(quotient, $name::new(0.8, -0.6)), "{quotient}");
            }

            #[test]
            fn a_quotient_that_is_too_large_is_infinite_and_one_that_is_too_small_is_zero_and_neither_is_nan() {
                let large = $name::new(MAX, 0.0) / $name::new(smallest(), 0.0);
                assert!(large.re.is_infinite() && large.im == 0.0, "{large:?}");

                let small = $name::new(smallest(), 0.0) / $name::new(MAX, 0.0);
                assert!(small.re == 0.0 && small.im == 0.0, "{small:?}");

                let one_over_max = $name::ONE / $name::new(MAX, 0.0);
                assert!((one_over_max.re - 1.0 / MAX).abs() <= smallest(), "{one_over_max:?}");
            }

            #[test]
            fn a_subnormal_numerator_is_divided_without_losing_it() {
                assert_eq!($name::new(smallest(), 0.0) / $name::ONE, $name::new(smallest(), 0.0));
                assert_eq!($name::new(smallest(), smallest()) / $name::new(2.0, 0.0), $name::new(0.0, 0.0));
                let quotient = $name::new(smallest() * 8.0, 0.0) / $name::new(2.0, 0.0);
                assert_eq!(quotient, $name::new(smallest() * 4.0, 0.0));
            }

            #[test]
            fn division_agrees_with_the_exact_answer_for_every_pair_of_sizes_from_one_end_of_the_range_to_the_other() {
                let edge = $float::MAX_EXP - 4;
                let (a0, b0) = ($name::new(1.1, -0.7), $name::new(0.9, 1.3));
                // The exact quotient of the two small numbers, which is then scaled by the difference of the sizes.
                let exact = a0 / b0;

                for ka in [-edge, -edge / 2, 0, edge / 2, edge] {
                    for kb in [-edge, -edge / 2, 0, edge / 2, edge] {
                        let (a, b) = (
                            $name::new($name::scaled_by_pow2(a0.re, ka), $name::scaled_by_pow2(a0.im, ka)),
                            $name::new($name::scaled_by_pow2(b0.re, kb), $name::scaled_by_pow2(b0.im, kb)),
                        );
                        let got = a / b;
                        let expected = $name::new(
                            $name::scaled_by_pow2(exact.re, ka - kb),
                            $name::scaled_by_pow2(exact.im, ka - kb),
                        );

                        for (got, expected) in [(got.re, expected.re), (got.im, expected.im)] {
                            assert!(!got.is_nan(), "NaN for sizes {ka} and {kb}");
                            if expected.is_finite() {
                                let allowed = (expected.abs() * $tol).max(smallest() * 4.0);
                                assert!(
                                    (got - expected).abs() <= allowed,
                                    "sizes {ka}, {kb}: {got:e} against {expected:e}"
                                );
                            } else {
                                assert_eq!(got, expected, "sizes {ka}, {kb}");
                            }
                        }
                    }
                }
            }

            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
            //   Parts of very different sizes
            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

            /// The exponent of the smallest number there is.
            fn smallest_exponent() -> i32 {
                $float::MIN_EXP - $float::MANTISSA_DIGITS as i32
            }

            fn power(exponent: i32) -> $float {
                $name::scaled_by_pow2(1.0, exponent)
            }

            /// `value` is `2^exponent` to the precision of the tests.
            fn is_power(value: $float, exponent: i32) -> bool {
                let expected = power(exponent);
                (value - expected).abs() <= expected * $tol
            }

            #[test]
            fn a_part_that_is_tiny_beside_another_does_not_make_the_quotient_zero() {
                // (2^eb * i) / (2^ec + 2^smallest * i). The real part of the quotient is `b d / (c^2 + d^2)`, which is
                // `2^(eb + smallest - 2 ec)`, and the imaginary part is `b c / (c^2 + d^2)`, which is `2^(eb - ec)`.
                // Smith's algorithm multiplies `b` by the ratio `d / c`, and that product is `2^(eb + smallest - ec)`,
                // which is below the smallest number there is, so it was zero. Every part is in the band that was thought
                // comfortable, which was judged by the larger part of each number only.
                let (eb, ec) = if $float::MAX_EXP > 200 { (-511, -500) } else { (-63, -60) };
                let quotient = $name::new(0.0, power(eb)) / $name::new(power(ec), smallest());

                let real = eb + smallest_exponent() - 2 * ec;
                assert!(
                    is_power(quotient.re, real),
                    "the real part was {:e}, and should be 2^{real}",
                    quotient.re
                );
                assert!(is_power(quotient.im, eb - ec), "the imaginary part was {:e}", quotient.im);
            }

            #[test]
            fn the_same_quotient_with_the_parts_in_every_other_place_is_right_too() {
                let (eb, ec) = if $float::MAX_EXP > 200 { (-511, -500) } else { (-63, -60) };
                let (tiny, b, c) = (smallest(), power(eb), power(ec));

                // (b + 0i) / (c + d i): the imaginary part of the quotient is the small one, `-b d / (c^2 + d^2)`.
                let quotient = $name::new(b, 0.0) / $name::new(c, tiny);
                assert!(is_power(quotient.re, eb - ec), "{quotient:?}");
                assert!(is_power(-quotient.im, eb + smallest_exponent() - 2 * ec), "{quotient:?}");

                // The small part in the divisor's real part, which is then the smaller of its parts: (b i) / (d + c i) is
                // `b (c + d i) / (c^2 + d^2)`.
                let quotient = $name::new(0.0, b) / $name::new(tiny, c);
                assert!(is_power(quotient.re, eb - ec), "{quotient:?}");
                assert!(is_power(quotient.im, eb + smallest_exponent() - 2 * ec), "{quotient:?}");

                // The small part in the numerator: (tiny + b i) / (c + 0i) is (tiny / c) + (b / c) i.
                let quotient = $name::new(tiny, b) / $name::new(c, 0.0);
                assert!(is_power(quotient.im, eb - ec), "{quotient:?}");
            }

            #[test]
            fn a_huge_numerator_over_a_divisor_with_a_tiny_part_keeps_the_part_that_is_still_representable() {
                // `b d / c^2` is `2^(eb + smallest - 2 ec)`. The divisor's parts differ in size by more than the smallest
                // number can hold as a ratio, so a division that scales the divisor down by its larger part loses `d`, and
                // with it a real part that is, because the numerator is huge, a perfectly good number.
                let (eb, ec) = if $float::MAX_EXP > 200 { (1000, 100) } else { (100, 20) };
                let quotient = $name::new(0.0, power(eb)) / $name::new(power(ec), smallest());

                let real = eb + smallest_exponent() - 2 * ec;
                assert!(quotient.re != 0.0, "the real part was lost: {quotient:?}");
                assert!(
                    is_power(quotient.re, real),
                    "the real part was {:e}, and should be 2^{real}",
                    quotient.re
                );
                assert!(is_power(quotient.im, eb - ec), "{quotient:?}");
            }

            #[test]
            fn every_part_must_be_in_the_comfortable_band_for_the_quick_algorithm() {
                let quarter = ($float::MAX_EXP - 1) / 4;

                assert!($name::is_comfortable(0.0) && $name::is_comfortable(-0.0));
                assert!($name::is_comfortable(1.0) && $name::is_comfortable(-3.5));
                assert!($name::is_comfortable(power(quarter)) && $name::is_comfortable(power(-quarter)));
                assert!(!$name::is_comfortable(power(quarter + 1)));
                assert!(!$name::is_comfortable(power(-quarter - 1)));
                assert!(!$name::is_comfortable(smallest()) && !$name::is_comfortable(MAX));
            }

            #[test]
            fn the_quick_algorithm_and_the_scaled_one_agree_wherever_the_quick_one_is_used() {
                // Every combination of parts that are zero, or in the comfortable band at five sizes and both signs.
                let quarter = ($float::MAX_EXP - 1) / 4;
                let mut parts = vec![0.0 as $float];
                for exponent in [-quarter, -quarter / 2, 0, quarter / 2, quarter] {
                    parts.push(power(exponent) * 1.25);
                    parts.push(-power(exponent) * 1.75);
                }
                for &a in &parts {
                    for &b in &parts {
                        for &c in &parts {
                            for &d in &parts {
                                if c == 0.0 && d == 0.0 {
                                    continue;
                                }
                                let quick = $name::new(a, b) / $name::new(c, d);
                                let scaled = $name::divide_scaled(a, b, c, d);
                                // Both are right, and differ by the rounding of the different order of the sums. A part that
                                // cancels is the size of the rounding of the parts it cancels between.
                                let allowed = |x: $float, y: $float| {
                                    (x - y).abs()
                                        <= (x.abs().max(y.abs()) * $tol).max(quick.re.abs().max(quick.im.abs()) * $tol)
                                };
                                assert!(
                                    allowed(quick.re, scaled.re) && allowed(quick.im, scaled.im),
                                    "({a:e} + {b:e}i) / ({c:e} + {d:e}i): {quick:?} against {scaled:?}"
                                );
                            }
                        }
                    }
                }
            }

            #[test]
            fn single_precision_division_agrees_with_double_precision_for_parts_of_every_size() {
                // Every part of a single precision number is comfortable in double precision, so double precision's own
                // division is a trustworthy answer for it, and is rounded to single precision to compare.
                if std::mem::size_of::<$float>() != 4 {
                    return;
                }
                let mut parts = vec![0.0_f64];
                for exponent in [-149, -126, -100, -60, -20, 0, 20, 60, 100, 127] {
                    parts.push(2.0_f64.powi(exponent) * 1.3);
                    parts.push(-(2.0_f64.powi(exponent)) * 1.9);
                }
                for &a in &parts {
                    for &b in &parts {
                        for &c in &parts {
                            for &d in &parts {
                                if c == 0.0 && d == 0.0 {
                                    continue;
                                }
                                let single =
                                    $name::new(a as $float, b as $float) / $name::new(c as $float, d as $float);
                                let double = crate::complex::Complex::new(f64::from(a as f32), f64::from(b as f32))
                                    / crate::complex::Complex::new(f64::from(c as f32), f64::from(d as f32));
                                for (got, want) in [(single.re, double.re as $float), (single.im, double.im as $float)]
                                {
                                    assert!(!got.is_nan(), "NaN for ({a:e} + {b:e}i) / ({c:e} + {d:e}i)");
                                    // A part that is close to zero beside the other is as accurate as the larger of the two.
                                    let larger = (double.re.abs().max(double.im.abs())) as $float;
                                    let allowed = (larger * 1e-5).max(smallest() * 4.0);
                                    assert!(
                                        got == want || (got - want).abs() <= allowed,
                                        "({a:e} + {b:e}i) / ({c:e} + {d:e}i): {got:e} against {want:e}"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            #[test]
            fn division_by_zero_is_an_infinity_and_nothing_divided_by_nothing_is_nan() {
                let by_zero = $name::new(1.0, 2.0) / $name::ZERO;
                assert!(by_zero.re.is_infinite() && by_zero.im.is_infinite(), "{by_zero:?}");

                assert!(($name::ZERO / $name::ZERO).re.is_nan());
            }

            #[test]
            fn a_number_that_is_not_finite_divides_as_it_always_did() {
                assert!(($name::new($float::NAN, 1.0) / $name::new(2.0, 1.0)).re.is_nan());
                // A finite number over an infinite one is zero.
                assert_eq!($name::new(1.0, 1.0) / $name::new($float::INFINITY, 0.0), $name::ZERO);
            }

            #[test]
            fn division_by_a_real_is_a_true_division_at_every_size() {
                let tiny = smallest() * 20.0;

                assert_eq!($name::new(tiny, 0.0) / tiny, $name::ONE);
                assert_eq!($name::new(smallest(), smallest()) / smallest(), $name::new(1.0, 1.0));
                assert_eq!($name::new(MAX, MAX) / MAX, $name::new(1.0, 1.0));
                // The reciprocal of the smallest number is infinite, which is what a reciprocal multiplied through gave.
                let mut z = $name::new(smallest(), 0.0);
                z /= smallest();
                assert_eq!(z, $name::ONE);

                let by_zero = $name::new(1.0, 1.0) / 0.0;
                assert!(by_zero.re.is_infinite() && by_zero.im.is_infinite());
            }

            #[test]
            fn division_by_a_real_matches_scaling_by_the_reciprocal_for_numbers_of_ordinary_size() {
                for divisor in [2.0, 4.0, 0.5, 8.0] {
                    let z = $name::new(3.0, -5.0);

                    assert_eq!(z / divisor, z.scale(1.0 / divisor));
                }
            }

            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
            //   The parts of the exponent
            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

            #[test]
            fn the_exponent_of_a_number_is_found_for_normal_and_subnormal_numbers_alike() {
                assert_eq!($name::ilogb(1.0), 0);
                assert_eq!($name::ilogb(1.999), 0);
                assert_eq!($name::ilogb(2.0), 1);
                assert_eq!($name::ilogb(0.75), -1);
                assert_eq!($name::ilogb(-8.0), 3);
                assert_eq!($name::ilogb(MAX), $float::MAX_EXP - 1);
                assert_eq!($name::ilogb($float::MIN_POSITIVE), $float::MIN_EXP - 1);
                // The subnormal numbers go on down from there, one exponent to each doubling.
                assert_eq!($name::ilogb($float::MIN_POSITIVE / 2.0), $float::MIN_EXP - 2);
                assert_eq!($name::ilogb(smallest()), $float::MIN_EXP - $float::MANTISSA_DIGITS as i32);
                assert_eq!(
                    $name::ilogb(smallest() * 3.0),
                    $float::MIN_EXP - $float::MANTISSA_DIGITS as i32 + 1
                );
            }

            #[test]
            fn scaling_by_a_power_of_two_is_exact_and_rounds_once_at_the_ends() {
                for k in -100..=100 {
                    for x in [1.0 as $float, 1.5, -2.75, 0.3] {
                        assert_eq!($name::scaled_by_pow2(x, k), x * (2.0 as $float).powi(k), "{x} scaled by {k}");
                    }
                }
                let largest_power = $float::MAX_EXP - 1;
                assert_eq!($name::scaled_by_pow2(1.0, largest_power), (2.0 as $float).powi(largest_power));
                assert!($name::scaled_by_pow2(1.0, largest_power + 1).is_infinite());
                assert!($name::scaled_by_pow2(-1.0, largest_power + 1) < 0.0);
                assert_eq!(
                    $name::scaled_by_pow2(1.0, $float::MIN_EXP - $float::MANTISSA_DIGITS as i32),
                    smallest()
                );
                // Half the smallest number is a tie, and rounds to the even one, which is zero.
                assert_eq!(
                    $name::scaled_by_pow2(1.0, $float::MIN_EXP - $float::MANTISSA_DIGITS as i32 - 1),
                    0.0
                );
                assert_eq!($name::scaled_by_pow2(MAX, -10 * $float::MAX_EXP), 0.0);
                assert_eq!($name::scaled_by_pow2(smallest(), 10 * $float::MAX_EXP), $float::INFINITY);
                assert_eq!($name::scaled_by_pow2(0.0, 5000), 0.0);
            }

            #[test]
            fn the_assigning_operators_match_the_plain_ones() {
                let (a, b) = ($name::new(1.0, 2.0), $name::new(3.0, -4.0));

                let mut z = a;
                z += b;
                assert_eq!(z, a + b);
                z = a;
                z -= b;
                assert_eq!(z, a - b);
                z = a;
                z *= b;
                assert_eq!(z, a * b);
                z = a;
                z /= b;
                assert_eq!(z, a / b);
                z = a;
                z *= 3.0;
                assert_eq!(z, a * 3.0);
                z = a;
                z /= 4.0;
                assert_eq!(z, a / 4.0);
            }

            #[test]
            fn norm_conjugate_and_argument() {
                let z = $name::new(3.0, 4.0);

                assert_eq!(z.norm(), 5.0);
                assert_eq!(z.norm_sqr(), 25.0);
                assert_eq!(z.conj(), $name::new(3.0, -4.0));
                assert_eq!(z * z.conj(), $name::from_real(25.0));
                assert!(($name::I.arg() - std::$float::consts::FRAC_PI_2).abs() < $tol);
                assert!(($name::new(-1.0, 0.0).arg() - std::$float::consts::PI).abs() < $tol);
            }

            #[test]
            fn polar_form_round_trips() {
                let z = $name::new(-1.25, 0.75);
                let (norm, arg) = z.to_polar();

                assert!(close($name::from_polar(norm, arg), z));
                assert!(close($name::cis(std::$float::consts::PI), $name::new(-1.0, 0.0)));
            }

            #[test]
            fn a_non_finite_part_is_noticed() {
                assert!($name::new(1.0, 2.0).is_finite());
                assert!(!$name::new($float::NAN, 0.0).is_finite());
                assert!(!$name::new(0.0, $float::INFINITY).is_finite());
            }

            #[test]
            fn numbers_sum_and_convert() {
                let total: $name = [$name::new(1.0, 1.0), $name::new(2.0, -3.0)].into_iter().sum();

                assert_eq!(total, $name::new(3.0, -2.0));
                assert_eq!($name::from(2.5), $name::new(2.5, 0.0));
                assert_eq!($name::from((1.0, -1.0)), $name::new(1.0, -1.0));
                assert_eq!($name::default(), $name::ZERO);
            }

            #[test]
            fn quarter_turn_roots_are_exact() {
                assert_eq!(root(0, 8), $name::ONE);
                assert_eq!(root(2, 8), $name::I);
                assert_eq!(root(4, 8), $name::new(-1.0, 0.0));
                assert_eq!(root(6, 8), $name::new(0.0, -1.0));
                assert_eq!(root(3, 12), $name::I);
            }

            #[test]
            fn an_eighth_turn_root_has_equal_parts() {
                let root = root(1, 8);

                assert!((root.re - std::$float::consts::FRAC_1_SQRT_2).abs() < $tol);
                assert!((root.im - std::$float::consts::FRAC_1_SQRT_2).abs() < $tol);
            }

            #[test]
            fn every_root_lies_on_the_unit_circle_at_its_angle() {
                for n in [1, 2, 3, 5, 6, 7, 8, 12, 64, 1000] {
                    for k in 0..n {
                        let root = root(k, n);
                        let expected = $name::cis(2.0 * std::$float::consts::PI * k as $float / n as $float);

                        assert!((root.norm() - 1.0).abs() < $tol, "|root({k}, {n})|");
                        assert!(close(root, expected), "root({k}, {n}) is {root}, expected {expected}");
                    }
                }
            }

            #[test]
            fn roots_are_conjugate_symmetric_to_the_last_bit() {
                for n in [3, 5, 8, 12, 64, 1000] {
                    for k in 1..n {
                        assert_eq!(root(n - k, n), root(k, n).conj(), "n = {n}, k = {k}");
                    }
                }
            }

            #[test]
            fn a_root_on_an_axis_has_a_positive_zero_and_never_a_negative_one() {
                for n in [4, 8, 12, 16] {
                    for k in 0..n {
                        let root = root(k, n);
                        for part in [root.re, root.im] {
                            assert!(part != 0.0 || part.is_sign_positive(), "root({k}, {n}) is {root:?}");
                        }
                    }
                }
            }

            #[test]
            fn k_is_taken_modulo_n() {
                assert_eq!(root(3, 8), root(11, 8));
                assert_eq!(root(8, 8), $name::ONE);
                assert_eq!(root(usize::MAX, 1), $name::ONE);
            }

            #[test]
            fn the_roots_sum_to_zero_and_multiply_by_adding_exponents() {
                for n in [2, 3, 8, 13, 64] {
                    let total: $name = roots(n, n).into_iter().sum();
                    assert!(total.norm() < $tol * n as $float, "the {n} roots sum to {total}");
                }

                let n = 16;
                for (j, k) in [(1, 2), (5, 9), (15, 15)] {
                    assert!(close(root(j, n) * root(k, n), root(j + k, n)));
                }
            }

            #[test]
            fn a_root_raised_to_n_is_one() {
                let n = 32;
                let root = root(1, n);
                let power = (0..n).fold($name::ONE, |acc, _| acc * root);

                assert!(close(power, $name::ONE));
            }

            #[test]
            fn the_table_of_roots_has_the_requested_length_and_order() {
                let table = roots(8, 4);

                assert_eq!(table.len(), 4);
                assert_eq!(table[0], $name::ONE);
                assert_eq!(table[2], $name::I);
                assert!(roots(8, 0).is_empty());
            }

            #[test]
            fn there_is_no_root_of_order_zero_and_asking_for_one_is_an_error_not_a_panic() {
                assert_eq!($name::unit_root(0, 0), Err(RootOfUnityError::ZeroOrder));
                assert_eq!($name::unit_root(5, 0), Err(RootOfUnityError::ZeroOrder));
                assert_eq!($name::unit_root(usize::MAX, 0), Err(RootOfUnityError::ZeroOrder));
                assert_eq!($name::unit_roots(0, 3), Err(RootOfUnityError::ZeroOrder));
                assert_eq!($name::unit_roots(0, 0), Err(RootOfUnityError::ZeroOrder));
            }

            #[test]
            fn the_error_says_what_is_wrong_and_is_a_standard_error() {
                let error = $name::unit_root(0, 0).expect_err("order zero");

                assert_eq!(error.to_string(), "there are no roots of unity of order zero");
                let boxed: Box<dyn std::error::Error> = Box::new(error);
                assert!(boxed.source().is_none());
            }

            #[test]
            fn display_signs_precision_and_negative_zero() {
                assert_eq!($name::new(1.0, 2.0).to_string(), "1+2i");
                assert_eq!($name::new(1.0, -2.0).to_string(), "1-2i");
                assert_eq!(format!("{:.3}", $name::new(0.7074, -0.7074)), "0.707-0.707i");
                assert_eq!(format!("{:.3}", $name::new(-0.0, -0.0)), "0.000+0.000i");
                assert_eq!(format!("{:.1}", $name::new(-0.0004, 0.0)), "-0.0+0.0i");
            }
        }
    };
}

scalar_tests!(double, Complex, f64, 1e-12);
scalar_tests!(single, Complex32, f32, 1e-5);
