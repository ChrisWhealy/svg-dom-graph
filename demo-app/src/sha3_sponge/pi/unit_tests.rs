use super::{destination, pi};

#[test]
fn pi_matches_the_fips_202_destination_form() {
    // FIPS 202 states `A'[x, y] = A[(x + 3y) mod 5, x]`; every lane must land where that reads it from.
    let input: [u64; 25] = std::array::from_fn(|i| i as u64);
    let out = pi(input);
    for (xp, yp) in (0..5).flat_map(|y| (0..5).map(move |x| (x, y))) {
        let (src_x, src_y) = ((xp + 3 * yp) % 5, xp);
        assert_eq!(out[xp + 5 * yp], input[src_x + 5 * src_y], "A'[{xp}, {yp}]");
    }
}

#[test]
fn pi_is_a_permutation_and_leaves_lane_zero_alone() {
    let mut seen = [false; 25];
    for lane in 0..25 {
        seen[destination(lane).3] = true;
    }
    assert!(seen.iter().all(|s| *s));
    assert_eq!(destination(0).3, 0);
}
