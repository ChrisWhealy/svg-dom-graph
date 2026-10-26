//! The buffer tests, written once and run for both precisions.
//!
//! Each kernel is checked against the scalar type on lengths either side of a vector register's width, because the
//! compiler's vectorised loop and its leftover loop are separate code, and an error in either shows only at some
//! lengths.

macro_rules! buffer_tests {
    ($module:ident, $buf:ident, $name:ident, $float:ident, $tol:expr) => {
        mod $module {
            use crate::complex::{$buf, $name};

            const LENGTHS: [usize; 11] = [0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 33];

            /// Numbers that differ from one another in both parts and in sign, so a swapped part or sign is noticed.
            fn numbers(len: usize, seed: $float) -> Vec<$name> {
                (0..len)
                    .map(|i| {
                        let x = i as $float + seed;
                        $name::new(x * 0.75 - 2.0, 3.0 - x * 0.5)
                    })
                    .collect()
            }

            fn same(buffer: &$buf, expected: &[$name]) {
                assert_eq!(buffer.len(), expected.len());

                for (i, want) in expected.iter().enumerate() {
                    let got = buffer.get(i);
                    assert!((got - *want).norm() <= $tol, "number {i}: {got}, expected {want}");
                }
            }

            /// A four-point decimation-in-time transform built from the butterfly alone: the input in bit-reversed
            /// order, then a stage of two butterflies with half 1 and a stage of one block with half 2. `conjugate` says
            /// whether the twiddles are conjugated first, which is what makes it the conventional forward transform.
            fn transform_of_four(input: [$name; 4], conjugate: bool) -> Vec<$name> {
                // Bit-reversed order of four: 0, 2, 1, 3.
                let mut data = $buf::from_complex(&[input[0], input[2], input[1], input[3]]);
                let mut stage = |half: usize, data: &mut $buf| {
                    let mut twiddles = $buf::unit_roots(2 * half, half).expect("a nonzero order");
                    if conjugate {
                        twiddles.conj();
                    }
                    for start in (0..4).step_by(2 * half) {
                        data.butterfly(start, half, &twiddles);
                    }
                };
                stage(1, &mut data);
                stage(2, &mut data);
                data.to_complex()
            }

            #[test]
            fn conjugated_roots_make_the_conventional_forward_transform() {
                let (zero, one) = ($name::new(0.0, 0.0), $name::new(1.0, 0.0));
                let forward = transform_of_four([zero, one, zero, zero], true);
                let want = [
                    $name::new(1.0, 0.0),
                    $name::new(0.0, -1.0),
                    $name::new(-1.0, 0.0),
                    $name::new(0.0, 1.0),
                ];
                for (i, w) in want.iter().enumerate() {
                    assert!((forward[i] - *w).norm() <= $tol, "forward {i}: {}, expected {w}", forward[i]);
                }
            }

            #[test]
            fn the_roots_as_they_are_make_the_positive_exponent_transform() {
                let (zero, one) = ($name::new(0.0, 0.0), $name::new(1.0, 0.0));
                let positive = transform_of_four([zero, one, zero, zero], false);
                let want = [
                    $name::new(1.0, 0.0),
                    $name::new(0.0, 1.0),
                    $name::new(-1.0, 0.0),
                    $name::new(0.0, -1.0),
                ];
                for (i, w) in want.iter().enumerate() {
                    assert!((positive[i] - *w).norm() <= $tol, "positive {i}: {}, expected {w}", positive[i]);
                }
            }

            #[test]
            fn a_forward_then_an_inverse_transform_scaled_once_by_one_over_n_returns_the_input() {
                let input = [
                    $name::new(1.0, 2.0),
                    $name::new(-3.0, 0.5),
                    $name::new(0.25, -4.0),
                    $name::new(2.0, 1.0),
                ];
                let forward = transform_of_four(input, true);
                // The inverse is the positive-exponent transform, and the `1/n` is applied once, outside the butterfly.
                let mut inverse =
                    $buf::from_complex(&transform_of_four([forward[0], forward[1], forward[2], forward[3]], false));
                inverse.scale(0.25);
                for (i, want) in input.iter().enumerate() {
                    assert!(
                        (inverse.get(i) - *want).norm() <= $tol * 4.0,
                        "number {i}: {}, expected {want}",
                        inverse.get(i)
                    );
                }
            }

            #[test]
            fn splitting_and_joining_round_trips() {
                for len in LENGTHS {
                    let original = numbers(len, 0.5);
                    let buffer = $buf::from_complex(&original);

                    assert_eq!(buffer.len(), len);
                    assert_eq!(buffer.is_empty(), len == 0);
                    assert_eq!(buffer.to_complex(), original);
                }
            }

            #[test]
            fn the_real_and_imaginary_parts_are_separate_arrays() {
                let buffer = $buf::from_complex(&[$name::new(1.0, 2.0), $name::new(3.0, 4.0)]);

                assert_eq!(buffer.re(), &[1.0, 3.0]);
                assert_eq!(buffer.im(), &[2.0, 4.0]);
            }

            #[test]
            fn a_number_is_read_and_written_by_index() {
                let mut buffer = $buf::new(3);
                buffer.set(1, $name::new(5.0, -6.0));

                assert_eq!(buffer.get(0), $name::ZERO);
                assert_eq!(buffer.get(1), $name::new(5.0, -6.0));
                let (re, im) = buffer.parts_mut();
                re[2] = 7.0;
                im[2] = 8.0;
                assert_eq!(buffer.get(2), $name::new(7.0, 8.0));
            }

            #[test]
            fn adding_subtracting_and_multiplying_match_the_scalar_type() {
                for len in LENGTHS {
                    let (a, b) = (numbers(len, 0.5), numbers(len, 4.25));

                    let mut sum = $buf::from_complex(&a);
                    sum.add_assign(&$buf::from_complex(&b));
                    same(&sum, &a.iter().zip(&b).map(|(x, y)| *x + *y).collect::<Vec<_>>());

                    let mut difference = $buf::from_complex(&a);
                    difference.sub_assign(&$buf::from_complex(&b));
                    same(&difference, &a.iter().zip(&b).map(|(x, y)| *x - *y).collect::<Vec<_>>());

                    let mut product = $buf::from_complex(&a);
                    product.mul_assign(&$buf::from_complex(&b));
                    same(&product, &a.iter().zip(&b).map(|(x, y)| *x * *y).collect::<Vec<_>>());
                }
            }

            #[test]
            fn scaling_conjugating_and_the_squared_norm_match_the_scalar_type() {
                for len in LENGTHS {
                    let a = numbers(len, 1.5);

                    let mut scaled = $buf::from_complex(&a);
                    scaled.scale(-2.5);
                    same(&scaled, &a.iter().map(|x| *x * -2.5).collect::<Vec<_>>());

                    let mut conjugated = $buf::from_complex(&a);
                    conjugated.conj();
                    same(&conjugated, &a.iter().map(|x| x.conj()).collect::<Vec<_>>());

                    let mut norms = vec![0.0; len];
                    $buf::from_complex(&a).norm_sqr_into(&mut norms);
                    for (got, x) in norms.iter().zip(&a) {
                        assert!((got - x.norm_sqr()).abs() <= $tol * 100.0);
                    }
                }
            }

            #[test]
            fn a_butterfly_matches_the_scalar_formula_at_every_width() {
                for half in LENGTHS {
                    let start = 3;
                    let data = numbers(start + 2 * half + 2, 0.25);
                    let twiddles = $buf::unit_roots(2 * half.max(1), half).expect("a non-zero order");

                    let mut buffer = $buf::from_complex(&data);
                    buffer.butterfly(start, half, &twiddles);

                    let mut expected = data.clone();
                    for i in 0..half {
                        let (a, b, w) = (data[start + i], data[start + half + i], twiddles.get(i));
                        expected[start + i] = a + w * b;
                        expected[start + half + i] = a - w * b;
                    }
                    same(&buffer, &expected);
                }
            }

            #[test]
            fn a_butterfly_leaves_everything_outside_its_block_alone() {
                let data = numbers(20, 0.0);
                let mut buffer = $buf::from_complex(&data);
                buffer.butterfly(6, 4, &$buf::unit_roots(8, 4).expect("a non-zero order"));

                assert_eq!(&buffer.to_complex()[..6], &data[..6]);
                assert_eq!(&buffer.to_complex()[14..], &data[14..]);
            }

            #[test]
            fn a_butterfly_with_unit_twiddles_is_a_sum_and_a_difference() {
                let data = numbers(8, 1.0);
                let mut buffer = $buf::from_complex(&data);
                let ones = $buf::from_complex(&[$name::ONE; 4]);
                buffer.butterfly(0, 4, &ones);

                for i in 0..4 {
                    assert_eq!(buffer.get(i), data[i] + data[i + 4]);
                    assert_eq!(buffer.get(i + 4), data[i] - data[i + 4]);
                }
            }

            #[test]
            fn the_table_of_roots_is_the_scalar_roots_split() {
                let table = $buf::unit_roots(16, 8).expect("a non-zero order");

                for k in 0..8 {
                    assert_eq!(table.get(k), $name::unit_root(k, 16).expect("a non-zero order"));
                }
            }

            #[test]
            fn the_table_of_roots_of_order_zero_is_an_error() {
                assert!($buf::unit_roots(0, 4).is_err());
                assert!($buf::unit_roots(0, 0).is_err());
            }

            #[test]
            fn a_non_finite_part_in_either_array_is_noticed() {
                let mut buffer = $buf::new(4);
                assert!(buffer.is_finite());

                buffer.set(2, $name::new(0.0, $float::NAN));
                assert!(!buffer.is_finite());
                buffer.set(2, $name::new($float::INFINITY, 0.0));
                assert!(!buffer.is_finite());
            }

            #[test]
            #[should_panic(expected = "different lengths")]
            fn arithmetic_between_buffers_of_different_lengths_is_refused() {
                $buf::new(3).add_assign(&$buf::new(4));
            }

            #[test]
            #[should_panic(expected = "outside the buffer")]
            fn a_butterfly_block_beyond_the_end_is_refused() {
                $buf::new(8).butterfly(5, 2, &$buf::unit_roots(8, 2).expect("a non-zero order"));
            }

            #[test]
            #[should_panic(expected = "outside the buffer")]
            fn a_butterfly_block_that_overflows_is_refused() {
                $buf::new(8).butterfly(usize::MAX, usize::MAX, &$buf::new(1));
            }

            #[test]
            #[should_panic(expected = "fewer twiddle factors")]
            fn a_butterfly_without_enough_twiddles_is_refused() {
                $buf::new(8).butterfly(0, 4, &$buf::new(3));
            }
        }
    };
}

buffer_tests!(double, ComplexBuf, Complex, f64, 1e-12);
buffer_tests!(single, ComplexBuf32, Complex32, f32, 1e-4);
