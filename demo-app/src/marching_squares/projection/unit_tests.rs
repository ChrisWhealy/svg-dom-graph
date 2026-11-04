//! The picture that a state gives, whatever way it was reached.

use super::*;
use crate::marching_squares::field::LabCase;

fn ripple() -> Trace {
    Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace")
}

fn state(step: usize) -> State {
    State {
        step,
        ..State::first(FieldId::Ripple)
    }
}

#[test]
fn the_first_snapshot_has_no_contour_and_no_cell() {
    let p = project(&ripple(), &state(0));
    assert!(p.lines.is_empty() && p.ambiguous.is_empty());
    assert_eq!((p.cell, p.inspected.clone()), (None, Inspected::NoCell));
    assert!(p.status.starts_with("No cell selected."), "{}", p.status);
    assert!(!p.complete);
    assert!(p.description.contains("partial"));
}

#[test]
fn a_cell_that_was_visited_follows_the_scan_until_one_is_chosen() {
    let t = ripple();
    let across = t.grid.cells_across();
    let p = project(&t, &state(across + 3));
    assert_eq!(p.cell, Some((1, 2)), "the last visited is index across + 2");
    assert!(matches!(p.inspected, Inspected::Cell(ref v) if (v.row, v.col) == (1, 2)));
    let chosen = project(
        &t,
        &State {
            selected: Some((10, 10)),
            ..state(across + 3)
        },
    );
    assert_eq!(chosen.cell, Some((10, 10)));
}

#[test]
fn a_cell_that_has_not_been_visited_reveals_nothing() {
    let t = ripple();
    let p = project(
        &t,
        &State {
            selected: Some((14, 12)),
            ..state(50)
        },
    );
    assert_eq!(p.inspected, Inspected::NotYet { row: 14, col: 12 });
    assert!(p.status.contains("Not yet evaluated"), "{}", p.status);
    for hidden in ["mask", "Q ", "rho", "T–"] {
        assert!(!p.status.contains(hidden), "{hidden}: {}", p.status);
    }
}

#[test]
fn the_cell_is_evaluated_from_the_step_after_it_is_visited() {
    let t = ripple();
    let across = t.grid.cells_across();
    let index = 14 * across + 12;
    let chosen = |step| {
        project(
            &t,
            &State {
                selected: Some((14, 12)),
                inspected: 1,
                ..state(step)
            },
        )
    };
    assert!(matches!(chosen(index).inspected, Inspected::NotYet { .. }));
    let Inspected::Cell(view) = chosen(index + 1).inspected else {
        panic!("evaluated")
    };
    assert_eq!((view.result.mask, view.slot), (10, 1));
    assert!(chosen(index + 1).status.contains("Q negative"), "{}", chosen(index + 1).status);
    assert!(chosen(index + 1).status.contains("rho 0.425"), "{}", chosen(index + 1).status);
}

#[test]
fn only_the_levels_that_are_shown_are_drawn_and_totals_cover_all_three() {
    let t = ripple();
    let all = project(&t, &state(432));
    let one = project(
        &t,
        &State {
            show: Show::One(2),
            ..state(432)
        },
    );
    let slots = |p: &Projection| p.lines.iter().map(|l| l.slot).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(slots(&all).len(), 3);
    assert_eq!(slots(&one), std::collections::BTreeSet::from([2]));
    assert_eq!(all.totals, one.totals, "the readout is the same whichever levels are shown");
    assert!(all.complete && all.status.contains("Complete."));
    assert!(all.status.contains("low:") && all.status.contains("high:"), "{}", all.status);
}

#[test]
fn a_loop_is_drawn_with_its_first_point_repeated() {
    let t = Trace::build(FieldId::Twin.grid().expect("grid"), FieldId::Twin.levels()).expect("trace");
    let p = project(
        &t,
        &State {
            step: 432,
            ..State::first(FieldId::Twin)
        },
    );
    let closed: Vec<&Line> = p.lines.iter().filter(|l| l.closed).collect();
    assert!(!closed.is_empty());
    assert!(closed.iter().all(|l| l.points.first() == l.points.last()));
    assert!(
        p.lines
            .iter()
            .filter(|l| !l.closed)
            .all(|l| l.points.first() != l.points.last())
    );
}

#[test]
fn the_same_state_gives_the_same_projection_however_it_was_reached() {
    let t = ripple();
    let direct = project(&t, &state(200));
    let walked = (0..=432)
        .rev()
        .take_while(|&k| k >= 200)
        .map(|k| project(&t, &state(k)))
        .last()
        .expect("walked");
    assert!(direct == walked);
    assert!(project(&t, &state(200)) == direct);
}

#[test]
fn the_ambiguous_cells_are_outlined_once_visited() {
    let t = ripple();
    let across = t.grid.cells_across();
    // Visited in row-major order: (4, 1), then (13, 22), then (14, 12).
    let before = project(&t, &state(13 * across + 22));
    assert_eq!(before.ambiguous, vec![(4, 1)]);
    let after = project(&t, &state(13 * across + 23));
    assert_eq!(after.ambiguous, vec![(4, 1), (13, 22)]);
    assert_eq!(
        project(&t, &state(14 * across + 13)).ambiguous,
        vec![(4, 1), (13, 22), (14, 12)]
    );
    let hidden = project(
        &t,
        &State {
            show: Show::One(0),
            ..state(432)
        },
    );
    assert_eq!(hidden.ambiguous, vec![(4, 1)], "only the levels that are shown");
}

#[test]
fn the_saddle_lab_reads_each_named_case() {
    for (case, text) in [
        (LabCase::FivePositive, "Q positive"),
        (LabCase::FiveNegative, "Q negative"),
        (LabCase::TenPositive, "Q positive"),
        (LabCase::TenNegative, "Q negative"),
        (LabCase::Tie, "Q exactly zero"),
    ] {
        let field = FieldId::Lab(case);
        let t = Trace::build(field.grid().expect("grid"), field.levels()).expect("trace");
        let p = project(&t, &State { step: 1, ..State::first(field) });
        assert!(p.status.contains(text), "{case:?}: {}", p.status);
        assert!(p.complete);
    }
    let tie = FieldId::Lab(LabCase::Tie);
    let t = Trace::build(tie.grid().expect("grid"), tie.levels()).expect("trace");
    let p = project(&t, &State { step: 1, ..State::first(tie) });
    assert_eq!(p.totals[1], Totals { open: 2, closed: 0, ties: 1 });
    assert!(p.status.contains("1 saddle ties"), "{}", p.status);
}

#[test]
fn the_text_names_the_level_by_its_slot_and_its_sign() {
    let t = ripple();
    let across = t.grid.cells_across();
    let p = project(
        &t,
        &State {
            selected: Some((13, 22)),
            inspected: 2,
            ..state(13 * across + 23)
        },
    );
    assert!(
        p.status.starts_with(
            "Cell (13, 22), high level +0.520: mask 5; Q positive; rho 0.416; T–R and B–L; 3 levels visible."
        ),
        "{}",
        p.status
    );
    assert_eq!(level_text(-0.53), "-0.530");
    assert!(p.description.contains("high +0.520 dotted"));
}

#[test]
fn a_step_beyond_the_end_is_the_end() {
    let t = ripple();
    assert!(project(&t, &state(9999)) == project(&t, &state(432)));
}

#[test]
fn the_show_keys_round_trip() {
    for show in [Show::All, Show::One(0), Show::One(1), Show::One(2)] {
        assert_eq!(Show::from_key(show.key()), Some(show));
    }
    assert_eq!(Show::from_key("x"), None);
}
