//! Quicksort in plain Rust, recorded one step at a time so that the demo can show every one. Nothing here touches the
//! DOM or the library.
//!
//! This is Quicksort with the Lomuto partition. The pivot is the last element of the range being sorted. One pass over
//! the rest moves every element smaller than the pivot to the front of the range, and then the pivot is swapped in
//! behind them. The pivot is now where it belongs, with the smaller elements on its left and the others on its right,
//! so each side is sorted the same way. A range of one element is already in place.

use crate::sorting::{Frame, Mark};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state of the sort after one step.
///
/// A step either looks or swaps, never both, so that a swap is a step of its own that can be shown moving. A step that
/// looks says which element is looked at and, if it has to be swapped, which element it will be swapped with. The next
/// step swaps those two.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Step {
    /// The elements as they are now.
    pub(super) array: Vec<u8>,
    /// The range being sorted, if a step is working on one. Everything outside it is waiting or finished.
    pub(super) range: Option<(usize, usize)>,
    /// The pivot of that range, which is its last element until the pivot is swapped into place.
    pub(super) pivot: Option<u8>,
    /// Where the pivot is: the last place of the range until it is swapped into its final place.
    pub(super) pivot_at: Option<usize>,
    /// The element the pass is looking at. Only a step that looks has one.
    pub(super) scan: Option<usize>,
    /// The element the pass has looked at and now has moved: where it is after the swap. Only a step that swaps has
    /// one.
    pub(super) examined: Option<usize>,
    /// The element the one looked at is to be swapped with: the first of the elements not yet known to be smaller.
    pub(super) candidate: Option<usize>,
    /// The two places whose elements this step swapped.
    pub(super) swapped: Option<(usize, usize)>,
    /// How many elements at the front of the range are known to be smaller than the pivot.
    pub(super) less: usize,
    /// Which elements are where they belong in the finished array.
    pub(super) fixed: Vec<bool>,
    /// What the step did, in a sentence or two of plain text.
    pub(super) note: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is left to sort, as a sentence: the ranges that wait on the stack, in the order they will be sorted.
fn waiting(stack: &[(usize, usize)]) -> String {
    let ranges: Vec<String> = stack
        .iter()
        .rev()
        .filter(|(lo, hi)| lo != hi)
        .map(|(lo, hi)| format!("{lo} to {hi}"))
        .collect();
    if ranges.is_empty() {
        String::new()
    } else {
        format!(" Still to sort: positions {}.", ranges.join(", then "))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every step of sorting `input`: first the array as it was given, then, for each range chosen, a step that chooses it,
/// a step for each element the pass looks at and another for the swap that element needs, a step that looks for the
/// pivot's place and another that swaps it in. The last step is the sorted array. There are always at least two steps
/// for an array that has an element.
pub(super) fn steps(input: &[u8]) -> Vec<Step> {
    let mut array = input.to_vec();
    let mut fixed = vec![false; array.len()];
    let mut out: Vec<Step> = Vec::new();
    // A step on the array as it is now, to which the caller adds what it is about.
    let step = |array: &[u8], fixed: &[bool], note: String| Step {
        array: array.to_vec(),
        fixed: fixed.to_vec(),
        note,
        ..Step::default()
    };

    let Some(last) = array.len().checked_sub(1) else {
        out.push(step(&array, &fixed, "There is nothing to sort.".to_owned()));
        return out;
    };
    out.push(step(
        &array,
        &fixed,
        format!("Quicksort starts with the whole array, positions 0 to {last}, and splits it around a pivot."),
    ));
    let mut stack = vec![(0, last)];
    while let Some((lo, hi)) = stack.pop() {
        if lo == hi {
            fixed[lo] = true;
            let note = format!(
                "A range of one element: {} is already in its final place.{}",
                array[lo],
                waiting(&stack)
            );
            out.push(Step {
                range: Some((lo, hi)),
                ..step(&array, &fixed, note)
            });
            continue;
        }
        let pivot = array[hi];
        let in_range = |step: Step| Step {
            range: Some((lo, hi)),
            pivot: Some(pivot),
            pivot_at: Some(hi),
            ..step
        };
        out.push(in_range(step(
            &array,
            &fixed,
            format!(
                "Sort positions {lo} to {hi}. The pivot is the last element, {pivot}. The pass moves every element smaller \
                 than the pivot to the front of the range."
            ),
        )));
        let mut less = 0;
        for scan in lo..hi {
            let value = array[scan];
            let target = lo + less;
            if value >= pivot {
                out.push(Step {
                    scan: Some(scan),
                    less,
                    ..in_range(step(
                        &array,
                        &fixed,
                        format!("{value} is not smaller than {pivot}, so it stays where it is."),
                    ))
                });
            } else if target == scan {
                out.push(Step {
                    scan: Some(scan),
                    less,
                    ..in_range(step(
                        &array,
                        &fixed,
                        format!(
                            "{value} is smaller than {pivot}, and is already next to the smaller elements, so nothing \
                             moves."
                        ),
                    ))
                });
                less += 1;
            } else {
                let other = array[target];
                out.push(Step {
                    scan: Some(scan),
                    candidate: Some(target),
                    less,
                    ..in_range(step(
                        &array,
                        &fixed,
                        format!(
                            "{value} is smaller than {pivot}, so it belongs with the smaller elements. The next place \
                             for it is position {target}, where {other} is, so the two will swap."
                        ),
                    ))
                });
                array.swap(target, scan);
                less += 1;
                out.push(Step {
                    examined: Some(target),
                    swapped: Some((target, scan)),
                    less,
                    ..in_range(step(
                        &array,
                        &fixed,
                        format!(
                            "Swap {value} with {other}. {value} is now at position {target}, with the smaller elements."
                        ),
                    ))
                });
            }
        }
        let place = lo + less;
        let displaced = array[place];
        if place == hi {
            out.push(Step {
                less,
                ..in_range(step(
                    &array,
                    &fixed,
                    format!("The pass is over. The pivot {pivot} is already last, so it is in its final place."),
                ))
            });
        } else {
            out.push(Step {
                candidate: Some(place),
                less,
                ..in_range(step(
                    &array,
                    &fixed,
                    format!(
                        "The pass is over. The first element that is not smaller than the pivot is {displaced} at \
                         position {place}. The pivot will swap with it, to go behind the smaller elements."
                    ),
                ))
            });
            array.swap(place, hi);
        }
        fixed[place] = true;
        if place < hi {
            stack.push((place + 1, hi));
        }
        if place > lo {
            stack.push((lo, place - 1));
        }
        let moved = if place == hi {
            format!("The pivot {pivot} stays where it is.")
        } else {
            format!(
                "Swap the pivot {pivot} with {displaced}. The pivot is now at position {place}, in its final place."
            )
        };
        out.push(Step {
            swapped: (place != hi).then_some((place, hi)),
            less,
            pivot_at: Some(place),
            ..in_range(step(
                &array,
                &fixed,
                format!(
                    "{moved} Everything before it is smaller and everything after it is not.{}",
                    waiting(&stack)
                ),
            ))
        });
    }
    out.push(step(
        &array,
        &fixed,
        "Every element is in its final place. The array is sorted.".to_owned(),
    ));
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What each bar of `step` is doing, for the colours. The pivot is red, and it stays red until the next range is
/// chosen. The element being looked at is green, and stays green when it is swapped, and the element it is to be
/// swapped with is blue. An element in its final place is lilac. Otherwise, inside the range being sorted, an element
/// known to be smaller than the pivot is teal, and everything outside the range is pale.
fn marks(step: &Step) -> Vec<Mark> {
    (0..step.array.len())
        .map(|i| {
            if step.pivot_at == Some(i) {
                Mark::Pivot
            } else if step.scan == Some(i) || step.examined == Some(i) {
                Mark::Examined
            } else if step.candidate == Some(i) {
                Mark::Candidate
            } else if step.fixed[i] {
                Mark::Done
            } else {
                match step.range {
                    Some((lo, _)) if (lo..lo + step.less).contains(&i) && step.pivot.is_some() => Mark::Group,
                    Some((lo, hi)) if i < lo || i > hi => Mark::Outside,
                    _ => Mark::Plain,
                }
            }
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The steps of sorting `input` as the demo draws them: the bars for each step, what they are doing, and the two that
/// changed places.
pub(crate) fn frames(input: &[u8]) -> Vec<Frame> {
    steps(input)
        .into_iter()
        .map(|step| Frame {
            marks: marks(&step),
            swap: step.swapped,
            values: step.array,
            note: step.note,
        })
        .collect()
}

#[cfg(test)]
mod unit_tests;
