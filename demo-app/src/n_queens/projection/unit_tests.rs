//! What each snapshot shows: the outline, the rays, the frame, the name of every square, the status, the geometry of
//! the rays, and where the keys move the active square.

use super::*;

fn trace(n: usize) -> Trace {
    Trace::build(n).expect("a board this size can be traced")
}

fn view(trace: &Trace, index: usize) -> View {
    project(trace, index).expect("the snapshot exists")
}

/// The number pairs of a path of the form `M x y L x y`.
fn ends(path: &str) -> [(f64, f64); 2] {
    let numbers: Vec<f64> = path
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().expect("a number"))
        .collect();
    [(numbers[0], numbers[1]), (numbers[2], numbers[3])]
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

// The first steps of four queens: Try (0,0), Place, Try (1,0), Reject column, Try (1,1), Reject main, Try (1,2) ...

#[test]
fn the_first_snapshot_is_an_empty_board_that_shows_nothing() {
    let trace = trace(4);
    let first = view(&trace, 0);
    assert_eq!(first.kind, Kind::Initial);
    assert_eq!((first.outline(), first.rays(), first.frame()), (None, None, false));
    assert_eq!(first.busy_cells(), vec![]);
    assert_eq!(status(&first).cell, NOTHING);
}

#[test]
fn a_try_outlines_the_candidate_and_draws_three_neutral_rays() {
    let trace = trace(4);
    let tried = view(&trace, 1);
    assert_eq!(tried.kind, Kind::Try);
    assert_eq!(
        tried.outline(),
        Some(Outline {
            kind: OutlineKind::Candidate,
            row: 0,
            col: 0
        })
    );
    assert_eq!(tried.rays(), Some(Rays { row: 0, col: 0, chosen: None }));
    assert_eq!(tried.cell_state(0, 0), CellState::Candidate);
    assert_eq!(status(&tried).reason, NOTHING, "a Try says nothing of a conflict");
}

#[test]
fn a_place_shows_the_queen_and_no_outline_or_rays() {
    let trace = trace(4);
    let placed = view(&trace, 2);
    assert_eq!(placed.kind, Kind::Place);
    assert_eq!((placed.outline(), placed.rays()), (None, None));
    assert_eq!(placed.cell_state(0, 0), CellState::Queen);
    assert_eq!(placed.busy_cells(), vec![(0, 0)]);
}

#[test]
fn a_reject_keeps_the_rays_marks_the_reason_and_outlines_the_square_as_rejected() {
    let trace = trace(4);
    // Index 4 is the Reject of (1, 0), which is under the queen at (0, 0).
    let rejected = view(&trace, 4);
    assert_eq!(rejected.kind, Kind::Reject);
    assert_eq!(rejected.outline().map(|o| o.kind), Some(OutlineKind::Rejected));
    assert_eq!(
        rejected.rays(),
        Some(Rays {
            row: 1,
            col: 0,
            chosen: Some(Conflict::Column)
        })
    );
    assert_eq!(rejected.cell_state(1, 0), CellState::Rejected(Conflict::Column));
    assert_eq!(status(&rejected).reason, "column");
}

#[test]
fn the_rays_of_a_try_are_the_same_whether_the_square_turns_out_to_be_free_or_not() {
    // Find a square that is tried twice in a board of eight, once to be refused and once to be accepted.
    let trace = trace(8);
    let mut refused = None;
    let mut accepted = None;
    for index in 1..trace.len() {
        let tried = view(&trace, index);
        if tried.kind != Kind::Try || tried.cell != Some((3, 3)) {
            continue;
        }
        match view(&trace, index + 1).kind {
            Kind::Reject => refused = refused.or(Some(tried)),
            Kind::Place => accepted = accepted.or(Some(tried)),
            _ => {},
        }
    }
    let (refused, accepted) = (refused.expect("refused"), accepted.expect("accepted"));
    assert_eq!(refused.rays(), accepted.rays());
    assert_eq!(refused.outline(), accepted.outline());
}

#[test]
fn a_solution_shows_the_frame_and_every_queen_and_no_outline_or_rays() {
    let trace = trace(4);
    let first = trace.next_solution(0).expect("a solution");
    let solved = view(&trace, first);
    assert_eq!(solved.kind, Kind::Solution);
    assert!(solved.frame());
    assert_eq!((solved.outline(), solved.rays()), (None, None));
    assert_eq!(solved.busy_cells().len(), 4);
    assert_eq!(solved.queens_one_based(), vec![2, 4, 1, 3]);
    assert_eq!(status(&solved).vector, "[2, 4, 1, 3]");
}

#[test]
fn a_remove_outlines_the_square_the_queen_left_and_shows_no_queen_there() {
    let trace = trace(4);
    let at = (1..=trace.len())
        .find(|&i| view(&trace, i).kind == Kind::Remove)
        .expect("a Remove");
    let removed = view(&trace, at);
    let outline = removed.outline().expect("an outline");
    assert_eq!(outline.kind, OutlineKind::Removed);
    assert_eq!(removed.cell_state(outline.row, outline.col), CellState::Removed);
    assert_eq!(removed.rays(), None);
}

#[test]
fn the_last_snapshot_is_exhausted_with_an_empty_board_and_nothing_drawn() {
    let trace = trace(5);
    let end = view(&trace, trace.len());
    assert_eq!(end.kind, Kind::Exhausted);
    assert_eq!(
        (end.outline(), end.rays(), end.frame(), end.busy_cells().len()),
        (None, None, false, 0)
    );
    assert_eq!(end.found, end.total);
}

#[test]
fn nothing_else_than_a_solution_has_a_frame_and_only_a_try_or_a_reject_has_rays() {
    let trace = trace(6);
    for index in 0..=trace.len() {
        let v = view(&trace, index);
        assert_eq!(v.frame(), v.kind == Kind::Solution, "at {index}");
        assert_eq!(v.rays().is_some(), matches!(v.kind, Kind::Try | Kind::Reject), "at {index}");
        assert_eq!(v.rays().and_then(|r| r.chosen).is_some(), v.kind == Kind::Reject, "at {index}");
    }
}

#[test]
fn a_queen_and_an_outline_are_never_on_the_same_square_and_a_square_has_one_name() {
    let trace = trace(8);
    for index in 0..=trace.len() {
        let v = view(&trace, index);
        if let Some(outline) = v.outline() {
            assert_ne!(v.cell_state(outline.row, outline.col), CellState::Queen, "at {index}");
            assert_eq!(v.snapshot.queen_in(outline.row), None, "at {index}");
        }
        let busy = v.busy_cells();
        let named = (0..8)
            .flat_map(|r| (0..8).map(move |c| (r, c)))
            .filter(|&(r, c)| v.cell_state(r, c) != CellState::Empty);
        assert_eq!(named.count(), busy.len(), "at {index}");
    }
}

#[test]
fn the_names_of_the_squares_describe_the_state_and_not_the_place() {
    assert_eq!(CellState::Empty.name(), "empty");
    assert_eq!(CellState::Queen.name(), "queen");
    assert_eq!(CellState::Candidate.name(), "candidate");
    assert_eq!(CellState::Rejected(Conflict::MainDiagonal).name(), "rejected: main diagonal");
    assert_eq!(CellState::Removed.name(), "removed");
}

// The status.

#[test]
fn the_status_is_written_one_based() {
    let trace = trace(4);
    let tried = status(&view(&trace, 3));
    assert_eq!(tried.position, format!("3/{}", trace.len()));
    assert_eq!(tried.kind, "Try");
    assert_eq!(tried.cell, "row 2, column 1");
    assert_eq!(tried.depth, "1 / 4");
    assert_eq!(tried.solutions, "0 / 2");
}

#[test]
fn a_reject_says_first_of_several_only_when_there_are_several() {
    assert_eq!(reason_text(Conflict::Column, 1), "column");
    assert_eq!(
        reason_text(Conflict::MainDiagonal, 2),
        "main diagonal (first of 2 conflicting directions)"
    );
    assert_eq!(reason_text(Conflict::Column, 3), "column (first of 3 conflicting directions)");
    let trace = trace(8);
    let found = (1..=trace.len()).map(|i| view(&trace, i)).filter(|v| v.kind == Kind::Reject);
    let (mut single, mut several) = (false, false);
    for v in found {
        let text = status(&v).reason;
        assert_eq!(text.contains("first of"), v.directions > 1, "{text}");
        single |= v.directions == 1;
        several |= v.directions > 1;
    }
    assert!(single && several, "a board of eight has both kinds of rejection");
}

#[test]
fn the_status_of_a_solution_gives_its_ordinal_and_the_progress() {
    let trace = trace(4);
    let second = trace.next_solution(trace.next_solution(0).expect("first")).expect("second");
    let text = status(&view(&trace, second));
    assert_eq!(text.solutions, "2 / 2 (solution 2)");
    assert_eq!(text.vector, "[3, 1, 4, 2]");
    assert_eq!(text.kind, "Solution");
}

#[test]
fn the_vector_is_shown_for_a_solution_only() {
    let trace = trace(5);
    for index in 0..=trace.len() {
        let v = view(&trace, index);
        assert_eq!(status(&v).vector != NOTHING, v.kind == Kind::Solution, "at {index}");
    }
}

#[test]
fn the_summary_says_what_happened_in_words() {
    let trace = trace(4);
    assert!(summary(&view(&trace, 0)).contains("empty"));
    assert!(summary(&view(&trace, 1)).contains("trying row 1, column 1"));
    assert!(summary(&view(&trace, 2)).contains("queen placed at row 1, column 1, 1 of 4"));
    assert!(summary(&view(&trace, 4)).contains("rejected, column"));
    assert!(summary(&view(&trace, trace.len())).contains("finished"));
    let solved = trace.next_solution(0).expect("a solution");
    assert!(summary(&view(&trace, solved)).contains("solution 1 of 2"));
}

#[test]
fn the_pause_summary_gives_the_step_the_depth_and_the_solutions() {
    let trace = trace(4);
    let text = pause_summary(&view(&trace, 2));
    assert!(text.starts_with("Paused at step 2 of"), "{text}");
    assert!(text.contains("1 of 4 queens") && text.contains("0 of 2 solutions"), "{text}");
}

#[test]
fn only_a_solution_and_the_end_are_announced_while_playing() {
    for kind in [Kind::Initial, Kind::Try, Kind::Reject, Kind::Place, Kind::Remove] {
        assert!(!announced_while_playing(kind), "{kind:?}");
    }
    assert!(announced_while_playing(Kind::Solution) && announced_while_playing(Kind::Exhausted));
}

// The ends.

#[test]
fn the_buttons_are_at_their_ends_at_the_first_and_last_snapshots() {
    let trace = trace(4);
    let first = view(&trace, 0).boundaries();
    assert!(first.previous && first.restart && !first.next && !first.play && !first.next_solution);
    assert!(first.previous_solution, "nothing is before the first snapshot");
    let last = view(&trace, trace.len()).boundaries();
    assert!(last.next && last.play && !last.previous && !last.restart && !last.previous_solution);
    assert!(last.next_solution, "nothing is after the last snapshot");
}

#[test]
fn the_solution_buttons_follow_the_solutions_strictly() {
    let trace = trace(4);
    let [first, second] = [
        trace.next_solution(0).expect("first"),
        trace.next_solution(first_of(&trace)).expect("second"),
    ];
    let at_first = view(&trace, first).boundaries();
    assert!(at_first.previous_solution && !at_first.next_solution);
    let at_second = view(&trace, second).boundaries();
    assert!(!at_second.previous_solution && at_second.next_solution);
}

fn first_of(trace: &Trace) -> usize {
    trace.next_solution(0).expect("a solution")
}

#[test]
fn the_seek_text_names_the_step_and_the_kind() {
    let trace = trace(4);
    assert_eq!(seek_text(&view(&trace, 0)), format!("Step 0 of {}, Initial", trace.len()));
}

// The geometry of the rays.

#[test]
fn the_column_ray_runs_from_the_top_of_the_board_to_the_bottom() {
    // Four squares of 120 pixels and a margin of 12: the centre of column 2 is at 12 + 2.5 * 120.
    let [from, to] = ends(&ray_path(4, 1, 2, Conflict::Column));
    assert!(near(from.0, 312.0) && near(to.0, 312.0));
    assert!(near(from.1, MARGIN) && near(to.1, MARGIN + BOARD_SIZE));
}

#[test]
fn the_diagonal_rays_run_corner_to_corner_through_the_centre_of_the_square() {
    let [from, to] = ends(&ray_path(4, 0, 0, Conflict::MainDiagonal));
    assert!(near(from.0, MARGIN) && near(from.1, MARGIN));
    assert!(near(to.0, MARGIN + BOARD_SIZE) && near(to.1, MARGIN + BOARD_SIZE));
    let [from, to] = ends(&ray_path(4, 0, 0, Conflict::AntiDiagonal));
    assert!(near(from.0, MARGIN) && near(from.1, MARGIN + 120.0));
    assert!(near(to.0, MARGIN + 120.0) && near(to.1, MARGIN));
}

#[test]
fn every_ray_of_every_square_stays_on_the_board_and_passes_through_the_square() {
    for size in [4, 8, 9] {
        let cell = cell_size(size);
        for row in 0..size {
            for col in 0..size {
                let (cx, cy) = (MARGIN + (col as f64 + 0.5) * cell, MARGIN + (row as f64 + 0.5) * cell);
                for line in [Conflict::Column, Conflict::MainDiagonal, Conflict::AntiDiagonal] {
                    let [(x0, y0), (x1, y1)] = ends(&ray_path(size, row, col, line));
                    for (x, y) in [(x0, y0), (x1, y1)] {
                        assert!(
                            (MARGIN - 0.01..=MARGIN + BOARD_SIZE + 0.01).contains(&x),
                            "{size} {row} {col} {line:?}"
                        );
                        assert!(
                            (MARGIN - 0.01..=MARGIN + BOARD_SIZE + 0.01).contains(&y),
                            "{size} {row} {col} {line:?}"
                        );
                    }
                    // The centre is on the segment: its distance from the line is nothing, to the rounding of the path.
                    let length = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
                    let distance = ((x1 - x0) * (cy - y0) - (y1 - y0) * (cx - x0)).abs() / length;
                    assert!(distance < 0.01, "{size} {row} {col} {line:?}: {distance}");
                    let within = (cx - x0) * (x1 - cx) >= -0.01 && (cy - y0) * (y1 - cy) >= -0.01 || near(x0, x1);
                    assert!(within, "{size} {row} {col} {line:?}");
                }
            }
        }
    }
}

#[test]
fn a_ray_does_not_depend_on_the_queens() {
    // The path is a function of the board size and the square alone.
    assert_eq!(ray_path(8, 3, 3, Conflict::Column), ray_path(8, 3, 3, Conflict::Column));
}

// The keys.

#[test]
fn the_arrow_keys_move_one_square_and_stop_at_the_edge() {
    assert_eq!(move_active("ArrowRight", false, (2, 2), 8), Some((2, 3)));
    assert_eq!(move_active("ArrowLeft", false, (2, 2), 8), Some((2, 1)));
    assert_eq!(move_active("ArrowDown", false, (2, 2), 8), Some((3, 2)));
    assert_eq!(move_active("ArrowUp", false, (2, 2), 8), Some((1, 2)));
    assert_eq!(move_active("ArrowLeft", false, (2, 0), 8), Some((2, 0)), "there is no wrap");
    assert_eq!(move_active("ArrowUp", false, (0, 5), 8), Some((0, 5)));
    assert_eq!(move_active("ArrowRight", false, (2, 7), 8), Some((2, 7)));
    assert_eq!(move_active("ArrowDown", false, (7, 5), 8), Some((7, 5)));
}

#[test]
fn home_and_end_go_to_the_ends_of_the_row_and_with_control_to_the_corners() {
    assert_eq!(move_active("Home", false, (3, 4), 8), Some((3, 0)));
    assert_eq!(move_active("End", false, (3, 4), 8), Some((3, 7)));
    assert_eq!(move_active("Home", true, (3, 4), 8), Some((0, 0)));
    assert_eq!(move_active("End", true, (3, 4), 8), Some((7, 7)));
}

#[test]
fn page_keys_enter_and_space_are_left_alone() {
    for key in ["PageUp", "PageDown", "Enter", " ", "a"] {
        assert_eq!(move_active(key, false, (3, 4), 8), None, "{key}");
        assert_eq!(move_active(key, true, (3, 4), 8), None, "{key}");
    }
    assert_eq!(move_active("ArrowRight", false, (0, 0), 0), None);
}

#[test]
fn a_square_off_a_smaller_board_is_moved_onto_it() {
    assert_eq!(clamp_cell((8, 8), 4), (3, 3));
    assert_eq!(clamp_cell((1, 2), 4), (1, 2));
}

// The page.

#[test]
fn the_panel_has_every_legend_entry_and_every_status_field() {
    let html = include_str!("../../../../demo/panels/14-backtracking/01-n-queens.html");
    for entry in [
        "Placed queen",
        "Candidate (Try)",
        "Rejected candidate",
        "Recently removed queen",
        "Candidate rays",
        "Chosen conflict ray",
        "Solution frame",
        "Keyboard focus ring",
    ] {
        assert!(html.contains(entry), "the legend has no {entry:?}");
    }
    for field in [
        "Step", "Event", "Active cell", "Queens placed", "Reason", "Solutions", "Solution", "Playback",
    ] {
        assert!(html.contains(&format!(">{field}<")), "the status has no {field:?}");
    }
}
