//! That every route is stable and distinct, that the six curves between one pair of vertices are six, and that no two
//! labels meet in either graph at any step.

use super::*;
use crate::edmonds_karp::{
    fixtures::{CANONICAL, FIXTURES, REVERSE, VIEW_HEIGHT, VIEW_WIDTH},
    projection::{Prefs, Speed, View, present},
    solver::Snapshot,
};

fn trace_of(fixture: &Fixture) -> Trace {
    Trace::build(&fixture.graph()).expect("a trace")
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The box that the text of a label fills, as its left, top, right and bottom.
fn label_box(at: (f64, f64), text: &str) -> (f64, f64, f64, f64) {
    let half = text.chars().count() as f64 * LABEL_CHARACTER / 2.0 + 2.0;
    (at.0 - half, at.1 - LABEL_HEIGHT / 2.0, at.0 + half, at.1 + LABEL_HEIGHT / 2.0)
}

fn meet(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// The snapshot of the canonical graph with the flows that make all four residual lanes between v1 and v2 positive:
/// `e2` at 3 of 10, and `e3` at 2 of 4. No run gives this state, and conservation does not hold in it.
fn six_lane_state(trace: &Trace) -> Snapshot {
    let mut snapshot = trace.snapshot(0).expect("a snapshot").clone();
    snapshot.flows[2] = 3;
    snapshot.flows[3] = 2;
    snapshot
}

#[test]
fn every_vertex_edge_and_lane_of_both_graphs_has_a_route_inside_the_picture() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        let g = geometry(fixture, &trace);
        assert_eq!(
            (g.vertices.len(), g.edges.len(), g.lanes.len()),
            (6, trace.edges().len(), trace.edges().len() * 2)
        );
        for point in g
            .vertices
            .iter()
            .chain(g.edges.iter().map(|r| &r.label))
            .chain(g.lanes.iter().map(|r| &r.label))
        {
            assert!(
                (0.0..=VIEW_WIDTH).contains(&point.0) && (0.0..=VIEW_HEIGHT).contains(&point.1),
                "{}: {point:?} is inside the picture",
                fixture.key
            );
        }
    }
}

#[test]
fn a_route_does_not_depend_on_the_flow() {
    // The geometry is a function of the graph alone, so a lane that has no room can be hidden and nothing moves.
    let trace = trace_of(&CANONICAL);
    assert_eq!(geometry(&CANONICAL, &trace), geometry(&CANONICAL, &trace));
}

#[test]
fn the_two_edges_between_v1_and_v2_are_on_opposite_sides_of_the_line() {
    let trace = trace_of(&CANONICAL);
    let g = geometry(&CANONICAL, &trace);
    // v1 and v2 are both at x=260, so a curve to one side of the line has its middle at a different x.
    let (e2, e3) = (&g.edges[2], &g.edges[3]);
    assert!(
        e2.mid.0 < 260.0 && e3.mid.0 > 260.0 || e2.mid.0 > 260.0 && e3.mid.0 < 260.0,
        "{:?} {:?}",
        e2.mid,
        e3.mid
    );
    assert!(distance(e2.mid, e3.mid) >= EDGE_STEP - 1.0);
}

#[test]
fn between_v1_and_v2_the_two_edges_and_four_lanes_are_six_distinct_curves() {
    let trace = trace_of(&CANONICAL);
    let g = geometry(&CANONICAL, &trace);
    let mut curves = vec![g.edges[2].clone(), g.edges[3].clone()];
    for e in [2, 3] {
        for direction in [Direction::Forward, Direction::Reverse] {
            curves.push(g.lane(e, direction).clone());
        }
    }
    for (i, a) in curves.iter().enumerate() {
        for b in &curves[i + 1..] {
            assert_ne!(a.d, b.d, "two curves have one path");
            assert!(
                distance(a.mid, b.mid) >= LANE_STEP - 1.0,
                "{:?} and {:?} are too close",
                a.mid,
                b.mid
            );
        }
    }
    assert_eq!(curves.len(), 6);
}

#[test]
fn the_six_labels_of_that_state_do_not_meet_and_the_lanes_are_the_pinned_residuals() {
    let trace = trace_of(&CANONICAL);
    let g = geometry(&CANONICAL, &trace);
    let snapshot = six_lane_state(&trace);
    let view = View::first(Prefs::default(), Speed::Two);
    let shown = present(&trace, &snapshot, &view);
    let slots = crate::edmonds_karp::projection::Slots::new(6, 10);
    let lane = |e, d| &shown.slots[slots.lane(e, d)];
    let label = |e, d| shown.slots[slots.lane_label(e, d)].text.clone().unwrap_or_default();
    let visible = |e, d| !lane(e, d).class.contains("ek-off");
    for (e, d, room) in [
        (2, Direction::Forward, "7"),
        (2, Direction::Reverse, "rev 3"),
        (3, Direction::Forward, "2"),
        (3, Direction::Reverse, "rev 2"),
    ] {
        assert!(visible(e, d));
        assert_eq!(label(e, d), room);
    }
    assert_eq!(shown.slots[slots.edge_label(2)].text.as_deref(), Some("3/10"));
    assert_eq!(shown.slots[slots.edge_label(3)].text.as_deref(), Some("2/4"));
    let mut boxes: Vec<(String, (f64, f64, f64, f64))> = vec![
        ("e2".into(), label_box(g.edges[2].label, "3/10")),
        ("e3".into(), label_box(g.edges[3].label, "2/4")),
    ];
    for (e, d, text) in [
        (2, Direction::Forward, "7"),
        (2, Direction::Reverse, "rev 3"),
        (3, Direction::Forward, "2"),
        (3, Direction::Reverse, "rev 2"),
    ] {
        boxes.push((format!("lane {e}{}", d.letter()), label_box(g.lane(e, d).label, text)));
    }
    for (i, (a, ba)) in boxes.iter().enumerate() {
        for (b, bb) in &boxes[i + 1..] {
            assert!(!meet(*ba, *bb), "the labels of {a} and {b} meet");
        }
    }
}

#[test]
fn no_two_labels_meet_at_any_step_of_either_graph_with_the_residual_lanes_shown() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        let g = geometry(fixture, &trace);
        let view = View::first(Prefs::default(), Speed::Two);
        let slots = crate::edmonds_karp::projection::Slots::new(trace.vertex_count(), trace.edges().len());
        for snapshot in trace.snapshots() {
            let shown = present(&trace, snapshot, &view);
            let mut boxes: Vec<(String, (f64, f64, f64, f64))> = Vec::new();
            for e in 0..trace.edges().len() {
                let text = shown.slots[slots.edge_label(e)].text.clone().unwrap_or_default();
                boxes.push((format!("e{e}"), label_box(g.edges[e].label, &text)));
                for d in [Direction::Forward, Direction::Reverse] {
                    if shown.slots[slots.lane(e, d)].class.contains("ek-off") {
                        continue;
                    }
                    let text = shown.slots[slots.lane_label(e, d)].text.clone().unwrap_or_default();
                    boxes.push((format!("lane e{e}{}", d.letter()), label_box(g.lane(e, d).label, &text)));
                }
            }
            for (i, (a, ba)) in boxes.iter().enumerate() {
                for (b, bb) in &boxes[i + 1..] {
                    assert!(
                        !meet(*ba, *bb),
                        "{} step {}: the labels of {a} and {b} meet",
                        fixture.key,
                        snapshot.index
                    );
                }
            }
        }
    }
}

#[test]
fn a_label_is_not_over_a_vertex() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        let g = geometry(fixture, &trace);
        for route in g.edges.iter().chain(&g.lanes) {
            for &vertex in &g.vertices {
                assert!(
                    distance(route.label, vertex) > NODE_RADIUS + 6.0,
                    "{}: a label is on a vertex",
                    fixture.key
                );
            }
        }
    }
}

#[test]
fn the_reverse_graph_has_no_crossing_of_its_original_edges() {
    // The layout was chosen so that none of the seven edges crosses another.
    let trace = trace_of(&REVERSE);
    let g = geometry(&REVERSE, &trace);
    let ends: Vec<((f64, f64), (f64, f64))> =
        trace.edges().iter().map(|e| (g.vertices[e.from], g.vertices[e.to])).collect();
    let side = |p: (f64, f64), a: (f64, f64), b: (f64, f64)| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            let (a, b) = (ends[i], ends[j]);
            let shared = [a.0, a.1].iter().any(|p| *p == b.0 || *p == b.1);
            if shared {
                continue;
            }
            let crosses =
                side(b.0, a.0, a.1) * side(b.1, a.0, a.1) < 0.0 && side(a.0, b.0, b.1) * side(a.1, b.0, b.1) < 0.0;
            assert!(!crosses, "e{i} crosses e{j}");
        }
    }
}

#[test]
fn a_route_starts_and_ends_on_the_edge_of_its_vertices() {
    let trace = trace_of(&CANONICAL);
    let g = geometry(&CANONICAL, &trace);
    for (e, edge) in trace.edges().iter().enumerate() {
        let numbers: Vec<f64> = g.edges[e]
            .d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().expect("a number"))
            .collect();
        let (start, end) = (
            (numbers[0], numbers[1]),
            (numbers[numbers.len() - 2], numbers[numbers.len() - 1]),
        );
        assert!(
            (distance(start, g.vertices[edge.from]) - NODE_RADIUS).abs() < 0.2,
            "e{e} starts on its vertex"
        );
        assert!(
            (distance(end, g.vertices[edge.to]) - NODE_RADIUS).abs() < 0.2,
            "e{e} ends on its vertex"
        );
    }
}
