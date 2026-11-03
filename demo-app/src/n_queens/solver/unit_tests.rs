//! The counts, the order of the events, and what every snapshot of the boards that the page offers must hold.

use super::*;

/// The solutions of N=1..=9, and the events of N=4, 8, 9 and 10, which were worked out by hand.
const SOLUTIONS: [usize; 9] = [1, 0, 0, 2, 10, 4, 40, 92, 352];
const EVENTS: [(usize, usize); 4] = [(4, 139), (8, 33_589), (9, 153_502), (10, 732_563)];

fn trace(n: usize) -> Trace {
    Trace::build(n).expect("a board this size can be traced")
}

/// The columns of the queens in snapshot `index`, from row 0 to the last row that has one.
fn queens(trace: &Trace, index: usize) -> Vec<usize> {
    let snapshot = trace.snapshot(index).expect("the snapshot exists");
    (0..usize::from(snapshot.depth))
        .filter_map(|row| snapshot.queen_in(row))
        .collect()
}

/// Whether no two of `columns` share a column or a diagonal.
fn is_legal(columns: &[usize]) -> bool {
    columns.iter().enumerate().all(|(r, &c)| {
        columns
            .iter()
            .enumerate()
            .skip(r + 1)
            .all(|(r2, &c2)| c != c2 && r2 - r != c.abs_diff(c2))
    })
}

// The counts.

#[test]
fn every_board_finds_the_known_number_of_solutions() {
    for (index, &expected) in SOLUTIONS.iter().enumerate() {
        assert_eq!(trace(index + 1).solution_count(), expected, "N={}", index + 1);
    }
}

#[test]
fn the_events_of_the_boards_that_were_counted_by_hand() {
    for (n, expected) in EVENTS {
        assert_eq!(count_only(n).map(|c| c.events), Ok(expected), "count-only N={n}");
        if n <= MAX_N {
            assert_eq!(trace(n).len(), expected, "trace N={n}");
        }
    }
}

#[test]
fn counting_without_a_trace_agrees_with_the_trace_for_every_board_the_trace_takes() {
    for n in 1..=MAX_N {
        let counts = count_only(n).expect("counted");
        let trace = trace(n);
        assert_eq!(
            (counts.events, counts.solutions),
            (trace.len(), trace.solution_count()),
            "N={n}"
        );
    }
}

#[test]
fn a_board_of_ten_has_724_solutions_but_cannot_be_traced() {
    assert_eq!(count_only(10).map(|c| c.solutions), Ok(724));
    assert_eq!(Trace::build(10).err(), Some(TraceError::UnsupportedBoardSize));
}

#[test]
fn the_sizes_that_are_refused_are_refused_before_anything_is_built() {
    assert_eq!(Trace::build(0).err(), Some(TraceError::InvalidBoardSize));
    assert_eq!(Trace::build(10).err(), Some(TraceError::UnsupportedBoardSize));
    assert_eq!(Trace::build(usize::MAX).err(), Some(TraceError::UnsupportedBoardSize));
    assert_eq!(count_only(0).err(), Some(TraceError::InvalidBoardSize));
    assert_eq!(count_only(11).err(), Some(TraceError::UnsupportedBoardSize));
}

#[test]
fn the_two_solutions_of_four_queens_are_the_known_ones() {
    let trace = trace(4);
    let found: Vec<Vec<usize>> = trace.solution_indices().iter().map(|&at| queens(&trace, at)).collect();
    assert_eq!(found, vec![vec![1, 3, 0, 2], vec![2, 0, 3, 1]]);
}

// The budget.

#[test]
fn a_search_that_needs_more_events_than_the_budget_is_an_error_and_not_a_short_trace() {
    // N=8 has 33589 events, so the event after the budget is refused, and no trace comes back to end in `Exhausted`.
    assert_eq!(Trace::build_within(8, 33_588).err(), Some(TraceError::EventBudgetExceeded));
    assert!(Trace::build_within(8, 33_589).is_ok());
    assert_eq!(Trace::build_within(8, 0).err(), Some(TraceError::EventBudgetExceeded));
}

#[test]
fn every_board_the_page_offers_fits_the_budget() {
    for n in 4..=9 {
        assert!(trace(n).len() <= EVENT_BUDGET, "N={n}");
    }
    assert!(count_only(10).map(|c| c.events).unwrap_or(0) > EVENT_BUDGET);
}

// The order of the events.

#[test]
fn the_search_starts_at_the_first_square_and_ends_once_with_exhausted() {
    for n in 1..=MAX_N {
        let trace = trace(n);
        assert_eq!(trace.event(0), None);
        assert_eq!(trace.event(1), Some(&SearchEvent::Try { row: 0, col: 0 }));
        assert_eq!(trace.event(trace.len()), Some(&SearchEvent::Exhausted));
        let exhausted = (1..=trace.len())
            .filter(|&i| trace.event(i) == Some(&SearchEvent::Exhausted))
            .count();
        assert_eq!(exhausted, 1, "N={n}");
    }
}

#[test]
fn every_try_is_followed_by_one_reject_or_place_for_the_same_square_and_nothing_else() {
    let trace = trace(8);
    let mut tries = 0;
    for index in 1..=trace.len() {
        let Some(&SearchEvent::Try { row, col }) = trace.event(index) else {
            continue;
        };
        tries += 1;
        match trace.event(index + 1) {
            Some(&SearchEvent::Reject { row: r, col: c, .. }) | Some(&SearchEvent::Place { row: r, col: c }) => {
                assert_eq!((r, c), (row, col), "at {index}");
            },
            other => panic!("a Try at {index} was followed by {other:?}"),
        }
    }
    assert!(tries > 0);
}

#[test]
fn a_complete_board_is_a_place_then_a_solution_then_a_remove_of_the_same_queen() {
    let trace = trace(8);
    for &at in trace.solution_indices() {
        let (Some(&SearchEvent::Place { row, col }), Some(&SearchEvent::Remove { row: r, col: c })) =
            (trace.event(at - 1), trace.event(at + 1))
        else {
            panic!("the solution at {at} is not between a Place and a Remove");
        };
        assert_eq!((row, col), (r, c), "at {at}");
        assert_eq!(usize::from(row), 7, "the last row completes a board");
    }
}

#[test]
fn every_place_is_removed_once_and_the_board_ends_empty() {
    for n in 1..=MAX_N {
        let trace = trace(n);
        let mut open: Vec<(u8, u8)> = Vec::new();
        for index in 1..=trace.len() {
            match trace.event(index) {
                Some(&SearchEvent::Place { row, col }) => open.push((row, col)),
                Some(&SearchEvent::Remove { row, col }) => {
                    assert_eq!(open.pop(), Some((row, col)), "N={n} at {index}");
                },
                _ => {},
            }
        }
        assert!(open.is_empty(), "N={n}");
        assert_eq!(trace.snapshot(trace.len()), Some(&CompactSnapshot::EMPTY), "N={n}");
    }
}

#[test]
fn the_search_tries_rows_and_columns_in_ascending_order() {
    let trace = trace(6);
    let mut last_try: Option<(u8, u8)> = None;
    for index in 1..=trace.len() {
        if let Some(&SearchEvent::Try { row, col }) = trace.event(index) {
            if let Some((r, c)) = last_try {
                // The next square is in the next column of the same row, or the first column of a later row, or the
                // search has gone back to a row above.
                assert!(
                    (row == r && col == c + 1) || (row == r + 1 && col == 0) || row <= r,
                    "at {index}"
                );
            }
            last_try = Some((row, col));
        }
    }
}

// The reason for a rejection.

#[test]
fn the_reason_for_a_rejection_is_the_first_of_column_main_diagonal_anti_diagonal() {
    // A queen at (0, 1). The square (1, 1) is under it, (1, 0) is on its anti-diagonal and (1, 2) is on its main one.
    assert_eq!(conflicts(&[1], 1, 1), Some((Conflict::Column, 1)));
    assert_eq!(conflicts(&[1], 1, 0), Some((Conflict::AntiDiagonal, 1)));
    assert_eq!(conflicts(&[1], 1, 2), Some((Conflict::MainDiagonal, 1)));
    assert_eq!(conflicts(&[1], 1, 3), None);
}

#[test]
fn directions_count_the_lines_and_not_the_queens() {
    // Queens at (0, 2), (1, 0) and (2, 4). The square (3, 2) is under the first, and on the main diagonal of the
    // second.
    assert_eq!(conflicts(&[2, 0, 4], 3, 2), Some((Conflict::Column, 2)));
    // Two queens on the same diagonal still count as one direction.
    assert_eq!(conflicts(&[0, 1], 2, 2), Some((Conflict::MainDiagonal, 1)));
    // Queens at (0, 3), (1, 1) and (2, 4) are on the column, the main diagonal and the anti-diagonal of (3, 3).
    assert_eq!(conflicts(&[3, 1, 4], 3, 3), Some((Conflict::Column, 3)));
}

#[test]
fn a_rejection_names_the_reason_and_the_directions_that_a_recount_finds() {
    let trace = trace(8);
    let mut rejections = 0;
    for index in 1..=trace.len() {
        let Some(&SearchEvent::Reject { row, col, reason, directions }) = trace.event(index) else {
            continue;
        };
        rejections += 1;
        // The board before the rejection is the one that made the Try, which is the snapshot just before.
        let before = trace.snapshot(index - 1).expect("the snapshot before");
        assert_eq!(
            conflicts(&before.positions, usize::from(row), usize::from(col)),
            Some((reason, directions)),
            "at {index}"
        );
        assert!((1..=3).contains(&directions));
    }
    assert!(rejections > 0);
}

#[test]
fn a_place_is_made_only_on_a_square_that_nothing_attacks() {
    let trace = trace(8);
    for index in 1..=trace.len() {
        if let Some(&SearchEvent::Place { row, col }) = trace.event(index) {
            let before = trace.snapshot(index - 1).expect("the snapshot before");
            assert_eq!(
                conflicts(&before.positions, usize::from(row), usize::from(col)),
                None,
                "at {index}"
            );
        }
    }
}

// The snapshots.

#[test]
fn snapshot_zero_is_empty_and_there_is_one_more_snapshot_than_event() {
    let trace = trace(5);
    assert_eq!(trace.snapshot(0), Some(&CompactSnapshot::EMPTY));
    assert!(trace.snapshot(trace.len()).is_some());
    assert_eq!(trace.snapshot(trace.len() + 1), None);
}

#[test]
fn a_snapshot_has_a_queen_in_each_row_above_its_depth_and_nowhere_else() {
    for n in [4, 8, 9] {
        let trace = trace(n);
        for index in 0..=trace.len() {
            let snapshot = trace.snapshot(index).expect("the snapshot exists");
            let depth = usize::from(snapshot.depth);
            assert!(depth <= n, "N={n} at {index}");
            for row in 0..MAX_N {
                if row < depth {
                    assert!(snapshot.positions[row] < n as u8, "N={n} at {index} row {row}");
                } else {
                    assert_eq!(snapshot.positions[row], UNPLACED, "N={n} at {index} row {row}");
                }
            }
            assert!(is_legal(&queens(&trace, index)), "N={n} at {index}");
        }
    }
}

#[test]
fn a_snapshot_changes_only_for_a_place_or_a_remove() {
    let trace = trace(7);
    for index in 1..=trace.len() {
        let same = trace.snapshot(index) == trace.snapshot(index - 1);
        match trace.event(index) {
            Some(SearchEvent::Place { .. } | SearchEvent::Remove { .. }) => assert!(!same, "at {index}"),
            _ => assert!(same, "at {index}"),
        }
    }
}

#[test]
fn a_solution_snapshot_has_every_queen_in_a_legal_board() {
    for n in [4, 8, 9] {
        let trace = trace(n);
        for &at in trace.solution_indices() {
            let found = queens(&trace, at);
            assert_eq!(found.len(), n);
            assert!(is_legal(&found));
        }
    }
}

#[test]
fn the_solutions_are_all_different_and_found_in_ascending_order() {
    let trace = trace(8);
    let found: Vec<Vec<usize>> = trace.solution_indices().iter().map(|&at| queens(&trace, at)).collect();
    assert!(found.windows(2).all(|pair| pair[0] < pair[1]));
}

// Where the solutions are.

#[test]
fn the_solution_indices_are_sorted_and_name_the_solution_events() {
    for n in [4, 8, 9] {
        let trace = trace(n);
        assert!(trace.solution_indices().windows(2).all(|pair| pair[0] < pair[1]));
        for &at in trace.solution_indices() {
            assert_eq!(trace.event(at), Some(&SearchEvent::Solution));
        }
    }
}

#[test]
fn the_ordinal_and_the_count_found_so_far_follow_the_solutions() {
    let trace = trace(4);
    let [first, second] = [trace.solution_indices()[0], trace.solution_indices()[1]];
    assert_eq!(trace.solutions_found(0), 0);
    assert_eq!(trace.solutions_found(first - 1), 0);
    assert_eq!(trace.solutions_found(first), 1);
    assert_eq!(trace.solutions_found(second - 1), 1);
    assert_eq!(trace.solutions_found(trace.len()), 2);
    assert_eq!(trace.solution_ordinal(first), Some(1));
    assert_eq!(trace.solution_ordinal(second), Some(2));
    assert_eq!(trace.solution_ordinal(first + 1), None);
    assert_eq!(trace.solution_ordinal(0), None);
}

#[test]
fn the_next_and_previous_solution_are_strictly_after_and_before() {
    let trace = trace(4);
    let [first, second] = [trace.solution_indices()[0], trace.solution_indices()[1]];
    assert_eq!(trace.next_solution(0), Some(first));
    assert_eq!(trace.next_solution(first - 1), Some(first));
    assert_eq!(
        trace.next_solution(first),
        Some(second),
        "from a solution, the next one and not the same"
    );
    assert_eq!(trace.next_solution(second), None);
    assert_eq!(trace.previous_solution(second), Some(first), "from a solution, the earlier one");
    assert_eq!(trace.previous_solution(first), None);
    assert_eq!(trace.previous_solution(trace.len()), Some(second));
    assert_eq!(trace.previous_solution(0), None);
}

#[test]
fn every_solution_of_eight_and_nine_can_be_reached_by_jumping() {
    for (n, expected) in [(8, 92), (9, 352)] {
        let trace = trace(n);
        let mut at = 0;
        let mut jumps = 0;
        while let Some(next) = trace.next_solution(at) {
            at = next;
            jumps += 1;
        }
        assert_eq!(jumps, expected, "N={n}");
        let mut back = 0;
        while let Some(before) = trace.previous_solution(at) {
            at = before;
            back += 1;
        }
        assert_eq!(back, expected - 1, "N={n}");
    }
}

// What a trace takes.

#[test]
fn a_snapshot_and_an_event_are_small() {
    assert_eq!(size_of::<CompactSnapshot>(), 10);
    assert!(size_of::<SearchEvent>() <= 8, "{}", size_of::<SearchEvent>());
}

#[test]
fn the_largest_trace_the_page_offers_takes_a_few_megabytes() {
    let bytes = trace(9).allocated_bytes();
    // About 153503 snapshots of 10 bytes, 153502 events of a few, and the indices of 352 solutions.
    assert!(bytes > 1_500_000 && bytes < 8_000_000, "{bytes}");
}
