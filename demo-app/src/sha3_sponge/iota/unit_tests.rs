use super::{ROUND_CONSTANTS, constant_for, iota};

#[test]
fn iota_changes_only_lane_zero_by_the_rounds_own_constant() {
    let input: [u64; 25] = std::array::from_fn(|i| 0x0123_4567_89AB_CDEF_u64.wrapping_mul(i as u64 + 1));
    for round in [0, 1, 23] {
        let out = iota(input, round);
        assert_eq!(out[0], input[0] ^ ROUND_CONSTANTS[round]);
        assert_eq!(out[1..], input[1..]);
    }
    assert_eq!(constant_for(0, 2), ROUND_CONSTANTS[2]);
    assert_eq!(constant_for(7, 2), 0);
}
