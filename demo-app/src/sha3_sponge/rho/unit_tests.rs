use super::{ROTATION_OFFSETS, offset, rho};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rotation_offsets_match_the_spec_s_own_derivation() {
    // FIPS 202 Algorithm 2 derives the offsets: walk `(x, y) -> (y, 2x + 3y)` from `(1, 0)`, and step `t`'s
    // offset is the triangular number `(t + 1)(t + 2) / 2`. Table 2 lists it unreduced; this table holds it
    // mod 64.
    let (mut x, mut y) = (1, 0);
    for t in 0..24 {
        assert_eq!(usize::from(offset(x + 5 * y)), (t + 1) * (t + 2) / 2 % 64, "step {t}");
        (x, y) = (y, (2 * x + 3 * y) % 5);
    }
    assert_eq!(offset(0), 0);
    assert_eq!(ROTATION_OFFSETS.len(), 24);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rho_rotates_each_lane_by_its_own_offset() {
    let out = rho([1; 25]);
    assert_eq!(out[0], 1);
    assert_eq!(out[2], 1 << 62);
    assert_eq!(out[24], 1 << 14);
}
