//! The five pinned traces, every error, the invariants of every snapshot, and the two oracles.

use super::{oracle::*, *};
use crate::edmonds_karp::fixtures::{CANONICAL, Fixture, NAMES, REVERSE};
use std::collections::BTreeMap;

fn trace_of(fixture: &Fixture) -> Trace {
    Trace::build(&fixture.graph()).expect("a trace")
}

/// One event as the specification writes it, with vertices by `names`.
fn describe(event: &Event, names: &[&str]) -> String {
    match event {
        Event::VertexDequeued { vertex } => format!("Dequeued({})", names[*vertex]),
        Event::ArcExamined { hop, already_discovered } => format!(
            "Examined(e{} {}{})",
            hop.edge,
            hop.direction.letter(),
            if *already_discovered { ", seen" } else { "" }
        ),
        Event::VertexDiscovered { vertex, .. } => format!("Discovered({})", names[*vertex]),
        Event::BottleneckDetermined { .. } => "Bottleneck".to_owned(),
        Event::FlowAugmented => "Augmented".to_owned(),
        other => other.name().to_owned(),
    }
}

fn sequence(trace: &Trace, names: &[&str]) -> Vec<String> {
    trace.snapshots().iter().map(|s| describe(&s.event, names)).collect()
}

/// How many snapshots there are for each kind of event.
fn histogram(trace: &Trace) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for snapshot in trace.snapshots() {
        *counts.entry(snapshot.event.name()).or_insert(0) += 1;
    }
    counts
}

fn expect_histogram(trace: &Trace, expected: &[(&'static str, usize)]) {
    let wanted: BTreeMap<_, _> = expected.iter().copied().collect();
    assert_eq!(histogram(trace), wanted);
}

/// A path as the names of the vertices on it.
fn path_names(hops: &[Hop], names: &[&str]) -> String {
    let mut text = names[hops[0].from].to_owned();
    for hop in hops {
        text.push('-');
        text.push_str(names[hop.to]);
    }
    text
}

fn graph(vertices: &[u32], source: u32, sink: u32, edges: &[(u32, u32, u32)]) -> Graph {
    Graph {
        vertices: vertices.to_vec(),
        source,
        sink,
        edges: edges
            .iter()
            .enumerate()
            .map(|(id, &(from, to, capacity))| EdgeSpec {
                id: id as u32,
                from,
                to,
                capacity,
            })
            .collect(),
    }
}

// The canonical network.

#[test]
fn the_canonical_network_finds_the_three_paths_the_flows_and_the_cut() {
    let trace = trace_of(&CANONICAL);
    let augmentations: Vec<&Augmentation> = trace.snapshots().iter().filter_map(|s| s.augmentation.as_ref()).collect();
    let paths: Vec<String> = augmentations.iter().map(|a| path_names(&a.path, &NAMES)).collect();
    assert_eq!(paths, ["s-v1-v3-t", "s-v2-v4-t", "s-v2-v4-v3-t"]);
    assert_eq!(augmentations.iter().map(|a| a.delta).collect::<Vec<_>>(), [12, 4, 7]);
    assert_eq!(augmentations.iter().map(|a| a.total_after).collect::<Vec<_>>(), [12, 16, 23]);
    let last = trace.snapshots().last().expect("a last snapshot");
    assert_eq!(last.flows, [12, 11, 0, 0, 12, 0, 11, 7, 19, 4]);
    let cut = last.final_cut.as_ref().expect("a cut");
    assert_eq!(cut.reachable, [0, 1, 2, 4], "S is s, v1, v2 and v4");
    assert_eq!(cut.crossing, [4, 7, 9], "e4, e7 and e9 go from S to T");
    assert_eq!(cut.capacity, 23);
    assert_eq!(trace.max_flow(), 23);
}

#[test]
fn the_canonical_network_has_87_snapshots_and_the_pinned_histogram() {
    let trace = trace_of(&CANONICAL);
    assert_eq!((trace.len(), trace.last()), (87, 86));
    expect_histogram(
        &trace,
        &[
            ("Initial", 1),
            ("SearchStarted", 4),
            ("VertexDequeued", 17),
            ("ArcExamined", 36),
            ("VertexDiscovered", 18),
            ("PathFound", 3),
            ("BottleneckDetermined", 3),
            ("FlowAugmented", 3),
            ("SearchExhausted", 1),
            ("MinCutShown", 1),
        ],
    );
}

#[test]
fn the_edge_that_goes_from_t_to_s_is_not_in_the_cut() {
    let trace = trace_of(&CANONICAL);
    let cut = trace.snapshots().last().and_then(|s| s.final_cut.clone()).expect("a cut");
    // e5 is v3 to v2, which is T to S. It has capacity 9, so counting it would give a cut of 32.
    assert!(!cut.crossing.contains(&5));
}

// The network that needs a reverse transition.

#[test]
fn the_reverse_network_takes_back_flow_from_e0_on_its_second_path() {
    let trace = trace_of(&REVERSE);
    let augmentations: Vec<&Augmentation> = trace.snapshots().iter().filter_map(|s| s.augmentation.as_ref()).collect();
    assert_eq!(augmentations.len(), 2);
    assert_eq!(path_names(&augmentations[0].path, &NAMES), "s-v1-v3-t");
    assert_eq!(path_names(&augmentations[1].path, &NAMES), "s-v4-v3-v1-v2-t");
    assert_eq!((augmentations[0].delta, augmentations[1].delta), (2, 1));
    assert_eq!((augmentations[0].total_after, augmentations[1].total_after), (2, 3));
    let steps = |a: &Augmentation| a.path.iter().map(|h| (h.edge, h.direction)).collect::<Vec<_>>();
    use Direction::{Forward as F, Reverse as R};
    assert_eq!(steps(augmentations[0]), [(4, F), (0, F), (1, F)]);
    assert_eq!(steps(augmentations[1]), [(5, F), (2, F), (0, R), (3, F), (6, F)]);
    assert_eq!(
        (augmentations[1].flows_before[0], augmentations[1].flows_after[0]),
        (2, 1),
        "e0 goes from 2 to 1"
    );
    assert_eq!((augmentations[1].total_before, augmentations[1].total_after), (2, 3));
}

#[test]
fn the_reverse_network_ends_with_the_pinned_flows_and_cut() {
    let trace = trace_of(&REVERSE);
    let last = trace.snapshots().last().expect("a last snapshot");
    assert_eq!(last.flows, [1, 2, 1, 1, 2, 1, 1]);
    let cut = last.final_cut.as_ref().expect("a cut");
    assert_eq!(
        (cut.reachable.clone(), cut.crossing.clone(), cut.capacity),
        (vec![0], vec![4, 5], 3)
    );
    assert_eq!((trace.len(), trace.last()), (44, 43));
    expect_histogram(
        &trace,
        &[
            ("Initial", 1),
            ("SearchStarted", 3),
            ("VertexDequeued", 10),
            ("ArcExamined", 12),
            ("VertexDiscovered", 10),
            ("PathFound", 2),
            ("BottleneckDetermined", 2),
            ("FlowAugmented", 2),
            ("SearchExhausted", 1),
            ("MinCutShown", 1),
        ],
    );
}

#[test]
fn a_search_that_does_not_use_reverse_transitions_finds_less_in_the_reverse_network() {
    // The path that cancels is the whole point of the network. Without it the flow cannot reach 3.
    let without = Trace::build_without_reverse(&REVERSE.graph()).expect("a trace");
    assert_eq!(without.max_flow(), 2);
}

// The three small networks that only the tests use.

#[test]
fn an_unreachable_sink_gives_the_pinned_twelve_events() {
    let names = ["s", "a", "b", "t"];
    let trace =
        Trace::build(&graph(&[0, 1, 2, 3], 0, 3, &[(0, 1, 2), (1, 2, 1), (2, 1, 1), (2, 3, 0)])).expect("a trace");
    assert_eq!(
        sequence(&trace, &names),
        [
            "Initial",
            "SearchStarted",
            "Dequeued(s)",
            "Examined(e0 F)",
            "Discovered(a)",
            "Dequeued(a)",
            "Examined(e1 F)",
            "Discovered(b)",
            "Dequeued(b)",
            "Examined(e2 F, seen)",
            "SearchExhausted",
            "MinCutShown"
        ]
    );
    let last = trace.snapshots().last().expect("a last snapshot");
    assert_eq!((trace.max_flow(), last.flows.clone()), (0, vec![0, 0, 0, 0]));
    let cut = last.final_cut.as_ref().expect("a cut");
    assert_eq!(
        (cut.reachable.clone(), cut.crossing.clone(), cut.capacity),
        (vec![0, 1, 2], vec![3], 0)
    );
    assert!(
        !sequence(&trace, &names).iter().any(|e| e.contains("e3")),
        "the edge of no capacity is never examined"
    );
}

#[test]
fn an_edge_of_no_capacity_alone_gives_five_snapshots() {
    let trace = Trace::build(&graph(&[0, 1], 0, 1, &[(0, 1, 0)])).expect("a trace");
    assert_eq!(
        sequence(&trace, &["s", "t"]),
        ["Initial", "SearchStarted", "Dequeued(s)", "SearchExhausted", "MinCutShown"]
    );
    let last = trace.snapshots().last().expect("a last snapshot");
    assert_eq!(
        (trace.max_flow(), last.final_cut.as_ref().map(|c| c.reachable.clone())),
        (0, Some(vec![0]))
    );
}

#[test]
fn parallel_edges_keep_their_ids_and_are_tried_in_order() {
    let names = ["s", "a", "t"];
    let trace = Trace::build(&graph(&[0, 1, 2], 0, 2, &[(0, 1, 1), (0, 1, 2), (1, 2, 3)])).expect("a trace");
    assert_eq!(
        sequence(&trace, &names),
        [
            "Initial",
            "SearchStarted",
            "Dequeued(s)",
            "Examined(e0 F)",
            "Discovered(a)",
            "Examined(e1 F, seen)",
            "Dequeued(a)",
            "Examined(e2 F)",
            "Discovered(t)",
            "PathFound",
            "Bottleneck",
            "Augmented",
            "SearchStarted",
            "Dequeued(s)",
            "Examined(e1 F)",
            "Discovered(a)",
            "Dequeued(a)",
            "Examined(e0 R, seen)",
            "Examined(e2 F)",
            "Discovered(t)",
            "PathFound",
            "Bottleneck",
            "Augmented",
            "SearchStarted",
            "Dequeued(s)",
            "SearchExhausted",
            "MinCutShown"
        ]
    );
    assert_eq!(trace.len(), 27);
    let last = trace.snapshots().last().expect("a last snapshot");
    assert_eq!(last.flows, [1, 2, 3]);
    let cut = last.final_cut.as_ref().expect("a cut");
    assert_eq!(
        (trace.max_flow(), cut.reachable.clone(), cut.crossing.clone(), cut.capacity),
        (3, vec![0], vec![0, 1], 3)
    );
    let deltas: Vec<u64> = trace
        .snapshots()
        .iter()
        .filter_map(|s| s.augmentation.as_ref())
        .map(|a| a.delta)
        .collect();
    assert_eq!(deltas, [1, 2]);
}

// What is refused, and in which order.

#[test]
fn every_kind_of_error_is_raised_by_an_input_that_has_it() {
    let ok = [(0, 1, 1)];
    let cases: Vec<(Graph, SolverError)> = vec![
        (graph(&[], 0, 1, &[]), SolverError::TooFewVertices { count: 0, min: 2 }),
        (graph(&[0], 0, 1, &[]), SolverError::TooFewVertices { count: 1, min: 2 }),
        (
            graph(&(0..13).collect::<Vec<_>>(), 0, 1, &[]),
            SolverError::TooManyVertices { count: 13, max: 12 },
        ),
        (graph(&[0, 1, 1], 0, 1, &ok), SolverError::DuplicateVertexId { vertex_id: 1 }),
        (graph(&[0, 1], 7, 1, &ok), SolverError::UnknownSource { source_id: 7 }),
        (graph(&[0, 1], 0, 8, &ok), SolverError::UnknownSink { sink_id: 8 }),
        (graph(&[0, 1], 1, 1, &ok), SolverError::SameSourceAndSink { vertex_id: 1 }),
        (
            graph(&[0, 1], 0, 1, &vec![(0, 1, 1); 49]),
            SolverError::TooManyEdges { count: 49, max: 48 },
        ),
        (
            {
                let mut g = graph(&[0, 1], 0, 1, &[(0, 1, 1), (0, 1, 1)]);
                g.edges[1].id = 0;
                g
            },
            SolverError::DuplicateEdgeId { edge_id: 0 },
        ),
        (
            graph(&[0, 1], 0, 1, &[(0, 9, 1)]),
            SolverError::UnknownEndpoint {
                edge_id: 0,
                vertex_id: 9,
                end: End::To,
            },
        ),
        (
            graph(&[0, 1], 0, 1, &[(9, 1, 1)]),
            SolverError::UnknownEndpoint {
                edge_id: 0,
                vertex_id: 9,
                end: End::From,
            },
        ),
        (
            graph(&[0, 1], 0, 1, &[(1, 1, 1)]),
            SolverError::SelfLoop { edge_id: 0, vertex_id: 1 },
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(Trace::build(&input).err(), Some(expected));
    }
    let small = graph(&[0, 1, 2], 0, 2, &[(0, 1, 1), (1, 2, 1)]);
    assert_eq!(
        Trace::build_within(&small, 2).err(),
        Some(SolverError::EventBudgetExceeded { limit: 2, attempted: 3 })
    );
}

#[test]
fn the_errors_come_in_the_pinned_order() {
    // An empty list of vertices is too few vertices, and not an unknown source.
    assert!(matches!(
        Trace::build(&graph(&[], 5, 6, &[])),
        Err(SolverError::TooFewVertices { .. })
    ));
    // A bad source comes before a bad edge, and the edges' count before their ends.
    assert!(matches!(
        Trace::build(&graph(&[0, 1], 7, 1, &[(9, 9, 1)])),
        Err(SolverError::UnknownSource { .. })
    ));
    assert!(matches!(
        Trace::build(&graph(&[0, 1], 0, 1, &vec![(9, 9, 1); 49])),
        Err(SolverError::TooManyEdges { .. })
    ));
    // A loop on a vertex that is not there is an unknown endpoint, and never a loop.
    assert_eq!(
        Trace::build(&graph(&[0, 1], 0, 1, &[(9, 9, 1)])).err(),
        Some(SolverError::UnknownEndpoint {
            edge_id: 0,
            vertex_id: 9,
            end: End::From
        })
    );
    // The ends of every edge are checked before any loop, so a loop on edge 0 does not hide an unknown end on edge 1.
    assert_eq!(
        Trace::build(&graph(&[0, 1], 0, 1, &[(1, 1, 1), (0, 8, 1)])).err(),
        Some(SolverError::UnknownEndpoint {
            edge_id: 1,
            vertex_id: 8,
            end: End::To
        })
    );
    // From is checked before To.
    assert_eq!(
        Trace::build(&graph(&[0, 1], 0, 1, &[(8, 9, 1)])).err(),
        Some(SolverError::UnknownEndpoint {
            edge_id: 0,
            vertex_id: 8,
            end: End::From
        })
    );
}

#[test]
fn two_vertices_and_no_edges_are_valid() {
    let trace = Trace::build(&graph(&[3, 4], 3, 4, &[])).expect("a trace");
    assert_eq!((trace.len(), trace.max_flow()), (5, 0));
}

#[test]
fn every_error_says_what_it_is_about() {
    for error in [
        SolverError::TooFewVertices { count: 1, min: 2 },
        SolverError::UnknownEndpoint {
            edge_id: 3,
            vertex_id: 9,
            end: End::To,
        },
        SolverError::EventBudgetExceeded { limit: 5, attempted: 6 },
    ] {
        assert!(!error.to_string().is_empty());
    }
}

// The budget.

#[test]
fn the_budget_is_counted_in_snapshots_and_a_run_that_does_not_fit_is_refused_whole() {
    for (fixture, count) in [(&REVERSE, 44), (&CANONICAL, 87)] {
        let graph = fixture.graph();
        assert!(Trace::build_within(&graph, count).is_ok(), "{} fits in {count}", fixture.key);
        assert_eq!(
            Trace::build_within(&graph, count - 1).err(),
            Some(SolverError::EventBudgetExceeded {
                limit: count - 1,
                attempted: count
            }),
            "{} does not fit in {}",
            fixture.key,
            count - 1
        );
    }
    assert_eq!(SNAPSHOT_BUDGET, 20_000);
    assert!(Trace::build_within(&CANONICAL.graph(), 0).is_err());
}

// What is true of every snapshot.

/// Checks what must hold of every snapshot of `trace`.
fn check_invariants(trace: &Trace, label: &str) {
    let (source, sink) = (trace.source(), trace.sink());
    let mut total_before = 0;
    let mut last_flows: Option<Vec<u64>> = None;
    for (at, snapshot) in trace.snapshots().iter().enumerate() {
        assert_eq!(snapshot.index, at, "{label}: the indices are contiguous");
        let mut net = vec![0i128; trace.vertex_count()];
        for (edge, &flow) in trace.edges().iter().zip(&snapshot.flows) {
            assert!(
                flow <= u64::from(edge.capacity),
                "{label} at {at}: a flow is within its capacity"
            );
            net[edge.from] += i128::from(flow);
            net[edge.to] -= i128::from(flow);
        }
        for (vertex, &n) in net.iter().enumerate() {
            if vertex != source && vertex != sink {
                assert_eq!(n, 0, "{label} at {at}: flow is conserved at vertex {vertex}");
            }
        }
        assert_eq!(
            net[source],
            i128::from(snapshot.total_flow),
            "{label} at {at}: the source sends the total"
        );
        assert_eq!(
            -net[sink],
            i128::from(snapshot.total_flow),
            "{label} at {at}: the sink takes the total"
        );
        assert!(snapshot.total_flow >= total_before, "{label} at {at}: the total never falls");
        total_before = snapshot.total_flow;
        // Flows change only when the whole path is augmented, and so no snapshot shows a path half done.
        if let Some(before) = &last_flows {
            if snapshot.event != Event::FlowAugmented {
                assert_eq!(&snapshot.flows, before, "{label} at {at}: flows change only at an augmentation");
            }
        }
        last_flows = Some(snapshot.flows.clone());
        assert_eq!(
            snapshot.augmentation.is_some(),
            snapshot.event == Event::FlowAugmented,
            "{label} at {at}"
        );
        assert_eq!(
            snapshot.final_cut.is_some(),
            snapshot.event == Event::MinCutShown,
            "{label} at {at}"
        );
        // An examined arc always has room, and a zero-residual transition is never examined.
        if let Event::ArcExamined { hop, .. } = snapshot.event {
            let flow = snapshot.flows[hop.edge];
            let capacity = u64::from(trace.edges()[hop.edge].capacity);
            let room = if hop.direction == Direction::Forward { capacity - flow } else { flow };
            assert!(room > 0, "{label} at {at}: an examined arc has room");
        }
    }
    let last = trace.snapshots().last().expect("a last snapshot");
    let cut = last.final_cut.as_ref().expect("a cut");
    assert_eq!(cut.capacity, last.total_flow, "{label}: the cut equals the flow");
    let discovered: Vec<usize> = {
        let mut found = last.bfs.discovered.clone();
        found.sort_unstable();
        found
    };
    assert_eq!(discovered, cut.reachable, "{label}: S is what the last search reached");
}

#[test]
fn every_snapshot_of_every_pinned_network_keeps_the_invariants() {
    check_invariants(&trace_of(&CANONICAL), "canonical");
    check_invariants(&trace_of(&REVERSE), "reverse");
    check_invariants(
        &Trace::build(&graph(&[0, 1, 2], 0, 2, &[(0, 1, 1), (0, 1, 2), (1, 2, 3)])).expect("a trace"),
        "parallel",
    );
    check_invariants(
        &Trace::build(&graph(&[0, 1, 2, 3], 0, 3, &[(0, 1, 2), (1, 2, 1), (2, 1, 1), (2, 3, 0)])).expect("a trace"),
        "unreachable",
    );
}

#[test]
fn a_snapshot_holds_a_whole_augmentation_and_nothing_shows_it_half_done() {
    let trace = trace_of(&REVERSE);
    let at = trace.augmentation_indices()[1];
    let snapshot = trace.snapshot(at).expect("a snapshot");
    let augmentation = snapshot.augmentation.as_ref().expect("an augmentation");
    assert_eq!(augmentation.flows_after, snapshot.flows);
    assert_eq!(trace.snapshot(at - 1).expect("the one before").flows, augmentation.flows_before);
    assert_eq!(snapshot.path, augmentation.path);
}

#[test]
fn the_search_looks_at_the_transitions_of_each_vertex_by_edge_id() {
    // Both v1 to v3 (e0) and v1 to v2 (e3) lead on from v1. The tie is settled by the ID, so e0 is looked at first.
    let trace = trace_of(&REVERSE);
    let first_from_v1 = trace
        .snapshots()
        .iter()
        .find(|s| matches!(s.event, Event::ArcExamined { hop, .. } if hop.from == 1))
        .expect("a transition of v1");
    assert!(matches!(first_from_v1.event, Event::ArcExamined { hop, .. } if hop.edge == 0));
}

#[test]
fn the_last_search_is_not_cut_short_and_the_sink_ends_the_others() {
    let trace = trace_of(&CANONICAL);
    for snapshot in trace.snapshots() {
        if let Event::VertexDiscovered { vertex, .. } = snapshot.event {
            if vertex == trace.sink() {
                let next = trace.snapshot(snapshot.index + 1).expect("one more");
                assert_eq!(next.event, Event::PathFound, "a search stops when the sink is found");
            }
        }
    }
    let last = trace.snapshots().last().expect("a last snapshot");
    assert!(!last.bfs.discovered.contains(&trace.sink()));
}

// Where the augmentations are.

#[test]
fn the_next_and_previous_augmentation_are_strictly_after_and_before() {
    let trace = trace_of(&CANONICAL);
    let at = trace.augmentation_indices().to_vec();
    assert_eq!(at.len(), 3);
    assert_eq!(trace.next_augmentation(0), Some(at[0]));
    assert_eq!(
        trace.next_augmentation(at[0]),
        Some(at[1]),
        "from an augmentation, the next and not itself"
    );
    assert_eq!(trace.next_augmentation(at[2]), None);
    assert_eq!(trace.previous_augmentation(at[1]), Some(at[0]));
    assert_eq!(trace.previous_augmentation(at[0]), None);
    assert_eq!(trace.previous_augmentation(trace.last()), Some(at[2]));
    assert_eq!(trace.previous_augmentation(0), None);
}

// The oracles.

/// Runs the solver on `raw`, and says its maximum flow and whether an augmenting path used a reverse transition.
fn solve(raw: &Raw) -> (u64, bool) {
    let trace = Trace::build(&graph_of(raw)).expect("a trace");
    let reversed = trace
        .snapshots()
        .iter()
        .filter_map(|s| s.augmentation.as_ref())
        .any(|a| a.path.iter().any(|h| h.direction == Direction::Reverse));
    (trace.max_flow(), reversed)
}

#[test]
fn the_solver_agrees_with_the_minimum_cut_on_500_broad_graphs() {
    let graphs = broad_graphs();
    assert_eq!(graphs.len(), 500);
    for (at, raw) in graphs.iter().enumerate() {
        assert_eq!(solve(raw).0, minimum_cut(raw), "broad graph {at}: {raw:?}");
    }
    assert!(
        graphs.iter().all(|(_, edges)| edges.iter().all(|&(from, to, _)| from != to)),
        "no loops"
    );
}

#[test]
fn the_layered_generator_makes_the_pinned_graphs() {
    let graphs = layered_graphs();
    assert_eq!(graphs.len(), 500);
    // The generator is checked before anything is compared with it: the first graph, and what all 500 add up to.
    assert_eq!(
        graphs[0],
        (
            8,
            vec![
                (0, 3, 3),
                (3, 6, 1),
                (0, 2, 2),
                (1, 5, 1),
                (1, 2, 3),
                (6, 7, 1),
                (4, 7, 2),
                (1, 5, 3),
                (5, 7, 3),
                (2, 6, 1),
                (2, 1, 2),
                (0, 2, 3),
                (0, 1, 1)
            ]
        )
    );
    assert_eq!(graphs.iter().map(|(_, e)| e.len()).sum::<usize>(), 6_067);
    assert_eq!(
        graphs
            .iter()
            .flat_map(|(_, e)| e.iter())
            .map(|&(_, _, c)| u64::from(c))
            .sum::<u64>(),
        12_188
    );
    assert!(
        graphs
            .iter()
            .all(|(_, e)| e.len() <= 16 && e.iter().all(|&(a, b, c)| a != b && (1..=3).contains(&c)))
    );
    assert!(graphs.iter().all(|(n, _)| (6..=8).contains(n)));
}

#[test]
fn the_solver_agrees_with_the_minimum_cut_on_all_500_layered_graphs() {
    let graphs = layered_graphs();
    let (mut nonzero, mut reversed) = (0, 0);
    for (at, raw) in graphs.iter().enumerate() {
        let (flow, used_reverse) = solve(raw);
        assert_eq!(flow, minimum_cut(raw), "layered graph {at}: {raw:?}");
        nonzero += usize::from(flow > 0);
        reversed += usize::from(used_reverse);
    }
    assert_eq!(nonzero, 500, "every layered graph has a path from the source to the sink");
    assert!(reversed >= 10, "{reversed} graphs need a reverse transition");
    assert_eq!(reversed, 24, "and this generator gives 24");
}

#[test]
fn a_search_without_reverse_transitions_is_caught_by_the_layered_graphs() {
    let graphs = layered_graphs();
    let mismatches: Vec<usize> = graphs
        .iter()
        .enumerate()
        .filter(|(_, raw)| {
            let trace = Trace::build_without_reverse(&graph_of(raw)).expect("a trace");
            trace.max_flow() != minimum_cut(raw)
        })
        .map(|(at, _)| at)
        .collect();
    assert!(
        mismatches.len() >= 10,
        "the negative control is caught on {} graphs: {mismatches:?}",
        mismatches.len()
    );
    assert_eq!(mismatches.len(), 24);
}

#[test]
fn the_broad_graphs_cannot_tell_a_search_without_reverse_transitions_from_a_right_one() {
    // The reason for the second suite: none of these needs flow to be taken back, so this suite alone would pass a
    // solver that has no reverse transitions at all.
    let graphs = broad_graphs();
    let caught = graphs
        .iter()
        .filter(|raw| {
            let trace = Trace::build_without_reverse(&graph_of(raw)).expect("a trace");
            trace.max_flow() != minimum_cut(raw)
        })
        .count();
    assert_eq!(caught, 0);
}

#[test]
fn with_reverse_transitions_allowed_the_test_build_matches_the_shipped_search_exactly() {
    // This checks the plumbing of the switch. If it did anything with reverse transitions allowed, a difference with
    // them off would not mean that they were missing.
    for raw in &layered_graphs() {
        let graph = graph_of(raw);
        let shipped = Trace::build(&graph).expect("a trace");
        let allowed = Trace::build_with(&graph, SNAPSHOT_BUDGET, Options::default()).expect("a trace");
        assert_eq!(shipped, allowed);
        assert_eq!(shipped.max_flow(), allowed.max_flow());
    }
}

#[test]
fn the_flows_of_every_layered_graph_keep_the_invariants() {
    for (at, raw) in layered_graphs().iter().enumerate().step_by(7) {
        check_invariants(&Trace::build(&graph_of(raw)).expect("a trace"), &format!("layered {at}"));
    }
    for (at, raw) in broad_graphs().iter().enumerate().step_by(11) {
        check_invariants(&Trace::build(&graph_of(raw)).expect("a trace"), &format!("broad {at}"));
    }
}

#[test]
fn the_cost_of_the_most_that_the_solver_is_asked_for_is_far_under_the_budget() {
    // 12 vertices and 48 edges of large capacity, drawn at random. The bound is 35,000 snapshots in theory, and what is
    // seen is about a thousand, so the budget is a limit that real input does not reach.
    let mut rng = SplitMix64::new(77);
    let mut worst = 0;
    for _ in 0..40 {
        let mut edges = Vec::new();
        while edges.len() < 48 {
            let (a, b) = (rng.next() % 12, rng.next() % 12);
            if a != b {
                edges.push((a as u32, b as u32, 1 + (rng.next() % 1_000_000) as u32));
            }
        }
        let trace = Trace::build(&graph(&(0..12).collect::<Vec<_>>(), 0, 11, &edges)).expect("a trace");
        worst = worst.max(trace.len());
    }
    assert!(worst < SNAPSHOT_BUDGET, "{worst}");
}
