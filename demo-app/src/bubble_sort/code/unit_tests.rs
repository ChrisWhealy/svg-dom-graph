use super::frames;
use demo_common::sorting::{
    Mark,
    testing::{assert_valid_frames, inputs},
};

#[test]
fn every_frame_is_valid_for_every_input() {
    for input in inputs() {
        assert_valid_frames(frames, &input);
    }
}

fn swaps(input: &[u8]) -> usize {
    frames(input).iter().filter(|f| f.swap.is_some()).count()
}

fn looks(input: &[u8]) -> usize {
    frames(input)
        .iter()
        .filter(|f| f.marks.contains(&Mark::Candidate) && f.swap.is_none())
        .count()
}

#[test]
fn an_array_in_reverse_needs_every_pair_swapped() {
    let reversed: Vec<u8> = (10..20).rev().collect();
    assert_eq!(swaps(&reversed), 45);
}

#[test]
fn an_array_in_order_needs_one_pass_and_no_swaps() {
    let sorted: Vec<u8> = (10..20).collect();
    assert_eq!(swaps(&sorted), 0);
    // One pass of nine comparisons, and then the pass that swaps nothing ends the sort.
    assert_eq!(looks(&sorted), 9);
}

#[test]
fn the_number_of_swaps_is_the_number_of_pairs_out_of_order() {
    // Bubble sort swaps only neighbours, so it makes one swap for each pair that is the wrong way round.
    for input in inputs() {
        let inversions = (0..input.len())
            .flat_map(|i| (i + 1..input.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| input[i] > input[j])
            .count();
        assert_eq!(swaps(&input), inversions, "{input:?}");
    }
}

#[test]
fn a_pair_that_is_compared_is_two_neighbours_green_on_the_left_and_blue_on_the_right() {
    for frame in frames(&[47, 12, 83, 29, 61, 18]) {
        if frame.swap.is_some() {
            // After the swap the green element is the one that was carried to the right.
            assert_eq!(frame.marks.iter().filter(|&&m| m == Mark::Examined).count(), 1);
            continue;
        }
        let green = frame.marks.iter().position(|&m| m == Mark::Examined);
        let blue = frame.marks.iter().position(|&m| m == Mark::Candidate);
        if let (Some(green), Some(blue)) = (green, blue) {
            assert_eq!(blue, green + 1);
        }
    }
}

#[test]
fn the_largest_element_left_is_in_place_at_the_end_of_each_pass() {
    // After the first pass of [3, 1, 2], the 3 is at the end and is lilac, and nothing else is.
    let all = frames(&[3, 1, 2]);
    let after_first_pass = all
        .iter()
        .find(|f| f.note.starts_with("The pass is over"))
        .expect("a pass ends");
    assert_eq!(after_first_pass.values, [1, 2, 3]);
    assert_eq!(after_first_pass.marks, [Mark::Plain, Mark::Plain, Mark::Done]);
}

#[test]
fn the_last_frame_has_everything_in_place() {
    let all = frames(&[47, 12, 83, 29, 61, 18]);
    assert!(all.last().expect("a frame").marks.iter().all(|&m| m == Mark::Done));
}
