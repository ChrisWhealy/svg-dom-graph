use super::{chi, chi_lane, window};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn chi_reads_three_lanes_along_x_with_wraparound() {
    assert_eq!(window(0), [0, 1, 2]);
    assert_eq!(window(3), [3, 4, 0]);
    assert_eq!(window(9), [9, 5, 6]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn chi_matches_the_fips_202_formula() {
    // `A'[x, y] = A[x, y] ^ ((A[(x + 1) mod 5, y] ^ 1) & A[(x + 2) mod 5, y])`, with `^ 1` a bitwise complement.
    let input: [u64; 25] = std::array::from_fn(|i| 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul(i as u64 + 1));
    let out = chi(input);
    for y in 0..5 {
        for x in 0..5 {
            let (a, b, c) = (input[x + 5 * y], input[(x + 1) % 5 + 5 * y], input[(x + 2) % 5 + 5 * y]);
            assert_eq!(out[x + 5 * y], a ^ (!b & c));
            assert_eq!(out[x + 5 * y], chi_lane(a, b, c));
        }
    }
}
