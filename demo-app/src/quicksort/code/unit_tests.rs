use super::{Step, frames, steps};
use demo_common::sorting::{Mark, array::LEN};

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

fn sorted(input: &[u8]) -> Vec<u8> {
    let mut sorted = input.to_vec();
    sorted.sort_unstable();
    sorted
}

#[test]
fn every_arrangement_of_six_elements_ends_sorted_with_every_element_fixed() {
    let mut all = Vec::new();
    permutations(&mut vec![1, 2, 3, 4, 5, 6], 6, &mut all);
    assert_eq!(all.len(), 720);
    for input in all {
        let steps = steps(&input);
        let last = steps.last().expect("a step");
        assert_eq!(last.array, sorted(&input), "{input:?}");
        assert!(last.fixed.iter().all(|&f| f), "{input:?}");
    }
}

#[test]
fn elements_that_repeat_are_sorted_too() {
    for input in [vec![3, 3, 3, 3], vec![5, 1, 5, 1, 5], vec![2, 1, 2, 1, 2, 1, 2], vec![7]] {
        let steps = steps(&input);
        assert_eq!(steps.last().expect("a step").array, sorted(&input), "{input:?}");
    }
}

#[test]
fn an_empty_array_has_one_step_that_says_there_is_nothing_to_sort() {
    let steps = steps(&[]);
    assert_eq!(steps.len(), 1);
    assert!(steps[0].note.contains("nothing to sort"));
}

#[test]
fn the_first_step_is_the_array_as_given_and_nothing_is_fixed() {
    let input = [47, 12, 83, 29];
    let first = &steps(&input)[0];
    assert_eq!(first.array, input);
    assert!(first.fixed.iter().all(|&f| !f) && first.range.is_none() && first.pivot.is_none());
}

#[test]
fn every_step_holds_the_same_elements_and_a_fixed_element_is_where_the_sorted_array_has_it() {
    let input = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];
    let done = sorted(&input);
    for step in steps(&input) {
        assert_eq!(sorted(&step.array), done);
        for (i, &fixed) in step.fixed.iter().enumerate() {
            assert!(!fixed || step.array[i] == done[i], "position {i}: {step:?}");
        }
    }
}

#[test]
fn a_fixed_element_never_becomes_unfixed_and_never_moves() {
    let steps = steps(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56]);
    for pair in steps.windows(2) {
        for i in 0..pair[0].fixed.len() {
            if pair[0].fixed[i] {
                assert!(pair[1].fixed[i]);
                assert_eq!(pair[1].array[i], pair[0].array[i]);
            }
        }
    }
}

#[test]
fn the_pass_looks_at_every_element_of_the_range_but_the_pivot() {
    // [3, 1, 2]: the range 0 to 2 has pivot 2, and the pass looks at positions 0 and 1.
    let looked: Vec<(usize, (usize, usize))> =
        steps(&[3, 1, 2]).iter().filter_map(|s| s.scan.zip(s.range)).take(2).collect();
    assert_eq!(looked, [(0, (0, 2)), (1, (0, 2))]);
}

#[test]
fn the_smaller_elements_are_at_the_front_of_the_range_during_the_pass() {
    for step in steps(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56]) {
        if let (Some((lo, _)), Some(pivot), Some(_)) = (step.range, step.pivot, step.scan) {
            assert!(step.array[lo..lo + step.less].iter().all(|&v| v < pivot), "{step:?}");
        }
    }
}

#[test]
fn the_pivot_is_swapped_in_after_the_smaller_elements_and_before_the_rest() {
    let after: Vec<Step> = steps(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56])
        .into_iter()
        .filter(|s| s.note.contains("Everything before it"))
        .collect();
    assert!(!after.is_empty());
    for step in after {
        let (lo, hi) = step.range.expect("a range");
        let pivot = step.pivot.expect("a pivot");
        let place = lo + step.less;
        assert_eq!(step.array[place], pivot, "{step:?}");
        assert!(step.array[lo..place].iter().all(|&v| v < pivot));
        assert!(step.array[place + 1..=hi].iter().all(|&v| v >= pivot));
    }
}

#[test]
fn an_array_that_is_already_sorted_is_the_worst_case_and_looks_at_every_pair() {
    // With the last element as the pivot, a sorted array splits into nothing and everything else every time.
    let looks = |input: &[u8]| steps(input).iter().filter(|s| s.scan.is_some()).count();
    let ascending: Vec<u8> = (10..20).collect();
    let descending: Vec<u8> = (10..20).rev().collect();
    assert_eq!(looks(&ascending), LEN * (LEN - 1) / 2);
    assert!(looks(&descending) >= LEN * (LEN - 1) / 4);
    assert!(looks(&[15, 12, 17, 11, 19, 14, 18, 13, 16, 10]) < LEN * (LEN - 1) / 2);
}

#[test]
fn there_is_a_mark_for_every_bar_of_every_frame() {
    for frame in frames(&[47, 12, 83, 29, 61, 18, 95, 34, 72, 56]) {
        assert_eq!(frame.marks.len(), frame.values.len());
    }
}

#[test]
fn the_first_frame_has_nothing_marked_and_the_last_has_everything_in_place() {
    let all = frames(&[47, 12, 83, 29, 61, 18]);
    assert!(all[0].marks.iter().all(|&m| m == Mark::Plain));
    assert!(all.last().expect("a frame").marks.iter().all(|&m| m == Mark::Done));
}

#[test]
fn the_pivot_is_red_and_it_is_the_last_bar_of_the_range_during_the_pass() {
    let input = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];
    for (step, frame) in steps(&input).iter().zip(frames(&input)) {
        let reds: Vec<usize> = (0..frame.marks.len()).filter(|&i| frame.marks[i] == Mark::Pivot).collect();
        match (step.pivot, step.range, step.scan) {
            (Some(pivot), Some((_, hi)), Some(_)) => {
                assert_eq!(reds, [hi], "{step:?}");
                assert_eq!(frame.values[hi], pivot);
            },
            (None, ..) => assert!(reds.is_empty(), "{step:?}"),
            _ => assert!(reds.len() <= 1, "{step:?}"),
        }
    }
}

/// A plain recursive Quicksort, counting the elements it looks at: one less than the size of each range it splits.
fn looks(values: &mut [u8]) -> usize {
    if values.len() < 2 {
        return 0;
    }
    let last = values.len() - 1;
    let pivot = values[last];
    let mut less = 0;
    for i in 0..last {
        if values[i] < pivot {
            values.swap(i, less);
            less += 1;
        }
    }
    values.swap(less, last);
    let (left, right) = values.split_at_mut(less);
    last + looks(left) + looks(&mut right[1..])
}

const INPUT: [u8; 10] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

#[test]
fn the_element_looked_at_is_green_and_stays_green_when_it_is_swapped() {
    let all = steps(&INPUT);
    let mut seen = 0;
    for (step, frame) in all.iter().zip(frames(&INPUT)) {
        let greens: Vec<usize> = (0..frame.marks.len()).filter(|&i| frame.marks[i] == Mark::Examined).collect();
        match (step.scan, step.examined) {
            (Some(scan), None) => {
                seen += 1;
                assert_eq!(greens, [scan], "{step:?}");
            },
            (None, Some(now)) => assert_eq!(greens, [now], "{step:?}"),
            _ => assert!(greens.is_empty(), "{step:?}"),
        }
    }
    assert_eq!(seen, looks(&mut INPUT.to_vec()));
}

#[test]
fn a_swap_is_a_step_of_its_own_that_follows_the_step_that_looks() {
    let all = steps(&INPUT);
    let mut swaps = 0;
    for (index, step) in all.iter().enumerate() {
        let Some((a, b)) = step.swapped else {
            // A step that swaps nothing leaves the bars where they were.
            assert!(index == 0 || all[index - 1].array == step.array, "step {index}: {step:?}");
            continue;
        };
        swaps += 1;
        let mut before = all[index - 1].array.clone();
        before.swap(a, b);
        assert_eq!(before, step.array, "step {index} swaps {a} and {b}");
        // The step before looked, and said which bar would swap with which, so the swap is no surprise.
        let looked = &all[index - 1];
        assert!(looked.swapped.is_none());
        match (looked.scan, looked.candidate) {
            (Some(scan), Some(candidate)) => assert_eq!((candidate, scan), (a, b), "step {index}"),
            (None, Some(candidate)) => {
                assert_eq!((candidate, looked.pivot_at.expect("a pivot")), (a, b), "step {index}")
            },
            other => panic!("a swap with nothing to swap before it: {other:?}"),
        }
    }
    assert!(swaps > 0);
}

#[test]
fn the_pivot_is_red_from_the_range_being_chosen_until_the_next_range_is() {
    for (step, frame) in steps(&INPUT).iter().zip(frames(&INPUT)) {
        let reds: Vec<usize> = (0..frame.marks.len()).filter(|&i| frame.marks[i] == Mark::Pivot).collect();
        assert_eq!(reds, step.pivot_at.into_iter().collect::<Vec<_>>(), "{step:?}");
    }
}

#[test]
fn the_element_to_swap_with_is_blue_only_while_it_is_waiting_for_the_swap() {
    for (step, frame) in steps(&INPUT).iter().zip(frames(&INPUT)) {
        let blues: Vec<usize> = (0..frame.marks.len()).filter(|&i| frame.marks[i] == Mark::Candidate).collect();
        assert_eq!(blues, step.candidate.into_iter().collect::<Vec<_>>(), "{step:?}");
        // A step that has swapped has nothing waiting.
        assert!(step.swapped.is_none() || step.candidate.is_none());
    }
}

#[test]
fn the_smaller_elements_are_blue_and_what_is_outside_the_range_is_pale() {
    let input = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];
    let all = steps(&input);
    let marks = frames(&input);
    let mut groups = 0;
    for (step, frame) in all.iter().zip(&marks) {
        for (i, &mark) in frame.marks.iter().enumerate() {
            if mark == Mark::Group {
                groups += 1;
                let (lo, _) = step.range.expect("a range");
                assert!((lo..lo + step.less).contains(&i));
            }
            if mark == Mark::Outside {
                let (lo, hi) = step.range.expect("a range");
                assert!(i < lo || i > hi);
            }
        }
    }
    assert!(groups > 0);
}

#[test]
fn every_frame_is_valid_for_every_input() {
    for input in demo_common::sorting::testing::inputs() {
        demo_common::sorting::testing::assert_valid_frames(frames, &input);
    }
}
