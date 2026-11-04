use super::*;
use crate::petri_net::net::{PlaceId, TransitionId as T};

fn net() -> Net {
    Net::bounded_buffer()
}

fn fill(trace: &mut Trace, net: &Net, producer: usize) {
    for t in [T::start(producer), T::finish(producer), T::put(producer)] {
        trace.push(net, t).unwrap();
    }
}

#[test]
fn a_new_trace_has_one_snapshot_with_no_event() {
    let trace = Trace::new(Snapshot::initial());
    assert_eq!((trace.len(), trace.last()), (1, 0));
    assert_eq!(trace.snapshot(0).unwrap().last, None);
    assert_eq!(trace.snapshot(1), None);
    assert!(trace.before(0).is_none());
}

#[test]
fn an_event_adds_one_snapshot_that_names_it_and_keeps_the_counters() {
    let (net, mut trace) = (net(), Trace::new(Snapshot::initial()));
    fill(&mut trace, &net, 2);
    trace.push(&net, T::take(1)).unwrap();
    assert_eq!(trace.len(), 5);
    let last = trace.snapshot(4).unwrap();
    assert_eq!(last.last, Some(Event { transition: T::take(1) }));
    assert_eq!(
        last.totals,
        Totals {
            puts: 1,
            takes: 1,
            completes: 0
        }
    );
    assert_eq!(last.marking.get(PlaceId::consumer_processing(1)), 1);
}

#[test]
fn a_failed_event_adds_nothing() {
    let (net, mut trace) = (net(), Trace::new(Snapshot::initial()));
    let before = trace.clone();
    assert!(matches!(
        trace.push(&net, T::put(0)),
        Err(TraceError::Fire(FireError::Disabled(_)))
    ));
    assert_eq!(trace, before);
}

#[test]
fn every_snapshot_of_a_run_passes_all_five_invariants() {
    let (net, mut trace) = (net(), Trace::new(Snapshot::initial()));
    for k in 0..8 {
        fill(&mut trace, &net, k % 4);
    }
    for k in 0..8 {
        trace.push(&net, T::take(k % 3)).unwrap();
        trace.push(&net, T::complete(k % 3)).unwrap();
    }
    assert!(trace.snapshots().iter().all(|s| s.check().is_ok()));
    let end = trace.snapshot(trace.last()).unwrap();
    assert_eq!(
        end.totals,
        Totals {
            puts: 8,
            takes: 8,
            completes: 8
        }
    );
}

#[test]
fn a_snapshot_whose_counters_disagree_with_its_marking_is_caught() {
    let mut snapshot = Snapshot::initial();
    snapshot.totals.puts = 1;
    assert_eq!(snapshot.check(), Err(Violation::BufferAccounting));
    let mut snapshot = Snapshot::initial();
    snapshot.totals.completes = 1;
    assert_eq!(
        snapshot.check(),
        Err(Violation::BufferAccounting).or(Err(Violation::ProcessingAccounting))
    );
}

#[test]
fn the_budget_is_checked_before_the_trace_changes() {
    let net = net();
    let mut trace = Trace::new(Snapshot::initial());
    // A Start/Finish/Put/Take/Complete round has five events and returns to the same marking, so the trace can be
    // filled.
    while trace.last() + 5 <= MAX_EVENTS {
        for t in [T::start(0), T::finish(0), T::put(0), T::take(0), T::complete(0)] {
            trace.push(&net, t).unwrap();
        }
    }
    assert_eq!(trace.last(), MAX_EVENTS);
    let before = trace.clone();
    assert_eq!(trace.push(&net, T::start(1)), Err(TraceError::Budget));
    assert_eq!(trace, before);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn script() -> Trace {
    let (net, mut trace) = (net(), Trace::new(Snapshot::initial()));
    fill(&mut trace, &net, 0);
    trace
}

#[test]
fn a_branch_starts_at_its_own_index_zero_with_the_counters_of_its_source() {
    let (net, mut trace) = (net(), script());
    trace.push(&net, T::take(0)).unwrap();
    let source = *trace.snapshot(4).unwrap();
    let branch = Branch::fork(&source);
    assert_eq!((branch.trace().last(), branch.cursor()), (0, 0));
    let first = branch.trace().snapshot(0).unwrap();
    assert_eq!(first.marking, source.marking);
    assert_eq!(first.totals, source.totals);
    assert_eq!(first.last, None, "the branch has no events of its own yet");
}

#[test]
fn a_candidate_leaves_the_branch_as_it_was() {
    let net = net();
    let branch = Branch::fork(&Snapshot::initial());
    let candidate = branch.candidate(&net, T::start(0)).unwrap();
    assert_eq!((branch.trace().last(), branch.cursor()), (0, 0));
    assert_eq!((candidate.trace().last(), candidate.cursor()), (1, 1));
}

#[test]
fn firing_from_an_older_snapshot_drops_the_later_ones() {
    let net = net();
    let mut branch = Branch::fork(&Snapshot::initial());
    for t in [T::start(0), T::finish(0), T::put(0)] {
        branch = branch.candidate(&net, t).unwrap();
    }
    assert_eq!(branch.trace().last(), 3);
    branch.set_cursor(1);
    let branch = branch.candidate(&net, T::start(1)).unwrap();
    assert_eq!((branch.trace().last(), branch.cursor()), (2, 2));
    let events: Vec<_> = branch
        .trace()
        .snapshots()
        .iter()
        .filter_map(|s| s.last)
        .map(|e| e.transition)
        .collect();
    assert_eq!(events, vec![T::start(0), T::start(1)]);
}

#[test]
fn a_disabled_candidate_is_refused_and_the_cursor_stays() {
    let net = net();
    let branch = Branch::fork(&Snapshot::initial());
    assert!(branch.candidate(&net, T::put(0)).is_err());
    assert_eq!(branch.cursor(), 0);
}

#[test]
fn the_cursor_cannot_pass_the_tip() {
    let net = net();
    let mut branch = Branch::fork(&Snapshot::initial()).candidate(&net, T::start(0)).unwrap();
    branch.set_cursor(99);
    assert_eq!(branch.cursor(), 1);
}

#[test]
fn a_branch_at_the_budget_refuses_a_firing_but_a_rewound_one_still_takes_it() {
    let net = net();
    let mut branch = Branch::fork(&Snapshot::initial());
    while branch.trace().last() + 5 <= MAX_EVENTS {
        for t in [T::start(0), T::finish(0), T::put(0), T::take(0), T::complete(0)] {
            branch = branch.candidate(&net, t).unwrap();
        }
    }
    assert_eq!(branch.candidate(&net, T::start(1)).unwrap_err(), TraceError::Budget);
    branch.set_cursor(10);
    assert!(branch.candidate(&net, T::start(1)).is_ok());
}
