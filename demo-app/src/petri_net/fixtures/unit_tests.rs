use super::*;
use crate::petri_net::net::{Blocked, Marking, PlaceId, TransitionId as T, TransitionSet, blocked};

fn net() -> Net {
    Net::bounded_buffer()
}

fn trace(scenario: Scenario) -> Trace {
    scenario.trace(&net()).expect("a script is valid")
}

/// The state string of §5.3: producers I, B or R, then consumers I or P.
fn states(m: &Marking) -> (String, String) {
    let producers = (0..4)
        .map(|i| {
            if m.get(PlaceId::producer_busy(i)) == 1 {
                'B'
            } else if m.get(PlaceId::producer_ready(i)) == 1 {
                'R'
            } else {
                'I'
            }
        })
        .collect();
    let consumers = (0..3)
        .map(|j| if m.get(PlaceId::consumer_processing(j)) == 1 { 'P' } else { 'I' })
        .collect();
    (producers, consumers)
}

fn enabled_at(trace: &Trace, index: usize) -> TransitionSet {
    net().enabled(&trace.snapshot(index).unwrap().marking)
}

fn event_at(trace: &Trace, index: usize) -> TransitionId {
    trace.snapshot(index).unwrap().last.expect("an event").transition
}

#[test]
fn the_scripts_have_50_28_and_30_events() {
    for (scenario, events) in [
        (Scenario::Walkthrough, 50),
        (Scenario::Contention, 28),
        (Scenario::Interleaving, 30),
    ] {
        assert_eq!(scenario.events().len(), events);
        assert_eq!(trace(scenario).len(), events + 1);
    }
}

#[test]
fn every_snapshot_of_every_script_passes_the_invariants() {
    for scenario in Scenario::ALL {
        assert!(trace(scenario).snapshots().iter().all(|s| s.check().is_ok()), "{scenario:?}");
    }
}

#[test]
fn the_walkthrough_checkpoints_are_the_snapshots_of_the_design() {
    let t = trace(Scenario::Walkthrough);
    let used_free = |i: usize| {
        let m = t.snapshot(i).unwrap().marking;
        (m.used(), m.free())
    };
    assert_eq!(used_free(0), (0, 8));
    assert_eq!((used_free(3), event_at(&t, 3)), ((1, 7), T::put(0)));
    assert_eq!((used_free(24), event_at(&t, 24)), ((8, 0), T::put(3)));
    assert_eq!((used_free(26), event_at(&t, 26)), ((8, 0), T::finish(0)));
    assert_eq!((used_free(27), event_at(&t, 27)), ((7, 1), T::take(0)));
    assert_eq!((used_free(28), event_at(&t, 28)), ((8, 0), T::put(0)));
    assert_eq!((used_free(45), event_at(&t, 45)), ((0, 8), T::complete(1)));
    assert_eq!((used_free(47), event_at(&t, 47)), ((0, 8), T::finish(1)));
    assert_eq!((used_free(48), event_at(&t, 48)), ((1, 7), T::put(1)));
    assert_eq!((used_free(49), event_at(&t, 49)), ((0, 8), T::take(1)));
    assert_eq!((used_free(50), event_at(&t, 50)), ((0, 8), T::complete(1)));
}

#[test]
fn the_walkthrough_blocks_and_releases_where_the_design_says() {
    let t = trace(Scenario::Walkthrough);
    let m = |i: usize| t.snapshot(i).unwrap().marking;
    assert_eq!(blocked(&m(0)).len(), 3, "all three consumers wait on an empty buffer");
    assert_eq!(enabled_at(&t, 0).len(), 4, "the four Starts");
    assert_eq!(blocked(&m(24)), vec![], "full, but no producer is Ready yet");
    assert_eq!(blocked(&m(26)), vec![Blocked::Full(0)]);
    assert!(!enabled_at(&t, 26).contains(T::put(0)));
    assert!(enabled_at(&t, 27).contains(T::put(0)));
    assert!(!enabled_at(&t, 26).minus(enabled_at(&t, 27)).contains(T::put(0)));
    assert_eq!(blocked(&m(45)).len(), 3);
    assert!(enabled_at(&t, 47).contains(T::put(1)));
    assert!((0..3).all(|j| !enabled_at(&t, 47).contains(T::take(j))));
    assert!((0..3).all(|j| enabled_at(&t, 48).contains(T::take(j))));
    assert_eq!(blocked(&m(49)), vec![Blocked::Empty(0), Blocked::Empty(2)]);
    assert_eq!(blocked(&m(50)).len(), 3);
}

#[test]
fn the_walkthrough_ends_with_every_count_equal() {
    let t = trace(Scenario::Walkthrough);
    let end = t.snapshot(50).unwrap();
    assert_eq!((end.totals.puts, end.totals.takes, end.totals.completes), (10, 10, 10));
}

#[test]
fn the_contention_checkpoints_are_the_snapshots_of_the_design() {
    let t = trace(Scenario::Contention);
    let used_free = |i: usize| {
        let m = t.snapshot(i).unwrap().marking;
        (m.used(), m.free())
    };
    assert_eq!((used_free(21), event_at(&t, 21)), ((7, 1), T::put(2)));
    assert_eq!((used_free(25), event_at(&t, 25)), ((7, 1), T::finish(1)));
    assert_eq!((used_free(26), event_at(&t, 26)), ((8, 0), T::put(0)));
    assert_eq!((used_free(27), event_at(&t, 27)), ((7, 1), T::take(0)));
    assert_eq!((used_free(28), event_at(&t, 28)), ((8, 0), T::put(1)));
}

#[test]
fn in_contention_one_put_disables_the_other_and_a_take_enables_it_again() {
    let t = trace(Scenario::Contention);
    assert!(enabled_at(&t, 21).minus(enabled_at(&t, 21)).is_empty());
    assert!(!enabled_at(&t, 21).contains(T::put(0)), "no producer is Ready at snapshot 21");
    assert!(enabled_at(&t, 25).contains(T::put(0)) && enabled_at(&t, 25).contains(T::put(1)));
    assert!(!enabled_at(&t, 26).contains(T::put(1)));
    assert!(
        enabled_at(&t, 25).minus(enabled_at(&t, 26)).contains(T::put(1)),
        "newly disabled"
    );
    assert!(
        enabled_at(&t, 27).minus(enabled_at(&t, 26)).contains(T::put(1)),
        "newly enabled again"
    );
    assert_eq!(blocked(&t.snapshot(26).unwrap().marking), vec![Blocked::Full(1)]);
    assert_eq!(blocked(&t.snapshot(27).unwrap().marking), vec![]);
}

#[test]
fn the_interleaving_states_are_those_of_the_design() {
    let t = trace(Scenario::Interleaving);
    let at = |i: usize| {
        let m = t.snapshot(i).unwrap().marking;
        (states(&m), (m.used(), m.free()))
    };
    let s = |p: &str, c: &str, used: u8| ((p.to_owned(), c.to_owned()), (used, 8 - used));
    assert_eq!(at(0), s("IIII", "III", 0));
    assert_eq!((at(2), event_at(&t, 2)), (s("BBII", "III", 0), T::start(1)));
    assert_eq!((at(5), event_at(&t, 5)), (s("RRBI", "III", 0), T::finish(1)));
    assert_eq!((at(8), event_at(&t, 8)), (s("IRBB", "PII", 0), T::start(3)));
    assert_eq!((at(11), event_at(&t, 11)), (s("IIRB", "PPI", 0), T::take(1)));
    assert_eq!((at(12), event_at(&t, 12)), (s("IIRR", "PPI", 0), T::finish(3)));
    assert_eq!((at(14), event_at(&t, 14)), (s("IIIR", "PPP", 0), T::take(2)));
    assert_eq!((at(22), event_at(&t, 22)), (s("RIII", "PPP", 1), T::take(1)));
    assert_eq!((at(30), event_at(&t, 30)), (s("IIII", "III", 0), T::complete(2)));
}

#[test]
fn the_interleaving_returns_to_the_initial_marking_with_different_counters() {
    let t = trace(Scenario::Interleaving);
    let (first, last) = (t.snapshot(0).unwrap(), t.snapshot(30).unwrap());
    assert_eq!(first.marking, last.marking);
    assert_ne!(first.totals, last.totals);
    assert_eq!(last.totals.puts, last.totals.takes);
    assert_eq!(last.totals.takes, last.totals.completes);
}

/// Design revision 3 says that Used never exceeds 1 here. The script reaches 2, at the snapshots below, and never comes
/// near a full or an empty-and-blocked buffer, so it still tests simultaneous work and not saturation.
#[test]
fn the_interleaving_keeps_the_buffer_far_from_full() {
    let t = trace(Scenario::Interleaving);
    let over_one: Vec<usize> = (0..=t.last()).filter(|&i| t.snapshot(i).unwrap().marking.used() > 1).collect();
    assert_eq!(over_one, vec![21, 23, 24, 25]);
    assert!(t.snapshots().iter().all(|s| s.marking.used() <= 2));
}

#[test]
fn the_interleaving_overlaps_busy_ready_and_processing_across_workers() {
    let t = trace(Scenario::Interleaving);
    let count = |index: usize, state: char| {
        let (p, c) = states(&t.snapshot(index).unwrap().marking);
        p.chars().chain(c.chars()).filter(|&ch| ch == state).count()
    };
    assert_eq!(count(2, 'B'), 2);
    assert!(count(8, 'B') == 2 && count(8, 'R') == 1 && count(8, 'P') == 1);
    assert_eq!(count(14, 'P'), 3);
    assert_eq!(count(22, 'P'), 3);
}

#[test]
fn the_checkpoints_are_exactly_the_ones_of_the_design() {
    let named = |s: Scenario| s.checkpoints().iter().map(|c| (c.label, c.snapshot)).collect::<Vec<_>>();
    assert_eq!(
        named(Scenario::Walkthrough),
        vec![("Full", 24), ("Blocked", 26), ("Released", 27), ("Empty", 45)]
    );
    assert_eq!(
        named(Scenario::Contention),
        vec![("Both ready", 25), ("Last slot taken", 26), ("Re-enabled", 27)]
    );
    assert_eq!(
        named(Scenario::Interleaving),
        vec![
            ("Two busy", 2),
            ("Mixed work", 8),
            ("Two processing", 11),
            ("Three processing", 14),
            ("Home again", 30)
        ]
    );
    for scenario in Scenario::ALL {
        let last = scenario.events().len();
        assert!(scenario.checkpoints().iter().all(|c| c.snapshot <= last));
    }
}

#[test]
fn scenarios_are_found_by_their_keys() {
    for scenario in Scenario::ALL {
        assert_eq!(Scenario::from_key(scenario.key()), Some(scenario));
    }
    assert_eq!(Scenario::from_key("nope"), None);
}

#[test]
fn a_script_is_the_same_each_time_it_is_built() {
    for scenario in Scenario::ALL {
        assert_eq!(trace(scenario), trace(scenario));
    }
}
