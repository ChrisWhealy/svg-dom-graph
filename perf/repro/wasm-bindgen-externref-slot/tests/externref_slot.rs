//! Run with `wasm-pack test --node` in this directory. See the crate's README for what it shows and how to read a
//! failure.

use externref_slot_repro::{nothing, number, object, slots_left_by};
use wasm_bindgen_test::wasm_bindgen_test;

const FAILURES: u32 = 1000;

/// An import that returns an object and throws should leave no slot behind, since the caller drops the `Err` at once.
/// It leaves one each. This test states what ought to be true, and fails while the behaviour exists.
///
/// A dependency update that makes it pass has fixed the problem. The test below, which states the behaviour as it is,
/// will then fail, which is how a repository that works around the problem is told that it can stop.
#[wasm_bindgen_test]
fn a_failed_import_that_returns_an_object_should_leave_no_slot() {
    let left = slots_left_by(object, FAILURES, true);
    assert_eq!(
        left, 0,
        "{left} slots were left live by {FAILURES} failures of an import that returns an object"
    );
}

/// The behaviour as it is in the pinned version: exactly one slot a failure, for an import that returns an object.
#[wasm_bindgen_test]
fn today_a_failed_import_that_returns_an_object_leaves_one_slot_each() {
    assert_eq!(slots_left_by(object, FAILURES, true), FAILURES);
}

/// The controls. An import that returns nothing, or a number, leaves nothing when it throws, and an import that returns
/// an object leaves nothing when it does not throw. So it is the throw of an import that returns an object, and nothing
/// else.
#[wasm_bindgen_test]
fn the_controls_leave_nothing() {
    assert_eq!(slots_left_by(nothing, FAILURES, true), 0, "returns nothing, throws");
    assert_eq!(slots_left_by(number, FAILURES, true), 0, "returns a number, throws");
    assert_eq!(slots_left_by(object, FAILURES, false), 0, "returns an object, does not throw");
    assert_eq!(slots_left_by(nothing, FAILURES, false), 0, "returns nothing, does not throw");
}
