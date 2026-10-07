//! Pure-computation tests for `display_outputs`. They are kept separate from this module's own DOM-heavy demo-building
//! functions, which need a browser. See `crate::highlight::unit_tests`'s own doc comment for the same "runs under plain
//! `cargo test`, not `wasm-pack test`" reasoning.

use super::display_outputs;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `display_outputs` once took a separately mutated `written: [bool; 5]` that was only ever set to `true` stepping
/// forward, never cleared stepping back. So "Previous" from row `2` to row `1` left row `2`'s own output still showing.
/// A walked-back-past value stayed visible, when this demo's own documentation says it should not. Deriving `display`
/// fresh from `outputs` and the walk's current position alone, with no separate flag to forget to clear, is exactly
/// what makes that impossible. This checks that stepping "backward" (calling with a smaller `to`) actually hides what a
/// later position had revealed. It does not merely check that stepping forward reveals values at all.
#[test]
fn stepping_backward_un_reveals_a_later_value() {
    let outputs: [u64; 5] = [10, 20, 30, 40, 50];

    let at_two = display_outputs(outputs, Some(2));
    assert_eq!(at_two, [10, 20, 30, 0, 0], "stepping to row 2 should reveal rows 0..=2");

    let back_to_one = display_outputs(outputs, Some(1));
    assert_eq!(
        back_to_one,
        [10, 20, 0, 0, 0],
        "stepping back to row 1 should un-reveal row 2 again, not just leave it showing"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `None` — unstarted, or walked/restarted all the way back — reveals nothing, regardless of what a previous position
/// had revealed.
#[test]
fn none_reveals_nothing() {
    let outputs: [u64; 5] = [10, 20, 30, 40, 50];
    assert_eq!(display_outputs(outputs, None), [0, 0, 0, 0, 0]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Some(4)` — the last row — reveals every row, the walk's own fully-stepped-forward state.
#[test]
fn the_last_position_reveals_every_row() {
    let outputs: [u64; 5] = [10, 20, 30, 40, 50];
    assert_eq!(display_outputs(outputs, Some(4)), outputs);
}
