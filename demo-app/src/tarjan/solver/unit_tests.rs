use super::*;
use crate::tarjan::fixture;
use std::collections::BTreeSet;

fn graph(n: usize, edges: &[(usize, usize)]) -> Graph {
    Graph::from_edges(n, edges).expect("a graph within the budget")
}

fn trace_of(n: usize, edges: &[(usize, usize)]) -> Trace {
    Trace::build(&graph(n, edges)).expect("a trace within the budget")
}

fn fixture_trace() -> Trace {
    Trace::build(&fixture::graph()).expect("the fixture is within the budget")
}

/// The partition that a trace ends with, as the oracle gives its own.
fn found(trace: &Trace) -> BTreeSet<Vec<usize>> {
    trace
        .snapshot(trace.last())
        .expect("a last snapshot")
        .components
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.sort_unstable();
            c
        })
        .collect()
}

fn events(trace: &Trace) -> Vec<&Event> {
    trace.snapshots().iter().filter_map(|s| s.event.as_ref()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_fixture_trace_has_the_events_and_the_snapshots_of_the_design() {
    let trace = fixture_trace();
    assert_eq!((trace.last(), trace.snapshots().len()), (252, 253));
    let all = events(&trace);
    let count = |f: fn(&Event) -> bool| all.iter().filter(|e| f(e)).count();
    assert_eq!(count(|e| matches!(e, Event::Enter { .. })), 64);
    assert_eq!(count(|e| matches!(e, Event::Inspect { .. })), 114);
    assert_eq!(count(|e| matches!(e, Event::Return { .. })), 63);
    assert_eq!(count(|e| matches!(e, Event::Closed { .. })), 10);
    assert_eq!(count(|e| matches!(e, Event::Complete)), 1);
    assert!(
        trace.snapshot(0).is_some_and(|s| s.event.is_none()),
        "snapshot 0 is before anything has happened"
    );
}

#[test]
fn the_ten_closures_are_where_the_design_says_with_the_roots_and_the_sizes_it_says() {
    let trace = fixture_trace();
    assert_eq!(trace.closures(), vec![74, 77, 81, 104, 117, 167, 198, 206, 247, 251]);
    let roots: Vec<usize> = events(&trace)
        .iter()
        .filter_map(|e| if let Event::Closed { root, .. } = e { Some(*root) } else { None })
        .collect();
    assert_eq!(roots, vec![63, 61, 57, 53, 33, 48, 39, 23, 12, 0]);
    let last = trace.snapshot(252).unwrap();
    let sizes: Vec<usize> = last.components.iter().map(Vec::len).collect();
    assert_eq!(sizes, vec![1, 3, 4, 5, 8, 6, 7, 9, 10, 11]);
    assert!(
        matches!(last.event, Some(Event::Complete)),
        "no component is emitted after Complete"
    );
}

/// How many searches were started: the snapshots at which a vertex was entered with nothing else on the call chain.
fn search_roots(trace: &Trace) -> usize {
    trace
        .snapshots()
        .iter()
        .filter(|s| matches!(s.event, Some(Event::Enter { .. })) && s.chain.len() == 1)
        .count()
}

#[test]
fn the_first_search_from_vertex_zero_reaches_all_64_vertices() {
    let trace = fixture_trace();
    assert_eq!(search_roots(&trace), 1, "one search from vertex 0 is enough");
    let entered = events(&trace).iter().filter(|e| matches!(e, Event::Enter { .. })).count();
    assert_eq!(entered, 64);
}

#[test]
fn every_snapshot_of_the_fixture_keeps_every_invariant() {
    let trace = fixture_trace();
    for (i, s) in trace.snapshots().iter().enumerate() {
        assert_eq!(s.check(), Ok(()), "snapshot {i}");
    }
}

#[test]
fn an_emitted_vertex_is_never_on_the_stack_again_and_the_stack_only_loses_whole_components() {
    let trace = fixture_trace();
    let mut ever_emitted = [false; 64];
    for (i, s) in trace.snapshots().iter().enumerate() {
        for (v, emitted) in ever_emitted.iter_mut().enumerate() {
            if *emitted {
                assert!(!s.on_stack[v], "vertex {v} came back at snapshot {i}");
            }
            if s.membership[v].is_some() {
                *emitted = true;
            }
        }
    }
    let sizes: Vec<usize> = trace
        .snapshots()
        .windows(2)
        .map(|w| w[0].scc_stack.len().saturating_sub(w[1].scc_stack.len()))
        .filter(|&drop| drop > 0)
        .collect();
    assert_eq!(
        sizes,
        vec![1, 3, 4, 5, 8, 6, 7, 9, 10, 11],
        "each drop is one whole component, in one step"
    );
}

#[test]
fn the_stack_peaks_at_27_and_the_call_chain_is_a_different_thing() {
    let trace = fixture_trace();
    let peak = trace.snapshots().iter().map(|s| s.scc_stack.len()).max().unwrap();
    assert_eq!(peak, 27);
    let chain = trace.snapshots().iter().map(|s| s.chain.len()).max().unwrap();
    assert!(chain < peak, "a vertex can be on the stack after its call has returned");
    assert!(
        trace
            .snapshots()
            .iter()
            .any(|s| s.scc_stack.iter().any(|v| !s.chain.contains(v)))
    );
}

#[test]
fn the_fixture_partition_matches_the_oracle_and_the_ten_regions() {
    let graph = fixture::graph();
    let trace = Trace::build(&graph).unwrap();
    assert_eq!(found(&trace), oracle::partition(&graph));
    let regions: BTreeSet<Vec<usize>> = (0..fixture::REGIONS).map(|g| fixture::members_of(g).collect()).collect();
    assert_eq!(found(&trace), regions);
}

#[test]
fn the_solver_is_the_same_every_time() {
    assert_eq!(fixture_trace(), fixture_trace());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_directed_graph_of_up_to_four_vertices_matches_the_oracle() {
    for n in 0..=4usize {
        let pairs: Vec<(usize, usize)> = (0..n).flat_map(|a| (0..n).map(move |b| (a, b))).collect();
        for mask in 0u32..(1 << pairs.len()) {
            let edges: Vec<(usize, usize)> = pairs
                .iter()
                .enumerate()
                .filter(|(i, _)| mask >> i & 1 == 1)
                .map(|(_, &e)| e)
                .collect();
            let g = graph(n, &edges);
            let trace = Trace::build(&g).unwrap();
            assert_eq!(found(&trace), oracle::partition(&g), "{n} vertices, edges {edges:?}");
            assert!(trace.snapshots().iter().all(|s| s.check().is_ok()), "{edges:?}");
        }
    }
}

#[test]
fn a_thousand_random_graphs_of_up_to_twelve_vertices_match_the_oracle() {
    let mut x = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..1000 {
        let n = (next() % 12) as usize + 1;
        let density = next() % 40 + 5;
        let mut edges = Vec::new();
        for a in 0..n {
            for b in 0..n {
                if next() % 100 < density && edges.len() < MAX_EDGES {
                    edges.push((a, b));
                }
            }
        }
        let g = graph(n, &edges);
        let trace = Trace::build(&g).unwrap();
        assert_eq!(found(&trace), oracle::partition(&g), "n={n} edges={edges:?}");
        assert!(trace.snapshots().iter().all(|s| s.check().is_ok()));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_tree_edge_uses_the_childs_lowlink_on_return() {
    // 0 to 1 to 2 to 0: vertex 2 sees 0 on the stack, which lowers it to 0, and 1 and then 0 take that on the way back.
    let trace = trace_of(3, &[(0, 1), (1, 2), (2, 0)]);
    let returns: Vec<(usize, usize, usize, usize)> = events(&trace)
        .iter()
        .filter_map(|e| {
            if let Event::Return { parent, child, before, after } = e {
                Some((*parent, *child, *before, *after))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(returns, vec![(1, 2, 1, 0), (0, 1, 0, 0)]);
}

#[test]
fn an_edge_to_a_vertex_on_the_stack_uses_its_index_not_its_lowlink() {
    // The five edges of the design. Looking at 3 to 1 must leave lowlink[3] at 1, the index of vertex 1, and not at 0,
    // which is vertex 1's lowlink by then.
    let trace = trace_of(4, &[(0, 1), (0, 3), (1, 2), (2, 0), (3, 1)]);
    let at = events(&trace)
        .into_iter()
        .find_map(|e| match e {
            Event::Inspect {
                v: 3,
                w: 1,
                kind,
                before,
                after,
            } => Some((*kind, *before, *after)),
            _ => None,
        })
        .expect("the edge 3 to 1 is inspected");
    assert_eq!(at, (Kind::Stack, 3, 1));
    let last = trace.snapshot(trace.last()).unwrap();
    assert_eq!(
        last.lowlink[1],
        Some(0),
        "vertex 1's own lowlink is 0, so using it would have given 0"
    );
    assert_eq!(found(&trace), BTreeSet::from([vec![0, 1, 2, 3]]));
}

#[test]
fn an_edge_to_a_closed_vertex_does_not_lower_anything() {
    // Only 1 to 0: vertex 0 closes alone first, so vertex 1 must not be lowered through it, and closes alone as well.
    let trace = trace_of(2, &[(1, 0)]);
    assert_eq!(found(&trace), BTreeSet::from([vec![0], vec![1]]));
    let inspected = events(&trace)
        .into_iter()
        .find_map(|e| {
            if let Event::Inspect { kind, before, after, .. } = e {
                Some((*kind, *before, *after))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(inspected, (Kind::Closed, 1, 1));
}

#[test]
fn a_vertex_with_a_self_loop_is_its_own_component() {
    let trace = trace_of(1, &[(0, 0)]);
    assert_eq!(found(&trace), BTreeSet::from([vec![0]]));
    assert!(
        events(&trace)
            .iter()
            .any(|e| matches!(e, Event::Inspect { kind: Kind::Stack, .. }))
    );
}

#[test]
fn an_isolated_vertex_is_its_own_component_and_so_is_each_of_two_that_do_not_touch() {
    assert_eq!(found(&trace_of(1, &[])), BTreeSet::from([vec![0]]));
    assert_eq!(found(&trace_of(2, &[])), BTreeSet::from([vec![0], vec![1]]));
}

#[test]
fn the_eight_vertex_diagnostic_graph_has_two_search_roots_and_the_components_of_the_design() {
    let edges = [(0, 1), (1, 2), (2, 0), (3, 4), (4, 3), (5, 6), (6, 5), (2, 3), (4, 5)];
    let trace = trace_of(8, &edges);
    assert_eq!(found(&trace), BTreeSet::from([vec![0, 1, 2], vec![3, 4], vec![5, 6], vec![7]]));
    let roots = search_roots(&trace);
    assert_eq!(roots, 2, "vertex 7 is not reached from vertex 0");
    let popped: Vec<usize> = trace.snapshot(trace.last()).unwrap().components.iter().map(Vec::len).collect();
    assert_eq!(popped, vec![2, 2, 3, 1]);
}

#[test]
fn a_component_is_popped_with_its_root_last() {
    let trace = trace_of(3, &[(0, 1), (1, 2), (2, 0)]);
    let members = events(&trace)
        .into_iter()
        .find_map(|e| {
            if let Event::Closed { members, root } = e {
                Some((members.clone(), *root))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(members, (vec![2, 1, 0], 0));
}

#[test]
fn attention_is_the_vertex_of_the_event_the_root_of_a_closure_and_zero_before_anything() {
    let trace = fixture_trace();
    assert_eq!(trace.snapshot(0).unwrap().attention(), Some(0));
    assert_eq!(
        trace.snapshot(34).unwrap().attention(),
        Some(3),
        "the inspected edge 3 to 33 is looked at from 3"
    );
    assert_eq!(trace.snapshot(35).unwrap().attention(), Some(33));
    assert_eq!(trace.snapshot(74).unwrap().attention(), Some(63));
    assert_eq!(trace.snapshot(252).unwrap().attention(), Some(0), "the last closure's root");
    assert_eq!(Trace::build(&graph(0, &[])).unwrap().snapshot(0).unwrap().attention(), None);
}

#[test]
fn the_closures_are_found_before_and_after_any_snapshot_with_no_wrap() {
    let trace = fixture_trace();
    assert_eq!((trace.previous_closure(0), trace.next_closure(0)), (None, Some(74)));
    assert_eq!((trace.previous_closure(77), trace.next_closure(77)), (Some(74), Some(81)));
    assert_eq!((trace.previous_closure(251), trace.next_closure(251)), (Some(247), None));
    assert_eq!(trace.next_closure(252), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_graph_over_the_budget_is_refused_with_a_typed_error() {
    assert_eq!(Graph::from_edges(65, &[]), Err(SolveError::TooManyVertices(65)));
    let many: Vec<(usize, usize)> = (0..129).map(|i| (i / 10, i % 10)).collect();
    assert_eq!(Graph::from_edges(64, &many), Err(SolveError::TooManyEdges(129)));
    assert_eq!(Graph::from_edges(3, &[(0, 3)]), Err(SolveError::UnknownVertex(3)));
    assert_eq!(Graph::from_edges(3, &[(0, 1), (0, 1)]), Err(SolveError::DuplicateEdge(0, 1)));
}

#[test]
fn a_run_that_would_pass_the_snapshot_budget_is_refused() {
    // 64 vertices on a path make 64 Enter, 63 Inspect, 63 Return and 64 Closed, which is under the budget. A graph of
    // 64 vertices with 128 edges that form long chains does not have to be: this one is built to pass it.
    let mut edges = Vec::new();
    for v in 0..63 {
        edges.push((v, v + 1));
    }
    for v in 0..64 {
        for w in [(v + 2) % 64, (v + 3) % 64] {
            if edges.len() < MAX_EDGES && !edges.contains(&(v, w)) {
                edges.push((v, w));
            }
        }
    }
    let g = graph(64, &edges);
    match Trace::build(&g) {
        Ok(trace) => assert!(trace.snapshots().len() <= MAX_SNAPSHOTS),
        Err(e) => assert_eq!(e, SolveError::TooManySnapshots),
    }
}

#[test]
fn a_neighbour_list_is_sorted_by_target_whatever_order_the_edges_were_given_in() {
    let g = graph(4, &[(0, 3), (0, 1), (0, 2)]);
    assert_eq!(g.targets(0), &[1, 2, 3]);
    assert_eq!(g.edges(), vec![(0, 1), (0, 2), (0, 3)]);
}
