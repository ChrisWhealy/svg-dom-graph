use super::{frames, gaps};
use demo_common::sorting::{
    Mark,
    testing::{assert_valid_frames, inputs},
};

#[test]
fn the_gaps_are_half_the_length_then_half_of_that_down_to_one() {
    assert_eq!(gaps(10), [5, 2, 1]);
    assert_eq!(gaps(8), [4, 2, 1]);
    assert_eq!(gaps(2), [1]);
    assert!(gaps(1).is_empty());
    assert!(gaps(0).is_empty());
}

#[test]
fn every_frame_is_valid_for_every_input() {
    for input in inputs() {
        assert_valid_frames(frames, &input);
    }
}

#[test]
fn every_swap_is_between_places_a_gap_apart() {
    for input in inputs() {
        let allowed = gaps(input.len());
        for frame in frames(&input) {
            if let Some((a, b)) = frame.swap {
                assert!(allowed.contains(&(b - a)), "{input:?}: {a} and {b}");
            }
        }
    }
}

#[test]
fn the_pass_for_each_gap_is_announced_in_order() {
    let starts: Vec<String> = frames(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56])
        .into_iter()
        .map(|f| f.note)
        .filter(|note| note.starts_with("The gap is"))
        .collect();
    assert_eq!(starts.len(), 3);
    assert!(starts[0].starts_with("The gap is 5."));
    assert!(starts[1].starts_with("The gap is 2."));
    assert!(starts[2].starts_with("The gap is 1,"));
}

#[test]
fn an_element_is_compared_with_the_one_a_gap_before_it() {
    for frame in frames(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56]) {
        if frame.swap.is_some() || !frame.note.starts_with("Gap ") {
            continue;
        }
        let green = frame.marks.iter().position(|&m| m == Mark::Examined).expect("green");
        let blue = frame.marks.iter().position(|&m| m == Mark::Candidate).expect("blue");
        let gap: usize = frame.note[4..].split(':').next().and_then(|n| n.parse().ok()).expect("a gap");
        assert_eq!(green - blue, gap, "{}", frame.note);
    }
}

#[test]
fn the_elements_a_gap_apart_up_to_the_one_being_placed_are_teal() {
    // The first comparison with a gap of 5 puts element 5 in order with element 0, so the list is those two.
    let all = frames(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56]);
    let first = all.iter().find(|f| f.note.starts_with("Gap 5")).expect("a comparison");
    let teal: Vec<usize> = (0..10).filter(|&i| first.marks[i] == Mark::Group).collect();
    // Elements 0 and 5 are the list; 5 is green and 0 is blue, so there are none left to be teal.
    assert!(teal.is_empty(), "{teal:?}");
    let later = all.iter().find(|f| f.note.starts_with("Gap 2")).expect("a comparison");
    assert!(later.marks.contains(&Mark::Group) || later.marks.contains(&Mark::Candidate));
}

#[test]
fn the_last_frame_has_everything_in_place() {
    let all = frames(&[47, 12, 83, 29, 61, 18]);
    assert!(all.last().expect("a frame").marks.iter().all(|&m| m == Mark::Done));
}

#[test]
fn a_larger_gap_moves_an_element_further_for_each_swap() {
    // An array in reverse is the worst case for bubble sort, which swaps 45 pairs. Shell sort moves elements more than
    // one place at a time while the gap is large, so it swaps fewer.
    let reversed: Vec<u8> = (10..20).rev().collect();
    let swaps = frames(&reversed).iter().filter(|f| f.swap.is_some()).count();
    assert!(swaps < 45, "{swaps}");
}
