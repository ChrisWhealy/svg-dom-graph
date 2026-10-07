//! Pure-computation tests for `theta`'s own SHA3 math — kept separate from `theta_c`'s/`theta_d`'s own DOM-heavy
//! `build_scene` functions, which need a browser (see `crate::highlight::unit_tests`'s own doc comment for the same
//! "runs under plain `cargo test`, not `wasm-pack test`" reasoning).

use super::{theta_d, xor_loop};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Theta` step is `D[x] = C[x-1] ⊕ rotl(C[x+1], 1)` — `rotl` a plain 64-bit left rotation, once a lane is
/// represented as a `u64`, with no further byte-order adjustment on top of it. `theta_d::outputs` once used
/// `rotate_right` instead of `rotate_left`, silently computing the wrong function while still "working" in every
/// DOM-level sense (the graph still drew, the toolbar still stepped) — nothing short of checking the real arithmetic
/// identity would have caught that. `c`'s own values are deliberately not all equal (and not all zero, where
/// `rotate_left`/`rotate_right` agree trivially), so a rotation in the wrong direction actually changes the result
/// here.
#[test]
fn outputs_matches_the_real_keccak_theta_identity() {
    let c: [u64; 5] = [
        0x0123_4567_89AB_CDEF,
        0xFEDC_BA98_7654_3210,
        0x1111_2222_3333_4444,
        0xAAAA_BBBB_CCCC_DDDD,
        0x0F0F_0F0F_F0F0_F0F0,
    ];
    let d = theta_d::outputs(c);
    for x in 0..5 {
        let expected = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        assert_eq!(d[x], expected, "D[{x}] did not match the real Keccak Theta identity");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `[[u64; 5]; 5]` counterpart to `crate::selection::unit_tests::stepping_backward_un_reveals_a_later_value` — same
/// bug class, same fix, indexed by flat cell position instead of row.
#[test]
fn stepping_backward_un_reveals_a_later_cell() {
    let outputs: [[u64; 5]; 5] = std::array::from_fn(|row| std::array::from_fn(|col| (row * 5 + col + 1) as u64));

    // Flat index 6 is (row 1, col 1): rows 0..=1's own first two cells revealed, nothing from col 2..=4 of row 1 or any
    // later row.
    let at_six = xor_loop::display_outputs(outputs, Some(6));
    assert_eq!(at_six[0], [1, 2, 3, 4, 5]);
    assert_eq!(at_six[1], [6, 7, 0, 0, 0]);
    assert_eq!(at_six[2], [0, 0, 0, 0, 0]);

    // Stepping back to flat index 2 (row 0, col 2) should un-reveal everything after it, including all of row 1.
    let back_to_two = xor_loop::display_outputs(outputs, Some(2));
    assert_eq!(back_to_two[0], [1, 2, 3, 0, 0]);
    assert_eq!(back_to_two[1], [0, 0, 0, 0, 0]);
}
