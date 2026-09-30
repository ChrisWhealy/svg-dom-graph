//! Pure-computation tests for `theta`'s own SHA3 math — kept separate from `theta_c`'s/`theta_d`'s own DOM-heavy
//! `build_scene` functions, which need a browser (see `crate::highlight::unit_tests`'s own doc comment for the
//! same "runs under plain `cargo test`, not `wasm-pack test`" reasoning).

use super::theta_d;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Theta` step is `D[x] = C[x-1] ⊕ rotl(C[x+1], 1)` — `rotl` a plain 64-bit left rotation, once a lane
/// is represented as a `u64`, with no further byte-order adjustment on top of it. `theta_d::outputs` once used
/// `rotate_right` instead of `rotate_left`, silently computing the wrong function while still "working" in every
/// DOM-level sense (the graph still drew, the toolbar still stepped) — nothing short of checking the real
/// arithmetic identity would have caught that. `c`'s own values are deliberately not all equal (and not all zero,
/// where `rotate_left`/`rotate_right` agree trivially), so a rotation in the wrong direction actually changes the
/// result here.
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
