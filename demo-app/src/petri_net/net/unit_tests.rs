use super::*;
use std::collections::{HashMap, HashSet};

fn net() -> Net {
    Net::bounded_buffer()
}

fn fire_all(net: &Net, from: Marking, events: &[TransitionId]) -> Marking {
    events
        .iter()
        .fold(from, |m, &t| net.fire(&m, t).expect("an enabled transition"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_topology_has_the_counts_of_the_design() {
    let net = net();
    assert_eq!((PLACES, TRANSITIONS, ARCS), (20, 18, 50));
    let arcs = net.arcs();
    assert_eq!(arcs.len(), 50);
    let unique: HashSet<(u8, u8, bool)> = arcs.iter().map(|a| (a.place.0, a.transition.0, a.input)).collect();
    assert_eq!(unique.len(), 50, "every arc is different");
}

#[test]
fn every_arc_joins_a_place_to_a_transition_and_each_transition_has_both_sides() {
    let net = net();
    for t in TransitionId::all() {
        let spec = net.spec(t);
        assert!(!spec.pre.is_empty() && !spec.post.is_empty(), "{}", t.name());
        assert!(spec.pre.iter().chain(&spec.post).all(|p| p.index() < PLACES));
    }
}

#[test]
fn each_hub_has_seven_arcs() {
    let arcs = net().arcs();
    for hub in [PlaceId::FREE, PlaceId::USED] {
        assert_eq!(arcs.iter().filter(|a| a.place == hub).count(), 7);
    }
}

#[test]
fn each_put_and_take_has_four_arcs_and_the_other_transitions_two() {
    let arcs = net().arcs();
    for t in TransitionId::all() {
        let count = arcs.iter().filter(|a| a.transition == t).count();
        let expected = match t.kind() {
            TransitionKind::Put | TransitionKind::Take => 4,
            _ => 2,
        };
        assert_eq!(count, expected, "{}", t.name());
    }
}

#[test]
fn ids_and_names_match_the_table_of_the_design() {
    assert_eq!(PlaceId::producer_idle(0).name(), "P0.Idle");
    assert_eq!(PlaceId::producer_ready(3).name(), "P3.Ready");
    assert_eq!(PlaceId::consumer_processing(2).name(), "C2.Processing");
    assert_eq!(
        (PlaceId::FREE.name(), PlaceId::USED.name()),
        ("Free".to_owned(), "Used".to_owned())
    );
    assert_eq!(TransitionId::put(2).name(), "Put[2]");
    assert_eq!(TransitionId::complete(1).name(), "Complete[1]");
    let names: HashSet<String> = PlaceId::all().map(PlaceId::name).collect();
    assert_eq!(names.len(), PLACES);
    let names: HashSet<String> = TransitionId::all().map(TransitionId::name).collect();
    assert_eq!(names.len(), TRANSITIONS);
}

#[test]
fn the_initial_marking_is_the_one_of_the_design() {
    let m = Marking::initial();
    assert_eq!((m.free(), m.used()), (8, 0));
    for i in 0..PRODUCERS {
        assert_eq!(m.get(PlaceId::producer_idle(i)), 1);
        assert_eq!(m.get(PlaceId::producer_busy(i)) + m.get(PlaceId::producer_ready(i)), 0);
    }
    for j in 0..CONSUMERS {
        assert_eq!(m.get(PlaceId::consumer_idle(j)), 1);
    }
    assert_eq!(m.check(), Ok(()));
}

#[test]
fn at_the_start_only_the_four_starts_are_enabled() {
    let net = net();
    let enabled = net.enabled(&Marking::initial());
    let expected: Vec<_> = (0..PRODUCERS).map(TransitionId::start).collect();
    assert_eq!(enabled.iter().collect::<Vec<_>>(), expected);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn start_finish_and_put_move_one_producer_round() {
    let net = net();
    let m0 = Marking::initial();
    let m1 = net.fire(&m0, TransitionId::start(1)).unwrap();
    assert_eq!((m1.get(PlaceId::producer_idle(1)), m1.get(PlaceId::producer_busy(1))), (0, 1));
    let m2 = net.fire(&m1, TransitionId::finish(1)).unwrap();
    assert_eq!((m2.get(PlaceId::producer_busy(1)), m2.get(PlaceId::producer_ready(1))), (0, 1));
    let m3 = net.fire(&m2, TransitionId::put(1)).unwrap();
    assert_eq!(m3.get(PlaceId::producer_idle(1)), 1);
    assert_eq!((m3.free(), m3.used()), (7, 1));
}

#[test]
fn take_frees_the_slot_at_once_and_complete_only_idles_the_consumer() {
    let net = net();
    let m = fire_all(
        &net,
        Marking::initial(),
        &[TransitionId::start(0), TransitionId::finish(0), TransitionId::put(0)],
    );
    let taken = net.fire(&m, TransitionId::take(2)).unwrap();
    assert_eq!((taken.free(), taken.used()), (8, 0), "the slot is released at the Take");
    assert_eq!(taken.get(PlaceId::consumer_processing(2)), 1);
    let done = net.fire(&taken, TransitionId::complete(2)).unwrap();
    assert_eq!(done, Marking::initial());
}

#[test]
fn a_disabled_transition_returns_its_missing_places_and_changes_nothing() {
    let net = net();
    let m = Marking::initial();
    for (t, missing) in [
        (TransitionId::finish(0), vec![PlaceId::producer_busy(0)]),
        (TransitionId::put(0), vec![PlaceId::producer_ready(0)]),
        (TransitionId::take(1), vec![PlaceId::USED]),
        (TransitionId::complete(2), vec![PlaceId::consumer_processing(2)]),
    ] {
        assert_eq!(net.fire(&m, t), Err(FireError::Disabled(missing)), "{}", t.name());
    }
}

#[test]
fn a_transition_missing_two_inputs_lists_the_actor_place_first() {
    let net = net();
    let m = Marking::initial();
    // Both inputs of a Put are absent at the start.
    let mut full = m;
    full.tokens[PlaceId::FREE.index()] = 0;
    full.tokens[PlaceId::USED.index()] = 8;
    assert_eq!(
        net.missing(&full, TransitionId::put(3)),
        vec![PlaceId::producer_ready(3), PlaceId::FREE]
    );
    assert_eq!(
        net.missing(&m, TransitionId::take(0)),
        vec![PlaceId::USED],
        "the consumer is idle, so only the buffer is missing"
    );
}

#[test]
fn a_put_into_a_full_buffer_is_refused_and_the_producer_stays_ready() {
    let net = net();
    let mut m = Marking::initial();
    for k in 0..8 {
        let i = k % 4;
        m = fire_all(
            &net,
            m,
            &[TransitionId::start(i), TransitionId::finish(i), TransitionId::put(i)],
        );
    }
    assert_eq!((m.free(), m.used()), (0, 8));
    let ready = fire_all(&net, m, &[TransitionId::start(0), TransitionId::finish(0)]);
    assert_eq!(ready.get(PlaceId::producer_ready(0)), 1);
    assert_eq!(
        net.fire(&ready, TransitionId::put(0)),
        Err(FireError::Disabled(vec![PlaceId::FREE]))
    );
    assert_eq!(blocked(&ready), vec![Blocked::Full(0)]);
}

#[test]
fn consumers_are_blocked_only_while_idle_and_the_buffer_is_empty() {
    let net = net();
    let m = Marking::initial();
    assert_eq!(blocked(&m), vec![Blocked::Empty(0), Blocked::Empty(1), Blocked::Empty(2)]);
    let m = fire_all(
        &net,
        m,
        &[TransitionId::start(0), TransitionId::finish(0), TransitionId::put(0)],
    );
    assert_eq!(blocked(&m), vec![]);
    let m = fire_all(&net, m, &[TransitionId::take(1)]);
    assert_eq!(
        blocked(&m),
        vec![Blocked::Empty(0), Blocked::Empty(2)],
        "a processing consumer is not blocked"
    );
}

#[test]
fn one_free_slot_and_two_ready_producers_enable_both_puts_but_only_one_can_fire() {
    let net = net();
    let mut m = Marking::initial();
    for k in 0..7 {
        let i = k % 4;
        m = fire_all(
            &net,
            m,
            &[TransitionId::start(i), TransitionId::finish(i), TransitionId::put(i)],
        );
    }
    let m = fire_all(
        &net,
        m,
        &[
            TransitionId::start(0),
            TransitionId::finish(0),
            TransitionId::start(1),
            TransitionId::finish(1),
        ],
    );
    assert!(net.is_enabled(&m, TransitionId::put(0)) && net.is_enabled(&m, TransitionId::put(1)));
    let after = net.fire(&m, TransitionId::put(0)).unwrap();
    assert!(!net.is_enabled(&after, TransitionId::put(1)), "the one free token is gone");
    assert!(net.fire(&after, TransitionId::put(1)).is_err());
    let released = net.fire(&after, TransitionId::take(0)).unwrap();
    assert!(net.is_enabled(&released, TransitionId::put(1)), "a Take enables it again");
}

#[test]
fn firing_does_not_change_the_marking_it_was_given() {
    let net = net();
    let m = Marking::initial();
    let copy = m;
    let _ = net.fire(&m, TransitionId::start(0));
    let _ = net.fire(&m, TransitionId::put(0));
    assert_eq!(m, copy);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_net_with_the_wrong_parts_is_refused_with_a_typed_error() {
    let rows = Net::table();
    assert_eq!(
        Net::from_specs(PLACES - 1, rows.clone()),
        Err(NetError::WrongCount {
            places: PLACES - 1,
            transitions: TRANSITIONS
        })
    );
    let mut short = rows.clone();
    short.pop();
    assert!(matches!(Net::from_specs(PLACES, short), Err(NetError::WrongCount { .. })));

    let mut unknown = rows.clone();
    unknown[0].post = vec![PlaceId(99)];
    assert_eq!(
        Net::from_specs(PLACES, unknown),
        Err(NetError::UnknownPlace(TransitionId(0), PlaceId(99)))
    );

    let mut twice = rows.clone();
    twice[2].pre = vec![PlaceId::FREE, PlaceId::FREE];
    assert_eq!(
        Net::from_specs(PLACES, twice),
        Err(NetError::DuplicateArc(TransitionId(2), PlaceId::FREE))
    );

    let mut empty = rows;
    empty[5].pre.clear();
    assert_eq!(Net::from_specs(PLACES, empty), Err(NetError::NoArcs(TransitionId(5))));
}

#[test]
fn a_marking_that_breaks_a_rule_names_the_rule() {
    let mut m = Marking::initial();
    m.tokens[PlaceId::producer_busy(2).index()] = 1;
    assert_eq!(m.check(), Err(Violation::ProducerExclusivity(2)));
    let mut m = Marking::initial();
    m.tokens[PlaceId::consumer_idle(1).index()] = 0;
    assert_eq!(m.check(), Err(Violation::ConsumerExclusivity(1)));
    let mut m = Marking::initial();
    m.tokens[PlaceId::USED.index()] = 1;
    assert_eq!(m.check(), Err(Violation::Capacity));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every reachable marking, found by a breadth-first search, with the markings that each reaches in one step.
fn reachable(net: &Net) -> (Vec<Marking>, HashMap<Marking, Vec<Marking>>) {
    let start = Marking::initial();
    let mut order = vec![start];
    let mut seen: HashSet<Marking> = HashSet::from([start]);
    let mut successors: HashMap<Marking, Vec<Marking>> = HashMap::new();
    let mut at = 0;
    while at < order.len() {
        let m = order[at];
        at += 1;
        for t in TransitionId::all() {
            if let Ok(next) = net.fire(&m, t) {
                successors.entry(m).or_default().push(next);
                if seen.insert(next) {
                    order.push(next);
                }
            }
        }
    }
    (order, successors)
}

#[test]
fn exactly_5832_markings_are_reachable_and_every_one_obeys_the_invariants() {
    let (all, _) = reachable(&net());
    assert_eq!(all.len(), 5832);
    assert_eq!(5832, 3usize.pow(4) * 2usize.pow(3) * 9);
    for m in &all {
        assert_eq!(m.check(), Ok(()));
        assert!(m.tokens.iter().all(|&n| n <= CAPACITY));
    }
}

#[test]
fn no_reachable_marking_is_dead() {
    let net = net();
    let (all, successors) = reachable(&net);
    assert!(all.iter().all(|m| !net.enabled(m).is_empty()));
    assert!(all.iter().all(|m| successors.contains_key(m)));
}

#[test]
fn the_initial_marking_is_a_home_marking() {
    let (all, successors) = reachable(&net());
    let mut backwards: HashMap<Marking, Vec<Marking>> = HashMap::new();
    for (from, tos) in &successors {
        for to in tos {
            backwards.entry(*to).or_default().push(*from);
        }
    }
    let start = Marking::initial();
    let mut seen = HashSet::from([start]);
    let mut stack = vec![start];
    while let Some(m) = stack.pop() {
        for &before in backwards.get(&m).into_iter().flatten() {
            if seen.insert(before) {
                stack.push(before);
            }
        }
    }
    assert_eq!(seen.len(), all.len());
}

#[test]
fn the_net_is_deterministic() {
    let net = net();
    let (all, _) = reachable(&net);
    for m in all.iter().step_by(37) {
        for t in TransitionId::all() {
            assert_eq!(net.fire(m, t), net.fire(m, t));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A second interpreter that works from an incidence matrix and not the arc lists, to check the production one against.
fn incidence_fire(net: &Net, m: &Marking, t: TransitionId) -> Option<Marking> {
    let mut pre = [0i16; PLACES];
    let mut post = [0i16; PLACES];
    for &p in &net.spec(t).pre {
        pre[p.index()] += 1;
    }
    for &p in &net.spec(t).post {
        post[p.index()] += 1;
    }
    let mut next = *m;
    for p in 0..PLACES {
        if i16::from(m.tokens[p]) < pre[p] {
            return None;
        }
        next.tokens[p] = u8::try_from(i16::from(m.tokens[p]) - pre[p] + post[p]).ok()?;
    }
    Some(next)
}

#[test]
fn the_production_interpreter_agrees_with_the_incidence_matrix_everywhere() {
    let net = net();
    let (all, _) = reachable(&net);
    for m in &all {
        for t in TransitionId::all() {
            assert_eq!(net.fire(m, t).ok(), incidence_fire(&net, m, t), "{} at {m:?}", t.name());
        }
    }
}

/// A deliberately wrong interpreter, only in the test build: a Put forgets to take the Free token.
fn defective_fire(net: &Net, m: &Marking, t: TransitionId) -> Option<Marking> {
    if !net.is_enabled(m, t) {
        return None;
    }
    let mut next = *m;
    for &p in &net.spec(t).pre {
        if !(t.kind() == TransitionKind::Put && p == PlaceId::FREE) {
            next.tokens[p.index()] -= 1;
        }
    }
    for &p in &net.spec(t).post {
        next.tokens[p.index()] += 1;
    }
    Some(next)
}

#[test]
fn the_negative_control_shows_that_the_checks_notice_a_put_that_keeps_its_free_token() {
    let net = net();
    let mut m = Marking::initial();
    for t in [TransitionId::start(0), TransitionId::finish(0), TransitionId::put(0)] {
        m = defective_fire(&net, &m, t).expect("enabled");
    }
    assert_eq!((m.free(), m.used()), (8, 1), "the defect left Free alone");
    assert_eq!(m.check(), Err(Violation::Capacity));
    // The accounting equation notices it too, on the snapshot that carries the counters.
    let snapshot = super::super::trace::Snapshot {
        marking: m,
        totals: super::super::trace::Totals { puts: 1, ..Default::default() },
        last: None,
    };
    assert!(snapshot.check().is_err());
}

#[test]
fn the_production_interpreter_refuses_to_keep_a_free_token() {
    let net = net();
    let m = fire_all(
        &net,
        Marking::initial(),
        &[TransitionId::start(0), TransitionId::finish(0), TransitionId::put(0)],
    );
    assert_eq!((m.free(), m.used()), (7, 1));
}
