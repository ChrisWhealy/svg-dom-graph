//! What every sorting algorithm's frames must satisfy, for the unit tests of each algorithm to share.

use super::{Frame, array::INITIAL};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every arrangement of `items`, by Heap's algorithm.
fn permutations(items: &mut Vec<u8>, size: usize, out: &mut Vec<Vec<u8>>) {
    if size <= 1 {
        out.push(items.clone());
        return;
    }
    for i in 0..size {
        permutations(items, size - 1, out);
        let swap = if size % 2 == 0 { i } else { 0 };
        items.swap(swap, size - 1);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The arrays an algorithm is tried on: every arrangement of five different elements, arrays with repeats, the empty
/// array and one element, an array already sorted and one in reverse, and the one the demos open on.
pub fn inputs() -> Vec<Vec<u8>> {
    let mut all = Vec::new();
    permutations(&mut vec![1, 2, 3, 4, 5], 5, &mut all);
    all.extend([
        vec![],
        vec![7],
        vec![3, 3, 3, 3],
        vec![5, 1, 5, 1, 5],
        vec![2, 1, 2, 1, 2, 1, 2],
        (10..20).collect(),
        (10..20).rev().collect(),
        INITIAL.to_vec(),
    ]);
    all
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks the frames an algorithm gives for `input`, which every algorithm's must satisfy:
///
/// - There is at least one frame, with a mark for every value, and the first is the array as it was given.
/// - Every frame holds the same elements, and the last is sorted.
/// - Between one frame and the next the array is the same, except that a frame which says it swapped two places has the
///   elements at those places exchanged.
/// - Every swap is between two different places that exist.
pub fn assert_valid_frames(frames: fn(&[u8]) -> Vec<Frame>, input: &[u8]) {
    let all = frames(input);
    let mut sorted = input.to_vec();
    sorted.sort_unstable();
    assert!(!all.is_empty(), "{input:?}");
    assert_eq!(all[0].values, input, "{input:?}");
    assert!(all[0].swap.is_none(), "{input:?}");
    assert_eq!(all.last().expect("a frame").values, sorted, "{input:?}");
    for (index, frame) in all.iter().enumerate() {
        assert_eq!(frame.marks.len(), frame.values.len(), "{input:?} step {index}");
        let mut values = frame.values.clone();
        values.sort_unstable();
        assert_eq!(values, sorted, "{input:?} step {index} lost an element");
        if index == 0 {
            continue;
        }
        let mut expected = all[index - 1].values.clone();
        if let Some((a, b)) = frame.swap {
            assert!(a != b && a < expected.len() && b < expected.len(), "{input:?} step {index}");
            expected.swap(a, b);
        }
        assert_eq!(frame.values, expected, "{input:?} step {index}");
    }
}
