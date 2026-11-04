use super::*;
use crate::petri_net::{net::TransitionId as T, trace::Branch};

fn net() -> Net {
    Net::bounded_buffer()
}

fn trace(s: Scenario) -> Trace {
    s.trace(&net()).unwrap()
}

fn at(trace: &Trace, index: usize, source: Source, view: &View) -> Presentation {
    present(&net(), trace, index, source, view).unwrap()
}

fn attr<'a>(p: &'a Presentation, slot: usize, name: &str) -> &'a str {
    &p.slots[slot].attrs.iter().find(|(n, _)| *n == name).unwrap().1
}

const WALK: Source = Source::Script(Scenario::Walkthrough);

#[test]
fn a_presentation_has_a_slot_for_everything_and_a_field_for_each_panel_row() {
    let p = at(&trace(Scenario::Walkthrough), 0, WALK, &View::first());
    assert_eq!(p.slots.len(), SLOTS);
    assert_eq!(SLOTS, 20 + 18 + 50 + 8 + 1);
    assert_eq!(p.panel.len(), PANEL.len());
    assert!(p.panel.iter().all(|f| !f.is_empty()));
}

#[test]
fn there_is_no_presentation_beyond_the_end() {
    let t = trace(Scenario::Contention);
    assert!(present(&net(), &t, t.last() + 1, Source::Script(Scenario::Contention), &View::first()).is_none());
}

#[test]
fn the_start_enables_four_starts_and_blocks_three_consumers() {
    let p = at(&trace(Scenario::Walkthrough), 0, WALK, &View::first());
    assert_eq!(p.panel[5], "Start[0], Start[1], Start[2] and Start[3]");
    assert_eq!(
        p.panel[6],
        "C0 waits: buffer empty; C1 waits: buffer empty; C2 waits: buffer empty."
    );
    assert_eq!(p.panel[2], "Initial marking");
    assert_eq!(p.panel[4], "None yet.");
    for i in 0..4 {
        assert_eq!(attr(&p, gate_slot(T::start(i)), "data-pn-status"), "enabled");
        assert_eq!(attr(&p, gate_slot(T::finish(i)), "data-pn-status"), "disabled");
    }
}

#[test]
fn a_gate_says_enabled_disabled_new_or_fired_in_words_and_a_class() {
    let t = trace(Scenario::Contention);
    let p = at(&t, 26, Source::Script(Scenario::Contention), &View::first());
    let put0 = gate_slot(T::put(0));
    assert_eq!(attr(&p, put0, "data-pn-status"), "fired");
    assert!(attr(&p, put0, "class").contains("pn-fired"));
    assert!(attr(&p, put0, "aria-label").contains("Just fired"));
    let put1 = gate_slot(T::put(1));
    assert_eq!(attr(&p, put1, "data-pn-status"), "disabled");
    assert!(attr(&p, put1, "class").contains("pn-disabled"));
    assert!(attr(&p, put1, "aria-label").contains("Missing Free"));
    let p = at(&t, 27, Source::Script(Scenario::Contention), &View::first());
    assert_eq!(attr(&p, put1, "data-pn-status"), "new");
    assert!(attr(&p, put1, "class").contains("pn-new") && attr(&p, put1, "class").contains("pn-enabled"));
    assert!(attr(&p, put1, "aria-label").contains("Newly enabled"));
}

#[test]
fn newly_enabled_and_disabled_are_said_for_each_event() {
    let t = trace(Scenario::Contention);
    let source = Source::Script(Scenario::Contention);
    let p = at(&t, 26, source, &View::first());
    assert_eq!(p.panel[4], "Newly enabled: Start[0]. Newly disabled: Put[0] and Put[1].");
    assert!(p.status.contains("Put[0] and Put[1] became disabled"), "{}", p.status);
    let p = at(&t, 27, source, &View::first());
    assert!(
        p.panel[4].starts_with("Newly enabled: Put[1] and Complete[0]"),
        "{}",
        p.panel[4]
    );
    assert_eq!(
        p.status,
        "Take[0] consumed C0.Idle and Used; produced C0.Processing and Free; Put[1] and Complete[0] became enabled; \
         Take[0] became disabled."
    );
}

#[test]
fn a_blocked_producer_and_a_blocked_consumer_say_why() {
    let t = trace(Scenario::Walkthrough);
    let p = at(&t, 26, WALK, &View::first());
    let ready = place_slot(PlaceId::producer_ready(0));
    assert!(attr(&p, ready, "aria-label").ends_with("Waiting: buffer full"));
    assert!(attr(&p, ready, "class").contains("pn-waiting"));
    assert_eq!(p.panel[6], "P0 waits: buffer full.");
    let p = at(&t, 45, WALK, &View::first());
    let idle = place_slot(PlaceId::consumer_idle(1));
    assert!(attr(&p, idle, "aria-label").ends_with("Waiting: buffer empty"));
}

#[test]
fn the_hubs_show_a_count_and_the_actor_places_a_class() {
    let t = trace(Scenario::Walkthrough);
    let p = at(&t, 24, WALK, &View::first());
    assert_eq!(p.slots[place_slot(PlaceId::USED)].text.as_deref(), Some("8"));
    assert_eq!(p.slots[place_slot(PlaceId::FREE)].text.as_deref(), Some("0"));
    assert_eq!(attr(&p, place_slot(PlaceId::USED), "aria-label"), "Used: 8 of 8 slots");
    assert_eq!(p.slots[place_slot(PlaceId::producer_idle(0))].text, None);
    assert!(attr(&p, place_slot(PlaceId::producer_idle(0)), "class").contains("pn-tok-1"));
    assert!(attr(&p, place_slot(PlaceId::producer_busy(0)), "class").contains("pn-tok-0"));
}

#[test]
fn the_buffer_picture_fills_one_cell_for_each_used_slot_and_always_adds_up() {
    let t = trace(Scenario::Walkthrough);
    for (index, used) in [(0, 0), (3, 1), (24, 8), (27, 7)] {
        let p = at(&t, index, WALK, &View::first());
        let full = (0..8).filter(|&c| attr(&p, cell_slot(c), "data-pn-full") == "true").count();
        assert_eq!(full, used);
        assert_eq!(p.slots[CAPTION_SLOT].text, Some(format!("Used {used} of 8, Free {}", 8 - used)));
    }
}

#[test]
fn the_wiring_emphasis_changes_only_arcs_and_only_in_focused_wiring() {
    let t = trace(Scenario::Walkthrough);
    let mut view = View::first();
    view.selected = Some(Target::Transition(T::put(2)));
    let overview = at(&t, 5, WALK, &view);
    assert!((0..ARCS).all(|a| attr(&overview, arc_slot(a), "data-pn-emphasis") == "normal"));
    view.wiring = Wiring::Focused;
    let focused = at(&t, 5, WALK, &view);
    let arcs = net().arcs();
    let on: Vec<usize> = (0..ARCS)
        .filter(|&a| attr(&focused, arc_slot(a), "data-pn-emphasis") == "on")
        .collect();
    assert_eq!(on.len(), 4, "a Put has four arcs, and all of them are put forward");
    assert!(on.iter().all(|&a| arcs[a].transition == T::put(2)));
    assert!((0..ARCS).all(|a| on.contains(&a) || attr(&focused, arc_slot(a), "data-pn-emphasis") == "dim"));
    // The marking, the panel and every other slot are the same.
    assert_eq!(overview.panel[..9], focused.panel[..9]);
    for slot in 0..PLACES + TRANSITIONS {
        let strip = |p: &Presentation| {
            p.slots[slot]
                .attrs
                .iter()
                .filter(|(n, _)| *n != "data-pn-selected" && *n != "aria-pressed")
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(strip(&overview), strip(&focused));
    }
}

#[test]
fn nothing_selected_means_no_emphasis_even_in_focused_wiring() {
    let mut view = View::first();
    view.wiring = Wiring::Focused;
    let p = at(&trace(Scenario::Walkthrough), 0, WALK, &view);
    assert!((0..ARCS).all(|a| attr(&p, arc_slot(a), "data-pn-emphasis") == "normal"));
}

#[test]
fn the_inspector_names_the_missing_input_of_a_disabled_transition() {
    let t = trace(Scenario::Walkthrough);
    let mut view = View::first();
    view.selected = Some(Target::Transition(T::put(0)));
    let p = at(&t, 26, WALK, &view);
    assert_eq!(
        p.panel[9],
        "Put[0]: inputs P0.Ready and Free; outputs P0.Idle and Used. Disabled: Free has no token."
    );
    let p = at(&t, 27, WALK, &view);
    assert!(p.panel[9].ends_with("Enabled: every input holds a token."));
}

#[test]
fn the_inspector_describes_a_place() {
    let mut view = View::first();
    view.selected = Some(Target::Place(PlaceId::FREE));
    let p = at(&trace(Scenario::Walkthrough), 24, WALK, &view);
    assert!(
        p.panel[9].starts_with("Free: 0 tokens. Consumed by Put[0], Put[1], Put[2] and Put[3]; produced by Take[0]")
    );
}

#[test]
fn the_counts_and_the_snapshot_number_come_from_the_snapshot() {
    let t = trace(Scenario::Walkthrough);
    let p = at(&t, 50, WALK, &View::first());
    assert_eq!(p.panel[0], "50 of 50");
    assert_eq!(p.panel[8], "Puts 10, Takes 10, Completes 10");
    assert!(p.panel[1].starts_with("Walkthrough"));
}

#[test]
fn a_manual_branch_says_where_it_came_from_and_starts_without_deltas() {
    let t = Branch::fork(trace(Scenario::Walkthrough).snapshot(26).unwrap()).trace().clone();
    let p = at(
        &t,
        0,
        Source::Manual {
            origin: Scenario::Walkthrough,
            fork: 26,
        },
        &View::first(),
    );
    assert!(
        p.panel[1].starts_with("Manual branch: forked from Walkthrough"),
        "{}",
        p.panel[1]
    );
    assert!(p.panel[1].contains("snapshot 26"));
    assert_eq!(p.panel[4], "None yet.");
    assert_eq!(p.panel[0], "0 of 0");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn apply(mut page: Presentation, writes: &[Write]) -> Presentation {
    for write in writes {
        match write {
            Write::Attr(at, name, value) => {
                let attrs = &mut page.slots[*at].attrs;
                match attrs.iter_mut().find(|(n, _)| n == name) {
                    Some(entry) => entry.1 = value.clone(),
                    None => attrs.push((name, value.clone())),
                }
            },
            Write::RemoveAttr(at, name) => page.slots[*at].attrs.retain(|(n, _)| n != name),
            Write::Text(at, text) => page.slots[*at].text = Some(text.clone()),
            Write::Panel(at, text) => page.panel[*at] = text.clone(),
        }
    }
    page
}

/// A page that has nothing on it, to draw a first presentation on.
fn blank() -> Presentation {
    Presentation {
        slots: vec![SlotState::default(); SLOTS],
        panel: vec![String::new(); PANEL.len()],
        status: String::new(),
    }
}

fn normal(mut p: Presentation) -> Presentation {
    for slot in &mut p.slots {
        slot.attrs.sort();
    }
    p.status.clear();
    p
}

#[test]
fn a_diff_gives_the_same_page_as_a_full_draw_for_every_pair_of_snapshots_of_every_script() {
    for scenario in Scenario::ALL {
        let t = trace(scenario);
        let source = Source::Script(scenario);
        let mut view = View::first();
        for (from, to) in (0..=t.last()).flat_map(|a| {
            [
                (a, (a * 7 + 3) % (t.last() + 1)),
                (a, a.saturating_sub(1)),
                (a, (a + 1).min(t.last())),
            ]
        }) {
            view.selected = Some(Target::Transition(T::take(to % 3)));
            let before = at(&t, from, source, &view);
            let after = at(&t, to, source, &view);
            let drawn_whole = apply(blank(), &full(&after));
            let patched = apply(apply(blank(), &full(&before)), &diff(&before, &after));
            assert_eq!(normal(patched), normal(drawn_whole), "{scenario:?} {from} to {to}");
        }
    }
}

#[test]
fn a_diff_between_equal_presentations_writes_nothing() {
    let t = trace(Scenario::Walkthrough);
    let p = at(&t, 12, WALK, &View::first());
    assert!(diff(&p, &p).is_empty());
}

#[test]
fn a_diff_between_neighbours_writes_few_things() {
    let t = trace(Scenario::Walkthrough);
    let (a, b) = (at(&t, 12, WALK, &View::first()), at(&t, 13, WALK, &View::first()));
    assert!(diff(&a, &b).len() < full(&b).len() / 4);
}

#[test]
fn a_change_of_selection_or_wiring_diffs_to_the_view_alone() {
    let t = trace(Scenario::Walkthrough);
    let before = at(&t, 12, WALK, &View::first());
    let mut view = View::first();
    view.selected = Some(Target::Place(PlaceId::USED));
    let after = at(&t, 12, WALK, &view);
    let writes = diff(&before, &after);
    assert!(
        writes
            .iter()
            .all(|w| matches!(w, Write::Attr(_, "data-pn-selected" | "aria-pressed", _) | Write::Panel(9, _))),
        "{writes:?}"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_keyboard_visits_each_target_once_in_ring_order() {
    let order = target_order();
    assert_eq!(order.len(), 38);
    let mut unique = order.clone();
    unique.dedup();
    assert_eq!(unique.len(), 38);
    assert_eq!(order[0], Target::Place(PlaceId::producer_idle(0)));
    assert_eq!(order[1], Target::Transition(T::start(0)));
    assert_eq!(order[24], Target::Place(PlaceId::FREE));
    assert_eq!(order[37], Target::Transition(T::complete(2)));
}

#[test]
fn the_arrows_home_and_end_move_the_roving_target_and_do_not_wrap() {
    let order = target_order();
    let first = order[0];
    assert_eq!(move_roving(&order, first, "ArrowRight"), Some(order[1]));
    assert_eq!(move_roving(&order, first, "ArrowLeft"), None);
    assert_eq!(move_roving(&order, first, "End"), Some(order[37]));
    assert_eq!(move_roving(&order, order[37], "ArrowDown"), None);
    assert_eq!(move_roving(&order, order[5], "Home"), Some(first));
    assert_eq!(move_roving(&order, first, "Home"), None);
    assert_eq!(move_roving(&order, first, "x"), None);
}

#[test]
fn exactly_one_target_is_a_tab_stop() {
    let t = trace(Scenario::Walkthrough);
    let p = at(&t, 3, WALK, &View::first());
    let stops = (0..PLACES + TRANSITIONS).filter(|&s| attr(&p, s, "tabindex") == "0").count();
    assert_eq!(stops, 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_bounds_dim_what_has_nowhere_to_go() {
    let t = trace(Scenario::Walkthrough);
    let b = |i: usize, source: Source, view: &View| bounds(&net(), &t, i, source, view);
    let first = b(0, WALK, &View::first());
    assert!(first.restart && first.previous && !first.next && !first.play && first.fire && !first.checkpoints);
    let last = b(50, WALK, &View::first());
    assert!(last.next && last.play && !last.previous);
}

#[test]
fn fire_is_available_only_in_manual_mode_with_an_enabled_transition_selected() {
    let t = trace(Scenario::Walkthrough);
    let manual = Source::Manual {
        origin: Scenario::Walkthrough,
        fork: 0,
    };
    let mut view = View::first();
    assert!(bounds(&net(), &t, 0, manual, &view).fire, "nothing selected");
    view.selected = Some(Target::Transition(T::start(0)));
    assert!(!bounds(&net(), &t, 0, manual, &view).fire);
    assert!(bounds(&net(), &t, 0, WALK, &view).fire, "a script cannot fire");
    view.selected = Some(Target::Transition(T::put(0)));
    assert!(bounds(&net(), &t, 0, manual, &view).fire, "a disabled transition");
    view.selected = Some(Target::Place(PlaceId::FREE));
    assert!(bounds(&net(), &t, 0, manual, &view).fire, "a place");
    let b = bounds(&net(), &t, 0, manual, &view);
    assert!(b.play && b.checkpoints);
}

#[test]
fn only_a_change_of_who_waits_or_the_end_is_said_during_playback() {
    let t = trace(Scenario::Walkthrough);
    assert!(!announced_while_playing(&t, 0));
    assert!(!announced_while_playing(&t, 1), "a producer starting changes no wait");
    assert!(announced_while_playing(&t, 3), "the first Put releases all three consumers");
    assert!(announced_while_playing(&t, 26), "a producer is blocked");
    assert!(announced_while_playing(&t, 27), "and released");
    assert!(announced_while_playing(&t, 50), "the end");
}

#[test]
fn the_spoken_summary_names_the_event_and_the_buffer() {
    let t = trace(Scenario::Contention);
    assert_eq!(
        summary(&net(), &t, 27),
        "Take[0] consumed C0.Idle and Used; produced C0.Processing and Free; Put[1] and Complete[0] became enabled; \
         Take[0] became disabled. Used 7 of 8, Free 1."
    );
    assert_eq!(summary(&net(), &t, 0), "Initial marking. Used 0 of 8, Free 8.");
    assert_eq!(summary(&net(), &t, 99), "");
    assert_eq!(
        pause_summary(&net(), &t, 26),
        "Paused at snapshot 26 of 28. Used 8 of 8, Free 0. 1 workers wait on the buffer."
    );
}

#[test]
fn a_disabled_attempt_names_the_missing_places_and_says_nothing_changed() {
    let t = trace(Scenario::Walkthrough);
    let snapshot = t.snapshot(26).unwrap();
    assert_eq!(
        disabled_sentence(&net(), snapshot, T::put(0)),
        "Put[0] is not enabled: Free has no token. Nothing changed."
    );
}

#[test]
fn the_speeds_have_keys_labels_and_intervals() {
    for speed in Speed::ALL {
        assert_eq!(Speed::from_key(speed.key()), Some(speed));
    }
    assert_eq!(Speed::from_key("3"), None);
    assert_eq!(Speed::Two.interval_ms(), 500);
    assert!(Speed::ALL.windows(2).all(|w| w[0].interval_ms() > w[1].interval_ms()));
    assert_eq!(Speed::Ten.label(), "10 events/s");
}
