use super::{Error, Graph, QueueEntry, RawEdge, Relaxation, Snapshot, steps};

const NAMES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];
const A: usize = 0;
const B: usize = 1;
const C: usize = 2;
const D: usize = 3;
const E: usize = 4;
const F: usize = 5;

/// The reference graph: its edges, in the order the demo draws them.
const REFERENCE: [RawEdge; 9] = [
    (A, B, 4),
    (A, C, 2),
    (C, B, 1),
    (B, D, 5),
    (C, D, 8),
    (C, E, 10),
    (D, E, 2),
    (D, F, 6),
    (E, F, 3),
];

fn graph(count: usize, edges: &[RawEdge]) -> Graph {
    Graph::new(&NAMES[..count], edges).expect("a valid graph")
}

fn last(steps: &[Snapshot]) -> &Snapshot {
    steps.last().expect("at least one step")
}

/// The first step that examined the edge from `from` to `to`.
fn examining<'a>(graph: &Graph, steps: &'a [Snapshot], from: usize, to: usize) -> &'a Snapshot {
    steps
        .iter()
        .find(|step| {
            step.examined
                .is_some_and(|(edge, _)| graph.edges()[edge].from == from && graph.edges()[edge].to == to)
        })
        .expect("the edge to be examined")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_reference_graph_gives_the_known_distances_and_predecessors() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, None).expect("a run");
    let end = last(&steps);
    assert_eq!(end.distance, [Some(0), Some(3), Some(2), Some(8), Some(10), Some(13)]);
    assert_eq!(end.predecessor, [None, Some(C), Some(A), Some(B), Some(D), Some(E)]);
    assert!(end.settled.iter().all(|&settled| settled));
}

#[test]
fn the_route_to_f_runs_through_c_b_d_and_e_with_a_length_of_13() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, Some(F)).expect("a run");
    let end = last(&steps);
    let route: Vec<(usize, usize)> = end.path.iter().map(|&e| (graph.edges()[e].from, graph.edges()[e].to)).collect();
    assert_eq!(route, [(A, C), (C, B), (B, D), (D, E), (E, F)]);
    assert!(end.note.contains("A to C to B to D to E to F"), "{}", end.note);
    assert!(end.note.contains("length of 13"), "{}", end.note);
}

#[test]
fn the_state_just_after_b_is_settled_and_b_to_d_is_relaxed_is_the_documented_one() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, None).expect("a run");
    let step = examining(&graph, &steps, B, D);
    assert_eq!(step.distance, [Some(0), Some(3), Some(2), Some(8), Some(12), None]);
    assert_eq!(step.predecessor, [None, Some(C), Some(A), Some(B), Some(C), None]);
    assert_eq!(step.settled, [true, true, true, false, false, false]);
    assert_eq!(step.examined.map(|(_, relaxation)| relaxation), Some(Relaxation::Improved));
}

#[test]
fn the_first_route_found_to_b_is_not_the_final_one() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, None).expect("a run");
    let first = examining(&graph, &steps, A, B);
    assert_eq!(first.distance[B], Some(4));
    assert_eq!(last(&steps).distance[B], Some(3));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_single_vertex_is_at_distance_zero_with_no_predecessor() {
    let graph = graph(1, &[]);
    let steps = steps(&graph, A, None).expect("a run");
    let end = last(&steps);
    assert_eq!(end.distance, [Some(0)]);
    assert_eq!(end.predecessor, [None]);
    assert_eq!(end.settled, [true]);
}

#[test]
fn a_vertex_that_cannot_be_reached_stays_unreached() {
    let graph = graph(3, &[(A, B, 1)]);
    let steps = steps(&graph, A, None).expect("a run");
    assert_eq!(last(&steps).distance, [Some(0), Some(1), None]);
    assert_eq!(last(&steps).predecessor[2], None);
}

#[test]
fn an_unreachable_destination_says_there_is_no_route_and_draws_no_path() {
    let graph = graph(3, &[(A, B, 1)]);
    let steps = steps(&graph, A, Some(2)).expect("a run");
    let end = last(&steps);
    assert_eq!(end.note, "No route from A to C.");
    assert!(end.path.is_empty());
}

#[test]
fn a_destination_that_is_the_source_has_a_route_of_length_zero() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, Some(A)).expect("a run");
    let end = last(&steps);
    assert!(end.path.is_empty());
    assert!(end.note.contains("length of 0"), "{}", end.note);
    assert_eq!(steps.len(), 3, "start, settle the source, end");
}

#[test]
fn a_run_with_a_destination_stops_when_it_is_settled() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, Some(C)).expect("a run");
    assert_eq!(last(&steps).distance[C], Some(2));
    assert!(!last(&steps).settled[F], "F is further away than C, and is not reached");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_zero_weight_edge_is_accepted_and_followed() {
    let graph = graph(3, &[(A, B, 0), (B, C, 0)]);
    let steps = steps(&graph, A, None).expect("a run");
    assert_eq!(last(&steps).distance, [Some(0), Some(0), Some(0)]);
}

#[test]
fn a_cycle_terminates() {
    let graph = graph(3, &[(A, B, 1), (B, C, 1), (C, A, 1)]);
    let steps = steps(&graph, A, None).expect("a run");
    assert_eq!(last(&steps).distance, [Some(0), Some(1), Some(2)]);
}

#[test]
fn of_parallel_edges_the_lighter_decides_the_route() {
    let graph = graph(2, &[(A, B, 5), (A, B, 3)]);
    let steps = steps(&graph, A, Some(B)).expect("a run");
    let end = last(&steps);
    assert_eq!(end.distance[B], Some(3));
    assert_eq!(
        end.via[B],
        Some(1),
        "the second edge, the lighter one, is the one the route uses"
    );
    assert_eq!(end.path, [1]);
}

#[test]
fn of_parallel_edges_of_equal_weight_the_first_is_kept() {
    let graph = graph(2, &[(A, B, 3), (A, B, 3)]);
    let steps = steps(&graph, A, Some(B)).expect("a run");
    assert_eq!(last(&steps).path, [0]);
}

#[test]
fn an_equal_route_found_later_does_not_replace_the_predecessor() {
    // D is 1 + 3 by way of C and 2 + 2 by way of B. C is settled first, since it is nearer, and offers D 4. B then
    // offers D 4 as well, which is no shorter, so D keeps C.
    let graph = graph(4, &[(A, B, 2), (A, C, 1), (B, D, 2), (C, D, 3)]);
    let steps = steps(&graph, A, None).expect("a run");
    assert_eq!(last(&steps).distance[D], Some(4));
    assert_eq!(last(&steps).predecessor[D], Some(C));
}

#[test]
fn equal_distances_are_taken_from_the_queue_in_vertex_order() {
    let graph = graph(4, &[(A, D, 1), (A, C, 1), (A, B, 1)]);
    let steps = steps(&graph, A, None).expect("a run");
    let settled: Vec<usize> = steps.iter().filter_map(|step| step.active).collect();
    let mut order = settled.clone();
    order.dedup();
    assert_eq!(order, [A, B, C, D]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn an_out_of_date_queue_entry_is_marked_and_then_ignored_without_changing_a_distance() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, None).expect("a run");
    // After A is processed, B is queued at 4. C then offers B a route of 3, so the entry for 4 is out of date.
    let after_c_to_b = examining(&graph, &steps, C, B);
    assert!(
        after_c_to_b.queue.contains(&QueueEntry {
            distance: 4,
            vertex: B,
            stale: true
        }),
        "{:?}",
        after_c_to_b.queue
    );
    assert!(after_c_to_b.queue.contains(&QueueEntry {
        distance: 3,
        vertex: B,
        stale: false
    }));
    let ignored: Vec<&Snapshot> = steps.iter().filter(|step| step.note.contains("out of date")).collect();
    assert!(!ignored.is_empty());
    for (before, after) in steps.iter().zip(steps.iter().skip(1)) {
        if after.note.contains("out of date") {
            assert_eq!(before.distance, after.distance);
            assert_eq!(before.predecessor, after.predecessor);
            assert_eq!(before.settled, after.settled);
        }
    }
}

#[test]
fn the_queue_is_shown_nearest_first() {
    let graph = graph(6, &REFERENCE);
    for step in steps(&graph, A, None).expect("a run") {
        let keys: Vec<(u64, usize)> = step.queue.iter().map(|entry| (entry.distance, entry.vertex)).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
    }
}

#[test]
fn a_settled_vertexs_distance_never_changes_afterwards() {
    let graph = graph(6, &REFERENCE);
    let steps = steps(&graph, A, None).expect("a run");
    for (before, after) in steps.iter().zip(steps.iter().skip(1)) {
        for vertex in 0..NAMES.len() {
            if before.settled[vertex] {
                assert_eq!(before.distance[vertex], after.distance[vertex]);
                assert!(after.settled[vertex]);
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_negative_weight_is_refused_before_the_run_starts() {
    let result = Graph::new(&NAMES[..2], &[(A, B, -1)]);
    assert_eq!(result, Err(Error::NegativeWeight { edge: 0, weight: -1 }));
}

#[test]
fn a_self_loop_is_refused_with_its_edge_and_vertex() {
    let result = Graph::new(&NAMES[..2], &[(A, B, 1), (B, B, 1)]);
    assert_eq!(result, Err(Error::SelfLoop { edge: 1, vertex: B }));
}

#[test]
fn an_edge_to_a_vertex_that_is_not_there_is_refused() {
    assert_eq!(Graph::new(&NAMES[..2], &[(A, 5, 1)]), Err(Error::UnknownVertex { vertex: 5 }));
    assert_eq!(Graph::new(&[], &[]), Err(Error::Empty));
}

#[test]
fn a_source_or_destination_that_is_not_there_is_refused() {
    let graph = graph(2, &[(A, B, 1)]);
    assert_eq!(steps(&graph, 7, None), Err(Error::UnknownVertex { vertex: 7 }));
    assert_eq!(steps(&graph, A, Some(9)), Err(Error::UnknownVertex { vertex: 9 }));
}

#[test]
fn a_distance_that_overflows_is_an_error_not_a_wrap() {
    // Each weight fits, and so does the sum of two, but the sum of three is past what a u64 holds.
    let graph = graph(4, &[(A, B, i64::MAX), (B, C, i64::MAX), (C, D, i64::MAX)]);
    assert_eq!(steps(&graph, A, None), Err(Error::DistanceOverflow { edge: 2 }));
}

#[test]
fn an_edge_back_to_a_settled_vertex_is_skipped_before_its_sum_can_overflow() {
    // C is settled at u64::MAX - 1, and C to A would add 2 to that. A is settled already, so the sum is never taken.
    let graph = graph(3, &[(A, B, i64::MAX), (B, C, i64::MAX), (C, A, 2)]);
    let steps = steps(&graph, A, None).expect("a run, not an overflow");
    let last = last(&steps);
    assert_eq!(last.distance, vec![Some(0), Some(i64::MAX as u64), Some(u64::MAX - 1)]);
    assert!(last.settled.iter().all(|&settled| settled));
}

#[test]
fn the_largest_distance_that_fits_is_not_mistaken_for_unreachable() {
    let graph = graph(3, &[(A, B, i64::MAX), (B, C, i64::MAX)]);
    let steps = steps(&graph, A, None).expect("a run");
    assert_eq!(last(&steps).distance[C], Some(u64::MAX - 1));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_same_input_gives_the_same_steps() {
    let graph = graph(6, &REFERENCE);
    assert_eq!(steps(&graph, A, None), steps(&graph, A, None));
}

#[test]
fn every_step_has_a_note_and_one_entry_for_each_vertex() {
    let graph = graph(6, &REFERENCE);
    for step in steps(&graph, A, None).expect("a run") {
        assert!(!step.note.is_empty());
        assert_eq!(step.distance.len(), 6);
        assert_eq!(step.predecessor.len(), 6);
        assert_eq!(step.settled.len(), 6);
    }
}

/// The shortest distances by the Bellman-Ford relaxation, which shares nothing with the queue: the oracle for
/// [`steps`] on graphs no one drew by hand.
fn bellman_ford(count: usize, edges: &[RawEdge], source: usize) -> Vec<Option<u64>> {
    let mut distance = vec![None; count];
    distance[source] = Some(0_u64);
    for _ in 0..count {
        for &(from, to, weight) in edges {
            if let Some(known) = distance[from] {
                let candidate = known + u64::try_from(weight).expect("a weight that is not negative");
                if distance[to].is_none_or(|current| candidate < current) {
                    distance[to] = Some(candidate);
                }
            }
        }
    }
    distance
}

#[test]
fn random_graphs_agree_with_bellman_ford() {
    // A small linear congruential generator, so that the graphs are the same on every run without a dependency.
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |bound: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % bound
    };
    for _ in 0..300 {
        let count = 2 + next(5) as usize;
        let mut edges: Vec<RawEdge> = Vec::new();
        for _ in 0..next(14) {
            let (from, to) = (next(count as u64) as usize, next(count as u64) as usize);
            if from != to {
                edges.push((from, to, next(10) as i64));
            }
        }
        let graph = graph(count, &edges);
        let steps = steps(&graph, 0, None).expect("a run");
        assert_eq!(last(&steps).distance, bellman_ford(count, &edges, 0), "{edges:?}");
    }
}
