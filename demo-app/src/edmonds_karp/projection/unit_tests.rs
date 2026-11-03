//! What each snapshot shows: the names, the text, the highlights, the buttons that are at an end, and that the writes
//! of a diff leave a page the same as the writes of a full redraw, at every step.

use super::*;
use crate::edmonds_karp::fixtures::{CANONICAL, FIXTURES, Fixture, REVERSE};

fn trace_of(fixture: &Fixture) -> Trace {
    Trace::build(&fixture.graph()).expect("a trace")
}

fn view() -> View {
    View::first(Prefs::default(), Speed::Two)
}

fn shown(trace: &Trace, index: usize, view: &View) -> Presentation {
    present_at(trace, index, view).expect("a snapshot")
}

/// Every preference that can be set, in every combination.
fn all_prefs() -> Vec<Prefs> {
    let mut all = Vec::new();
    for bits in 0..8 {
        all.push(Prefs {
            show_residual: bits & 1 != 0,
            show_queue: bits & 2 != 0,
            detailed: bits & 4 != 0,
        });
    }
    all
}

// The names.

#[test]
fn an_edge_is_named_with_its_id_ends_flow_capacity_and_both_residuals() {
    let trace = trace_of(&REVERSE);
    // The spec's own example: e0 is v1 to v3 with capacity 2, and carries 1.
    assert_eq!(
        edge_name(&trace, 0, 1),
        "e0 v1 to v3; flow 1 of 2; forward residual 1; reverse residual 1"
    );
    assert_eq!(
        edge_name(&trace, 4, 0),
        "e4 s to v1; flow 0 of 2; forward residual 2; reverse residual 0"
    );
}

#[test]
fn a_residual_transition_names_its_parent_edge_and_its_direction() {
    let trace = trace_of(&REVERSE);
    assert_eq!(
        lane_name(&trace, 0, Direction::Reverse, 2),
        "reverse of e0 v3 to v1, capacity 2"
    );
    assert_eq!(
        lane_name(&trace, 0, Direction::Forward, 1),
        "forward of e0 v1 to v3, capacity 1"
    );
}

#[test]
fn the_antiparallel_edges_and_their_transitions_are_told_apart_by_their_ids() {
    let trace = trace_of(&CANONICAL);
    let e2 = edge_name(&trace, 2, 0);
    let e3 = edge_name(&trace, 3, 0);
    assert!(e2.starts_with("e2 v1 to v2") && e3.starts_with("e3 v2 to v1"));
    // The forward transition of e2 and the reverse transition of e3 both go from v1 to v2. Only the names differ.
    assert_ne!(
        lane_name(&trace, 2, Direction::Forward, 5),
        lane_name(&trace, 3, Direction::Reverse, 5)
    );
}

// The text.

#[test]
fn the_event_line_changes_at_every_step_of_both_graphs() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        let lines: Vec<String> = trace.snapshots().iter().map(|s| event_line(&trace, s)).collect();
        let unique: BTreeSet<&String> = lines.iter().collect();
        assert_eq!(unique.len(), lines.len(), "{}: every step has its own line", fixture.key);
        for (at, line) in lines.iter().enumerate() {
            assert!(line.starts_with(&format!("#{at} ")), "{line}");
        }
    }
}

#[test]
fn the_event_line_says_the_edge_direction_ends_and_outcome_of_an_examination() {
    let trace = trace_of(&CANONICAL);
    let examined = trace
        .snapshots()
        .iter()
        .find(|s| matches!(s.event, Event::ArcExamined { already_discovered: true, .. }))
        .expect("an examination of a vertex already discovered");
    let line = event_line(&trace, examined);
    assert!(
        line.contains("ArcExamined") && line.contains(" F ") && line.contains('\u{2192}'),
        "{line}"
    );
    assert!(line.contains("already discovered"), "{line}");
}

#[test]
fn the_panel_has_a_field_for_everything_that_the_page_shows() {
    let trace = trace_of(&CANONICAL);
    let text = panel_text(&trace, trace.snapshot(0).expect("a snapshot"), Prefs::default());
    assert_eq!(text.len(), PANEL.len());
    assert_eq!(text[0], "Step 0 of 86");
    assert_eq!(text[1], "Initial");
}

#[test]
fn the_panel_shows_the_queue_the_path_the_bottleneck_and_the_total_in_text() {
    let trace = trace_of(&REVERSE);
    let at = trace.augmentation_indices()[1];
    let found = at - 2;
    let bottleneck = panel_text(&trace, trace.snapshot(at - 1).expect("a snapshot"), Prefs::default());
    assert_eq!(bottleneck[8], "1");
    let after = panel_text(&trace, trace.snapshot(at).expect("a snapshot"), Prefs::default());
    assert!(
        after[7].starts_with("s \u{2192} v4 \u{2192} v3 \u{2192} v1 \u{2192} v2 \u{2192} t"),
        "{}",
        after[7]
    );
    assert!(after[9].contains("e0 R takes back 1: 2 to 1"), "{}", after[9]);
    assert!(after[9].contains("total 2 to 3"));
    assert_eq!(after[10], "3");
    let queue_at = trace
        .snapshots()
        .iter()
        .find(|s| s.bfs.queue.len() > 1)
        .expect("a queue of two");
    assert!(panel_text(&trace, queue_at, Prefs::default())[3].contains(','));
    let _ = found;
}

#[test]
fn the_cut_shows_both_sets_the_crossing_edges_and_the_equation() {
    let trace = trace_of(&CANONICAL);
    let last = trace.snapshots().last().expect("a snapshot");
    let text = panel_text(&trace, last, Prefs::default());
    assert_eq!(
        text[11],
        "S = s, v1, v2, v4; T = v3, t; crossing e4, e7, e9; max flow = min cut = 23"
    );
    assert!(event_line(&trace, last).contains("max flow = min cut = 23"));
}

#[test]
fn the_list_of_what_was_looked_at_is_only_there_in_the_detailed_mode() {
    let trace = trace_of(&CANONICAL);
    let examined = trace
        .snapshots()
        .iter()
        .rfind(|s| matches!(s.event, Event::ArcExamined { .. }))
        .expect("one");
    let detailed = panel_text(
        &trace,
        examined,
        Prefs {
            detailed: true,
            ..Prefs::default()
        },
    );
    let compact = panel_text(
        &trace,
        examined,
        Prefs {
            detailed: false,
            ..Prefs::default()
        },
    );
    assert_ne!(detailed[6], NOTHING);
    assert_eq!(compact[6], NOTHING);
    // Everything else, and above all the line for the event, is the same in both.
    for (at, (a, b)) in detailed.iter().zip(&compact).enumerate() {
        if at != 6 {
            assert_eq!(a, b, "field {at}");
        }
    }
}

// What the picture shows.

#[test]
fn every_graph_element_has_a_slot_and_the_slots_count_is_exact() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        let slots = Slots::new(trace.vertex_count(), trace.edges().len());
        let page = shown(&trace, 0, &view());
        assert_eq!(page.slots.len(), slots.count());
        let mut seen = BTreeSet::new();
        for v in 0..trace.vertex_count() {
            assert!(seen.insert(slots.vertex(v)) && seen.insert(slots.badge(v)));
        }
        for e in 0..trace.edges().len() {
            assert!(seen.insert(slots.edge(e)) && seen.insert(slots.edge_label(e)));
            for d in [Direction::Forward, Direction::Reverse] {
                assert!(seen.insert(slots.lane(e, d)) && seen.insert(slots.lane_label(e, d)));
            }
        }
        assert_eq!(seen.len(), slots.count(), "no two elements share a slot");
    }
}

#[test]
fn the_original_edge_label_is_always_there_and_a_zero_edge_reads_zero_over_zero() {
    let trace = Trace::build(&super::super::solver::Graph {
        vertices: vec![0, 1],
        source: 0,
        sink: 1,
        edges: vec![super::super::solver::EdgeSpec {
            id: 0,
            from: 0,
            to: 1,
            capacity: 0,
        }],
    })
    .expect("a trace");
    let slots = Slots::new(2, 1);
    for index in 0..trace.len() {
        let page = shown(&trace, index, &view());
        assert_eq!(page.slots[slots.edge_label(0)].text.as_deref(), Some("0/0"));
        assert!(page.slots[slots.edge(0)].class.contains("ek-zero"));
        for d in [Direction::Forward, Direction::Reverse] {
            assert!(page.slots[slots.lane(0, d)].class.contains("ek-off"), "it has no transition");
        }
    }
}

#[test]
fn a_lane_without_room_is_hidden_and_the_others_keep_their_slots() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let first = shown(&trace, 0, &view());
    let last = shown(&trace, trace.last(), &view());
    // At the start, every forward lane has room and no reverse lane does.
    for e in 0..10 {
        assert!(!first.slots[slots.lane(e, Direction::Forward)].class.contains("ek-off"));
        assert!(first.slots[slots.lane(e, Direction::Reverse)].class.contains("ek-off"));
    }
    // At the end e8 has 1 left, and its flow is 19, so both of its lanes are drawn. e5 has none either way.
    assert!(!last.slots[slots.lane(8, Direction::Forward)].class.contains("ek-off"));
    assert!(!last.slots[slots.lane(8, Direction::Reverse)].class.contains("ek-off"));
    assert!(!last.slots[slots.lane(5, Direction::Forward)].class.contains("ek-off"));
    assert!(last.slots[slots.lane(5, Direction::Reverse)].class.contains("ek-off"));
    assert_eq!(first.slots.len(), last.slots.len());
}

#[test]
fn the_residual_toggle_hides_every_lane_and_nothing_else() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let on = shown(&trace, 40, &view());
    let off = shown(
        &trace,
        40,
        &View {
            prefs: Prefs {
                show_residual: false,
                ..Prefs::default()
            },
            ..view()
        },
    );
    for e in 0..10 {
        for d in [Direction::Forward, Direction::Reverse] {
            assert!(off.slots[slots.lane(e, d)].class.contains("ek-off"));
        }
        assert_eq!(on.slots[slots.edge(e)], off.slots[slots.edge(e)]);
    }
    assert_eq!(on.panel, off.panel, "the text panel does not change");
}

#[test]
fn the_queue_toggle_hides_the_marks_but_the_text_queue_stays() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let at = trace
        .snapshots()
        .iter()
        .position(|s| s.bfs.queue.len() >= 2)
        .expect("a queue of two");
    let on = shown(&trace, at, &view());
    let off = shown(
        &trace,
        at,
        &View {
            prefs: Prefs {
                show_queue: false,
                ..Prefs::default()
            },
            ..view()
        },
    );
    let marked = |p: &Presentation| (0..6).filter(|&v| !p.slots[slots.badge(v)].class.contains("ek-off")).count();
    assert!(marked(&on) >= 2);
    assert_eq!(marked(&off), 0);
    assert_eq!(on.panel[3], off.panel[3], "the text of the queue is the same");
    assert_ne!(on.panel[3], NOTHING);
}

#[test]
fn the_examined_transition_is_marked_only_in_the_detailed_mode() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let at = trace
        .snapshots()
        .iter()
        .position(|s| matches!(s.event, Event::ArcExamined { .. }))
        .expect("one");
    let Event::ArcExamined { hop, .. } = trace.snapshot(at).expect("a snapshot").event else {
        unreachable!()
    };
    let detailed = shown(&trace, at, &view());
    let compact = shown(
        &trace,
        at,
        &View {
            prefs: Prefs {
                detailed: false,
                ..Prefs::default()
            },
            ..view()
        },
    );
    assert!(detailed.slots[slots.edge(hop.edge)].class.contains("ek-examined"));
    assert!(
        detailed.slots[slots.lane(hop.edge, hop.direction)]
            .class
            .contains("ek-examined")
    );
    assert!(!compact.slots[slots.edge(hop.edge)].class.contains("ek-examined"));
    assert!(!compact.slots[slots.lane(hop.edge, hop.direction)].class.contains("ek-examined"));
}

#[test]
fn the_augmenting_path_and_the_changed_edges_are_marked_and_the_label_says_before_and_after() {
    let trace = trace_of(&REVERSE);
    let slots = Slots::new(6, 7);
    let at = trace.augmentation_indices()[1];
    let page = shown(&trace, at, &view());
    assert_eq!(
        page.slots[slots.edge_label(0)].text.as_deref(),
        Some("2\u{2192}1/2"),
        "e0 goes from 2 to 1"
    );
    assert!(page.slots[slots.edge_label(0)].class.contains("ek-aug"));
    assert!(page.slots[slots.edge(0)].class.contains("ek-aug"));
    assert!(
        page.slots[slots.lane(0, Direction::Reverse)].class.contains("ek-path"),
        "the reverse lane is on the path"
    );
    // An edge that the path did not use keeps its plain label.
    assert_eq!(page.slots[slots.edge_label(1)].text.as_deref(), Some("2/2"));
}

#[test]
fn the_cut_shades_s_and_t_and_marks_only_the_edges_that_cross_from_s_to_t() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let page = shown(&trace, trace.last(), &view());
    for v in [0, 1, 2, 4] {
        assert!(page.slots[slots.vertex(v)].class.contains("ek-s"), "v{v} is in S");
    }
    for v in [3, 5] {
        assert!(page.slots[slots.vertex(v)].class.contains("ek-t"), "v{v} is in T");
    }
    let marked: Vec<usize> = (0..10)
        .filter(|&e| page.slots[slots.edge(e)].class.contains("ek-cut"))
        .collect();
    assert_eq!(marked, [4, 7, 9]);
    assert!(
        !page.slots[slots.edge(5)].class.contains("ek-cut"),
        "the edge from T to S is not marked"
    );
    assert!(page.slots[slots.edge(5)].class.contains("ek-dim"));
}

#[test]
fn nothing_is_shaded_before_the_cut() {
    let trace = trace_of(&CANONICAL);
    for index in 0..trace.last() {
        let page = shown(&trace, index, &view());
        for slot in &page.slots {
            let shaded = slot
                .class
                .split_whitespace()
                .any(|c| c == "ek-cut" || c == "ek-s" || c == "ek-t" || c == "ek-dim");
            assert!(!shaded, "step {index}: {}", slot.class);
        }
    }
}

#[test]
fn exactly_one_target_has_tabindex_zero_and_the_focused_one_is_marked() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let order = target_order(&trace);
    assert_eq!(order.len(), 16);
    for &target in &order {
        let v = View {
            roving: target,
            focused: Some(target),
            ..view()
        };
        let page = shown(&trace, 5, &v);
        let zeros = (0..6)
            .map(|v| slots.vertex(v))
            .chain((0..10).map(|e| slots.edge(e)))
            .filter(|&at| page.slots[at].attrs.contains(&("tabindex", "0".to_owned())))
            .count();
        assert_eq!(zeros, 1);
        let at = match target {
            Target::Vertex(v) => slots.vertex(v),
            Target::Edge(e) => slots.edge(e),
        };
        assert!(page.slots[at].class.contains("ek-focused"));
        assert!(page.slots[at].attrs.contains(&("tabindex", "0".to_owned())));
    }
    assert_eq!(target_order(&trace_of(&REVERSE)).len(), 13);
}

#[test]
fn residual_lanes_are_never_a_target() {
    let trace = trace_of(&CANONICAL);
    let slots = Slots::new(6, 10);
    let page = shown(&trace, 40, &view());
    for e in 0..10 {
        for d in [Direction::Forward, Direction::Reverse] {
            assert!(!page.slots[slots.lane(e, d)].attrs.iter().any(|(n, _)| *n == "tabindex"));
        }
    }
}

#[test]
fn the_arrow_keys_go_through_the_targets_in_list_order_and_wrap() {
    let trace = trace_of(&CANONICAL);
    let order = target_order(&trace);
    let first = Target::Vertex(0);
    let last = Target::Edge(9);
    assert_eq!(move_roving(&order, first, "ArrowRight"), Some(Target::Vertex(1)));
    assert_eq!(move_roving(&order, first, "ArrowDown"), Some(Target::Vertex(1)));
    assert_eq!(move_roving(&order, first, "ArrowLeft"), Some(last), "it wraps");
    assert_eq!(move_roving(&order, first, "ArrowUp"), Some(last));
    assert_eq!(move_roving(&order, last, "ArrowRight"), Some(first));
    assert_eq!(
        move_roving(&order, Target::Vertex(5), "ArrowRight"),
        Some(Target::Edge(0)),
        "vertices, then edges"
    );
    assert_eq!(move_roving(&order, Target::Edge(3), "Home"), Some(first));
    assert_eq!(move_roving(&order, Target::Edge(3), "End"), Some(last));
    for key in ["Enter", " ", "PageDown", "a", "Tab"] {
        assert_eq!(move_roving(&order, first, key), None, "{key}");
    }
}

// What a diff writes.

#[test]
fn the_writes_of_a_diff_leave_a_page_the_same_as_a_full_redraw_at_every_step_in_every_mode() {
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        for prefs in all_prefs() {
            let v = View { prefs, ..view() };
            let mut model = shown(&trace, 0, &v);
            // The model of a page that was drawn in full at step 0, and then only diffed.
            for index in 1..=trace.last() {
                let next = shown(&trace, index, &v);
                let writes = diff(&model, &next);
                apply(&mut model, &writes);
                assert_eq!(model, next, "{} {prefs:?} step {index}: a diff", fixture.key);
                let mut fresh = Presentation {
                    slots: vec![Slot::default(); next.slots.len()],
                    panel: vec![String::new(); next.panel.len()],
                };
                apply(&mut fresh, &full(&next));
                assert_eq!(fresh, next, "{} {prefs:?} step {index}: a full redraw", fixture.key);
            }
        }
    }
}

#[test]
fn the_writes_of_a_diff_leave_a_page_the_same_after_random_seeks_and_back() {
    let trace = trace_of(&CANONICAL);
    let v = view();
    let mut model = shown(&trace, 0, &v);
    let mut x = 12_345u64;
    for _ in 0..300 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let index = (x >> 33) as usize % trace.len();
        let next = shown(&trace, index, &v);
        let writes = diff(&model, &next);
        apply(&mut model, &writes);
        assert_eq!(model, next, "after a seek to {index}");
    }
}

#[test]
fn a_full_redraw_over_a_page_that_shows_something_else_leaves_it_the_same_as_the_new_one() {
    // A page that was shown at one step, redrawn in full at another, has nothing left over from the first: a lane that
    // was shown and is hidden now has lost its name. This is what a repair does.
    for fixture in FIXTURES {
        let trace = trace_of(fixture);
        for prefs in all_prefs() {
            let v = View { prefs, ..view() };
            for (from, to) in [(0, trace.last()), (trace.last(), 0), (7, 31), (31, 7)] {
                let from = from.min(trace.last());
                let to = to.min(trace.last());
                let mut model = shown(&trace, from, &v);
                let next = shown(&trace, to, &v);
                apply(&mut model, &full(&next));
                assert_eq!(model, next, "{} {prefs:?} from {from} to {to}", fixture.key);
            }
        }
    }
}

#[test]
fn a_diff_between_equal_presentations_writes_nothing_and_a_step_writes_little() {
    let trace = trace_of(&CANONICAL);
    let same = shown(&trace, 10, &view());
    assert!(diff(&same, &same).is_empty());
    // A step that only looks at another transition changes a handful of things, and not the whole picture.
    let a = shown(&trace, 10, &view());
    let b = shown(&trace, 11, &view());
    let writes = diff(&a, &b).len();
    assert!(writes < 20, "{writes} writes");
    assert!(full(&b).len() > 100 && writes * 5 < full(&b).len());
}

#[test]
fn a_toggle_or_a_focus_change_writes_only_what_it_changes() {
    let trace = trace_of(&CANONICAL);
    let base = view();
    let a = shown(&trace, 30, &base);
    let focus = shown(
        &trace,
        30,
        &View {
            focused: Some(Target::Edge(4)),
            roving: Target::Edge(4),
            ..base
        },
    );
    let writes = diff(&a, &focus);
    assert!(!writes.is_empty() && writes.len() <= 6, "{writes:?}");
    assert!(
        writes.iter().all(|w| !matches!(w, Write::Panel(..))),
        "the panel does not change with the focus"
    );
}

// The buttons that are at an end.

#[test]
fn the_buttons_at_the_first_step_are_the_three_that_have_nowhere_to_go() {
    let trace = trace_of(&CANONICAL);
    let b = bounds(&trace, 0);
    assert!(b.restart && b.previous && b.previous_augmentation);
    assert!(!b.next && !b.play && !b.next_augmentation && !b.jump_to_cut);
}

#[test]
fn the_buttons_at_the_last_step_are_the_ones_that_go_forward() {
    let trace = trace_of(&CANONICAL);
    let b = bounds(&trace, trace.last());
    assert!(b.next && b.play && b.next_augmentation && b.jump_to_cut);
    assert!(!b.restart && !b.previous && !b.previous_augmentation);
}

#[test]
fn the_augmentation_buttons_follow_the_augmentations_strictly() {
    let trace = trace_of(&CANONICAL);
    let at = trace.augmentation_indices().to_vec();
    assert!(bounds(&trace, at[0]).previous_augmentation, "none before the first");
    assert!(!bounds(&trace, at[1]).previous_augmentation && !bounds(&trace, at[1]).next_augmentation);
    assert!(bounds(&trace, at[2]).next_augmentation, "no augmentation after the last");
    assert!(!bounds(&trace, at[2]).previous_augmentation);
}

// What is said.

#[test]
fn only_the_path_the_augmentation_and_the_cut_are_announced_while_playing() {
    let trace = trace_of(&CANONICAL);
    let announced: Vec<&str> = trace
        .snapshots()
        .iter()
        .filter(|s| announced_while_playing(&s.event))
        .map(|s| s.event.name())
        .collect();
    assert_eq!(announced.iter().filter(|&&n| n == "PathFound").count(), 3);
    assert_eq!(announced.iter().filter(|&&n| n == "FlowAugmented").count(), 3);
    assert_eq!(announced.iter().filter(|&&n| n == "MinCutShown").count(), 1);
    assert_eq!(announced.len(), 7);
}

#[test]
fn the_summary_gives_the_step_and_the_phase() {
    let trace = trace_of(&CANONICAL);
    assert_eq!(
        summary(&trace, trace.snapshot(5).expect("a snapshot")),
        format!("Step 5 of 86: {}", trace.snapshot(5).expect("a snapshot").event.name())
    );
    assert!(pause_summary(&trace, trace.snapshot(5).expect("a snapshot")).starts_with("Paused at step 5 of 86"));
}

#[test]
fn the_speeds_are_the_four_pinned_intervals_and_the_default_is_the_third() {
    assert_eq!(Speed::ALL.map(Speed::interval_ms), [800, 400, 200, 100]);
    assert_eq!(Speed::ALL.map(Speed::key), ["0.5", "1", "2", "4"]);
    for speed in Speed::ALL {
        assert_eq!(Speed::from_key(speed.key()), Some(speed));
    }
    assert_eq!(Speed::from_key("3"), None);
    // The canonical run at the default speed takes about 17 seconds.
    assert_eq!(86 * Speed::Two.interval_ms(), 17_200);
}

#[test]
fn the_preferences_toggle_one_at_a_time() {
    let p = Prefs::default();
    assert!(p.show_residual && p.show_queue && p.detailed);
    for pref in [Pref::Residual, Pref::Queue, Pref::Detail] {
        let once = p.toggled(pref);
        assert_ne!(once.get(pref), p.get(pref));
        assert_eq!(once.toggled(pref), p);
        for other in [Pref::Residual, Pref::Queue, Pref::Detail] {
            if other != pref {
                assert_eq!(once.get(other), p.get(other));
            }
        }
    }
}
