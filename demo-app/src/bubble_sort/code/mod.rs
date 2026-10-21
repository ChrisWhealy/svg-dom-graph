//! Bubble sort in plain Rust, recorded one step at a time so that the demo can show every one. Nothing here touches the
//! DOM or the library.
//!
//! One pass walks along the array comparing each element with the one next to it, and swaps the two if they are the
//! wrong way round. The largest element left is carried to the end of the array by the swaps, like a bubble rising, so
//! after each pass one more element is in its final place at the end. If a whole pass makes no swap, the array is
//! sorted.

use crate::sorting::{Frame, Mark};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The frame for the array `values` with `done` elements at its end in their final place. `look` is the element at the
/// left of the pair being compared and `swap` the two places swapped by this step. The caller adds what it is about.
fn frame(values: &[u8], done: usize, note: String) -> Frame {
    let first_done = values.len() - done;
    Frame {
        values: values.to_vec(),
        marks: (0..values.len())
            .map(|i| if i >= first_done { Mark::Done } else { Mark::Plain })
            .collect(),
        swap: None,
        note,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every step of sorting `input`: first the array as it was given, then for each pair a pass compares, a step that
/// looks at the pair, and, if they are the wrong way round, another that swaps them. After a pass that swapped
/// something, a step says where the pass ended. The last step is the sorted array, which is reached after a pass that
/// swapped nothing, or when only one element is left.
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
            "The array is not sorted. Each pass compares neighbours from the left and swaps any that are the wrong way round."
                .to_owned(),
        )
    }];
    if n < 2 {
        return out;
    }

    let mut done = 0;
    while done < n - 1 {
        let mut swapped = false;
        for left in 0..n - 1 - done {
            let (a, b) = (array[left], array[left + 1]);
            let mut look = frame(&array, done, String::new());
            look.marks[left] = Mark::Examined;
            look.marks[left + 1] = Mark::Candidate;
            if a > b {
                look.note =
                    format!("Compare {a} with {b}. {a} is larger, so they are the wrong way round and will swap.");
                out.push(look);
                array.swap(left, left + 1);
                // The larger element is carried on, so it stays green.
                let mut moved = frame(&array, done, format!("Swap {a} and {b}. {a} has moved one place to the right."));
                moved.marks[left + 1] = Mark::Examined;
                moved.swap = Some((left, left + 1));
                out.push(moved);
                swapped = true;
            } else {
                look.note = format!("Compare {a} with {b}. {a} is not larger, so they are in order.");
                out.push(look);
            }
        }
        done += 1;
        if !swapped {
            // Nothing needed to move, so everything before the end is in order already.
            out.push(frame(
                &array,
                n,
                "That pass made no swaps, so every element is in order. The array is sorted.".to_owned(),
            ));
            return out;
        }
        out.push(frame(
            &array,
            done,
            format!(
                "The pass is over. The largest element left, {}, has been carried to the end, so it is in its final place.",
                array[n - done]
            ),
        ));
    }
    // Only one element is left, and it can only be the smallest.
    out.push(frame(
        &array,
        n,
        "Every element is in its final place. The array is sorted.".to_owned(),
    ));
    out
}

#[cfg(test)]
mod unit_tests;
