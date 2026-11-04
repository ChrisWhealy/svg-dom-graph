use super::*;
use crate::tarjan::{fixture, solver::Trace};

fn graph() -> Graph {
    fixture::graph()
}

fn trace() -> Trace {
    Trace::build(&graph()).unwrap()
}

fn at(trace: &Trace, k: usize) -> Condensation {
    Condensation::of(&graph(), trace.snapshot(k).unwrap())
}

#[test]
fn nothing_is_emitted_at_the_start_so_the_condensation_is_empty() {
    let c = at(&trace(), 0);
    assert!(c.nodes.is_empty() && c.arcs.is_empty());
    assert_eq!(c.placements(), Ok(vec![]));
    assert!(!c.has_cycle());
}

#[test]
fn the_final_condensation_has_ten_nodes_and_seventeen_arcs_and_no_loop_duplicate_or_cycle() {
    let c = at(&trace(), 252);
    assert_eq!((c.nodes.len(), c.arcs.len()), (10, 17));
    assert!(c.arcs.iter().all(|&(a, b)| a != b), "no loop");
    let mut unique = c.arcs.clone();
    unique.dedup();
    assert_eq!(unique.len(), 17, "no pair twice");
    assert!(c.arcs.windows(2).all(|w| w[0] < w[1]), "sorted");
    assert!(!c.has_cycle());
    assert!(
        c.arcs.iter().all(|&(from, to)| from > to),
        "every arrow goes from a later component to an earlier one"
    );
    let covered: usize = c.nodes.iter().map(|n| n.members.len()).sum();
    assert_eq!(covered, 64, "every vertex is in exactly one component");
}

#[test]
fn at_every_snapshot_the_condensation_is_within_its_bounds_and_acyclic() {
    let trace = trace();
    for k in 0..=trace.last() {
        let c = at(&trace, k);
        assert!(c.nodes.len() <= 10 && c.arcs.len() <= 17, "snapshot {k}");
        assert!(!c.has_cycle(), "snapshot {k}");
        assert!(c.arcs.iter().all(|&(a, b)| a != b));
    }
}

#[test]
fn an_arrow_appears_only_when_both_of_its_components_are_known() {
    let trace = trace();
    for k in 0..=trace.last() {
        let c = at(&trace, k);
        let known: Vec<usize> = c.nodes.iter().map(|n| n.id).collect();
        assert!(
            c.arcs.iter().all(|(a, b)| known.contains(a) && known.contains(b)),
            "snapshot {k}"
        );
    }
}

#[test]
fn the_condensation_only_grows_going_forward_and_each_step_adds_at_most_one_node() {
    let trace = trace();
    let mut previous = at(&trace, 0);
    for k in 1..=trace.last() {
        let next = at(&trace, k);
        assert!(next.nodes.starts_with(&previous.nodes), "snapshot {k}");
        assert!(next.nodes.len() <= previous.nodes.len() + 1);
        assert!(previous.arcs.iter().all(|a| next.arcs.contains(a)), "snapshot {k}");
        previous = next;
    }
}

#[test]
fn the_condensation_changes_exactly_at_the_ten_closures() {
    let trace = trace();
    let changes: Vec<usize> = (1..=trace.last()).filter(|&k| at(&trace, k) != at(&trace, k - 1)).collect();
    assert_eq!(changes, trace.closures());
}

#[test]
fn the_columns_are_the_longest_path_to_a_sink_with_1_1_2_3_2_1_nodes() {
    let placements = at(&trace(), 252).placements().unwrap();
    let columns: Vec<usize> = (0..6).map(|c| placements.iter().filter(|p| p.column == c).count()).collect();
    assert_eq!(columns, vec![1, 1, 2, 3, 2, 1]);
    let table: Vec<(usize, usize, usize)> = placements.iter().map(|p| (p.id, p.column, p.row)).collect();
    assert_eq!(
        table,
        vec![
            (1, 0, 0),
            (2, 1, 0),
            (3, 2, 0),
            (4, 2, 1),
            (5, 3, 0),
            (6, 3, 1),
            (7, 3, 2),
            (8, 4, 0),
            (9, 4, 1),
            (10, 5, 0)
        ]
    );
}

#[test]
fn a_node_never_moves_once_it_has_been_placed() {
    let trace = trace();
    let mut previous: Vec<Placement> = Vec::new();
    for k in 0..=trace.last() {
        let now = at(&trace, k).placements().unwrap();
        assert!(now.starts_with(&previous), "snapshot {k}: an earlier node moved");
        previous = now;
    }
}

#[test]
fn going_back_and_forward_again_gives_the_same_places() {
    let trace = trace();
    for &k in &trace.closures() {
        let (before, after) = (at(&trace, k - 1).placements().unwrap(), at(&trace, k).placements().unwrap());
        assert_eq!(after.len(), before.len() + 1);
        assert_eq!(at(&trace, k - 1).placements().unwrap(), before);
        assert_eq!(at(&trace, k).placements().unwrap(), after);
        assert!(after.starts_with(&before));
    }
}

#[test]
fn the_slots_are_on_the_158_pitch_and_89_apart_in_a_column() {
    for (column, slots) in SLOTS.iter().enumerate() {
        assert!(slots.iter().all(|&(x, _)| x == 100.0 + 158.0 * column as f64));
        assert!(slots.windows(2).all(|w| w[1].1 - w[0].1 >= 89.0));
    }
}

/// The distance from a segment to the rectangle round `centre` that is `BOX` and a margin big, which is 0 if they
/// cross.
fn distance_to_box(from: (f64, f64), to: (f64, f64), centre: (f64, f64)) -> f64 {
    let (half_width, half_height) = (BOX.0 / 2.0 + MARGIN, BOX.1 / 2.0 + MARGIN);
    (0..=4000)
        .map(|i| {
            let t = f64::from(i) / 4000.0;
            let (x, y) = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            let (dx, dy) = ((x - centre.0).abs() - half_width, (y - centre.1).abs() - half_height);
            dx.max(0.0).hypot(dy.max(0.0))
        })
        .fold(f64::MAX, f64::min)
}

#[test]
fn no_straight_arrow_goes_through_another_node_at_any_of_the_ten_stages_with_about_ten_units_to_spare() {
    let trace = trace();
    let mut smallest = f64::MAX;
    for &k in &trace.closures() {
        let c = at(&trace, k);
        let placed = c.placements().unwrap();
        let centre = |id: usize| placed.iter().find(|p| p.id == id).unwrap().centre;
        for &(from, to) in &c.arcs {
            for node in &c.nodes {
                if node.id != from && node.id != to {
                    let d = distance_to_box(centre(from), centre(to), centre(node.id));
                    assert!(d > 0.0, "stage {k}: the arrow C{from} to C{to} goes through C{}", node.id);
                    smallest = smallest.min(d);
                }
            }
        }
    }
    assert!((smallest - 9.84).abs() < 0.05, "{smallest}");
}

#[test]
fn every_arrow_points_from_right_to_left() {
    let placed = at(&trace(), 252).placements().unwrap();
    let x = |id: usize| placed.iter().find(|p| p.id == id).unwrap().centre.0;
    assert!(at(&trace(), 252).arcs.iter().all(|&(from, to)| x(from) > x(to)));
}

#[test]
fn an_arrow_to_a_component_that_is_not_emitted_is_a_typed_error() {
    let c = Condensation {
        nodes: vec![Node { id: 1, members: vec![0] }],
        arcs: vec![(1, 2)],
    };
    assert_eq!(c.placements(), Err(PlaceError::ForwardArc { from: 1, to: 2 }));
}

#[test]
fn a_node_that_the_table_has_no_place_for_is_a_typed_error() {
    let nodes: Vec<Node> = (1..=3).map(|id| Node { id, members: vec![id] }).collect();
    let c = Condensation { nodes, arcs: vec![] };
    assert_eq!(c.placements(), Err(PlaceError::NoSlot { column: 0, row: 1 }));
}

#[test]
fn a_cycle_is_found() {
    let nodes: Vec<Node> = (1..=2).map(|id| Node { id, members: vec![id] }).collect();
    let c = Condensation {
        nodes,
        arcs: vec![(1, 2), (2, 1)],
    };
    assert!(c.has_cycle());
}
