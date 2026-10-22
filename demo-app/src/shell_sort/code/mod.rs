//! Shell sort in plain Rust, recorded one step at a time so that the demo can show every one. Nothing here touches the
//! DOM or the library.
//!
//! Shell sort is insertion sort done at several distances. For a gap of 5, say, it treats the elements 5 apart as a
//! short list of their own and puts each in order with the ones before it, by swaps. Then it does the same with a
//! smaller gap. By the last gap, which is 1, the array is nearly in order, so the plain insertion sort that ends it has
//! little to do. An element can cross many places in one swap while the gap is large, which is what bubble and
//! insertion sort cannot do.
//!
//! The gaps are the ones Shell first suggested: half the length, then half of that, and so on down to 1.

use demo_common::sorting::{Frame, Mark};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gaps for an array of `len` elements, from the largest: half of `len`, then half of that, down to 1.
pub(crate) fn gaps(len: usize) -> Vec<usize> {
    std::iter::successors(Some(len / 2), |gap| Some(gap / 2))
        .take_while(|&gap| gap > 0)
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A frame of the array `values`, with the elements `gap` apart that end at `upto` in teal, as the short list that
/// `upto` is being put in order in. A `gap` of 0 has none. The caller adds what the step is about.
fn frame(values: &[u8], gap: usize, upto: usize, note: String) -> Frame {
    Frame {
        values: values.to_vec(),
        marks: (0..values.len())
            .map(|i| {
                if gap > 0 && i <= upto && i % gap == upto % gap {
                    Mark::Group
                } else {
                    Mark::Plain
                }
            })
            .collect(),
        swap: None,
        note,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every step of sorting `input`: first the array as it was given, then, for each gap, a step that starts it. For each
/// element from the gap on, the steps compare it with the one a gap before it, and swap them if they are the wrong way
/// round, and go on comparing it with the one a gap before that, until it is in order with the others of its list. The
/// last step is the sorted array.
pub(crate) fn frames(input: &[u8]) -> Vec<Frame> {
    let mut array = input.to_vec();
    let n = array.len();
    let mut out = vec![if n == 0 {
        Frame {
            values: Vec::new(),
            marks: Vec::new(),
            swap: None,
            note: "There is nothing to sort.".to_owned(),
        }
    } else {
        frame(
            &array,
            0,
            0,
            "Shell sort puts the elements a gap apart in order with each other, for a smaller gap each time, ending with a gap of 1."
                .to_owned(),
        )
    }];
    if n < 2 {
        return out;
    }

    for gap in gaps(n) {
        out.push(frame(
            &array,
            0,
            0,
            if gap == 1 {
                "The gap is 1, so this is a plain insertion sort. The array is nearly in order by now, so it has little to do."
                    .to_owned()
            } else {
                format!("The gap is {gap}. Elements {gap} places apart are put in order with each other, as a short list of their own.")
            },
        ));
        for start in gap..n {
            // Move the element at `start` back along its list, a gap at a time, for as long as the one before it is
            // larger.
            let mut at = start;
            loop {
                if at < gap {
                    break;
                }
                let (before, here) = (array[at - gap], array[at]);
                let mut look = frame(&array, gap, start, String::new());
                look.marks[at] = Mark::Examined;
                look.marks[at - gap] = Mark::Candidate;
                if before > here {
                    look.note = format!(
                        "Gap {gap}: compare {here} with {before}, {gap} places before it. {before} is larger, so they are the \
                         wrong way round and will swap."
                    );
                    out.push(look);
                    array.swap(at - gap, at);
                    let mut moved = frame(
                        &array,
                        gap,
                        start,
                        format!("Swap {here} and {before}. {here} has moved {gap} places to the left."),
                    );
                    moved.marks[at - gap] = Mark::Examined;
                    moved.swap = Some((at - gap, at));
                    out.push(moved);
                    at -= gap;
                } else {
                    look.note = format!(
                        "Gap {gap}: compare {here} with {before}, {gap} places before it. {before} is not larger, so {here} \
                         is in order with its list."
                    );
                    out.push(look);
                    break;
                }
            }
        }
    }
    let mut last = frame(
        &array,
        0,
        0,
        "The gap-1 pass is done. Every element is in its final place. The array is sorted.".to_owned(),
    );
    last.marks.iter_mut().for_each(|mark| *mark = Mark::Done);
    out.push(last);
    out
}

#[cfg(test)]
mod unit_tests;
