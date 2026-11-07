use super::*;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Config, Engine, Limits, ScriptEvent};

const FIXTURES: &str = include_str!("../../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn engine_of(scenario: &Scenario) -> Engine {
    Engine::new(
        &scenario.config_id,
        scenario.seed,
        scenario.config,
        &scenario.script,
        scenario.params.as_ref(),
        Limits::default(),
    )
    .expect("an engine")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The model before the first slice, and after each slice, extracted for `view`.
fn models(scenario: &Scenario, view: &View) -> Vec<Model> {
    let mut engine = engine_of(scenario);
    let mut all = vec![extract(&engine, view)];
    while engine.step().expect("a slice").is_some() {
        all.push(extract(&engine, view));
    }
    all
}

fn fixture(name: &str) -> Scenario {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    Scenario::fixture(name, &fixtures[name]).expect("a fixture")
}

fn text_of<'a>(p: &'a Projection, key: &str) -> &'a str {
    &p.slots
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("no slot {key}"))
        .text
}

fn slot_of<'a>(p: &'a Projection, key: &str) -> &'a Slot {
    p.slots.iter().find(|s| s.key == key).unwrap_or_else(|| panic!("no slot {key}"))
}

/// Applies a delta to a copy of `old`, writing from `new` as the surface does.
fn apply(old: &Projection, new: &Projection, writes: &[Write]) -> Projection {
    let mut out = old.clone();
    for w in writes {
        let (to, from) = (&mut out.slots[w.slot], &new.slots[w.slot]);
        match w.field {
            Field::Text => to.text.clone_from(&from.text),
            Field::Class => to.class.clone_from(&from.class),
            Field::Label => to.label.clone_from(&from.label),
            Field::Hidden => to.hidden = from.hidden,
        }
    }
    out.data = new.data;
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One row of the E0 ledger: what a reader sees after a slice. It is written out from the table in section 4 of the design, tick by
/// tick, and not taken from the engine's output.
struct Row {
    tick: u64,
    servers: [&'static str; 2],
    /// The waiting window, head first, as the slots say it.
    waiting: &'static [&'static str],
    /// Arrived, admitted, rejected, completed, timed out, cancelled, failed, waiting to retry, stale pops, no-op pops.
    counters: [u64; 10],
}

#[rustfmt::skip]
const E0: [Row; 13] = [
    Row { tick: 0,  servers: ["idle", "idle"], waiting: &[],                              counters: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    Row { tick: 0,  servers: ["J0",   "J1"],   waiting: &[],                              counters: [2, 2, 0, 0, 0, 0, 0, 0, 0, 0] },
    Row { tick: 1,  servers: ["J0",   "J1"],   waiting: &["J2 H"],                        counters: [3, 3, 0, 0, 0, 0, 0, 0, 0, 0] },
    Row { tick: 2,  servers: ["down", "J1"],   waiting: &["J2 H", "J3 L"],               counters: [4, 4, 0, 0, 0, 0, 0, 1, 0, 0] },
    Row { tick: 4,  servers: ["down", "J1"],   waiting: &["J2 H", "J0 H", "J3 L"],       counters: [5, 4, 1, 0, 0, 0, 0, 0, 1, 0] },
    Row { tick: 5,  servers: ["down", "J1"],   waiting: &["J2 H", "J0 H", "J5 L"],       counters: [6, 5, 1, 0, 1, 0, 0, 0, 1, 0] },
    Row { tick: 6,  servers: ["down", "J2"],   waiting: &["J0 H"],                        counters: [6, 5, 1, 1, 1, 1, 0, 0, 1, 0] },
    Row { tick: 7,  servers: ["J0",   "J2"],   waiting: &[],                              counters: [6, 5, 1, 1, 1, 1, 0, 0, 1, 0] },
    Row { tick: 8,  servers: ["J0",   "idle"], waiting: &[],                              counters: [6, 5, 1, 2, 1, 1, 0, 0, 2, 0] },
    Row { tick: 9,  servers: ["idle", "idle"], waiting: &[],                              counters: [6, 5, 1, 3, 1, 1, 0, 0, 2, 0] },
    Row { tick: 10, servers: ["idle", "idle"], waiting: &[],                              counters: [6, 5, 1, 3, 1, 1, 0, 0, 3, 0] },
    Row { tick: 15, servers: ["idle", "idle"], waiting: &[],                              counters: [6, 5, 1, 3, 1, 1, 0, 0, 4, 0] },
    Row { tick: 20, servers: ["idle", "idle"], waiting: &[],                              counters: [6, 5, 1, 3, 1, 1, 0, 0, 5, 0] },
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// E0 projects exactly: after each of its 12 slices, and before the first, every slot says what the hand-audited ledger says.
#[test]
fn e0_projects_exactly_what_the_ledger_says() {
    let all = models(&fixture("E0"), &View::default());
    assert_eq!(all.len(), E0.len(), "13 snapshots including index 0");
    for (k, (model, row)) in all.iter().zip(&E0).enumerate() {
        let p = project(model, &View::default());
        assert_eq!(text_of(&p, "clock"), format!("tick {} · slice {k}", row.tick), "clock at {k}");
        for (i, want) in row.servers.iter().enumerate() {
            assert_eq!(text_of(&p, &format!("S{i}")), *want, "server S{i} at {k}");
        }
        for position in 0..QUEUE_SLOTS {
            let slot = slot_of(&p, &format!("Q{position}"));
            match row.waiting.get(position) {
                Some(want) => assert_eq!((slot.text.as_str(), slot.hidden), (*want, false), "Q{position} at {k}"),
                None => assert_eq!((slot.text.as_str(), slot.hidden), ("", true), "Q{position} at {k}"),
            }
        }
        assert_eq!(text_of(&p, "queue"), format!("{}/3", row.waiting.len()), "queue count at {k}");
        assert_eq!(text_of(&p, "more"), "", "no overflow at {k}");
        let counters: Vec<String> = COUNTER_NAMES.iter().map(|(key, _)| text_of(&p, key).to_owned()).collect();
        let want: Vec<String> = row.counters.iter().map(ToString::to_string).collect();
        assert_eq!(counters, want, "counters at {k}");
        assert_eq!(
            p.data[0],
            u32::try_from(k).expect("small"),
            "the data node's first cell is the slice ordinal at {k}"
        );
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An empty queue slot is blank and hidden, and a filled one is neither. A busy server says its job, attempt and due tick.
#[test]
fn empty_slots_are_hidden_and_a_busy_server_names_its_job() {
    let all = models(&fixture("E0"), &View::default());
    let p = project(&all[1], &View::default());
    assert_eq!(slot_of(&p, "S0").label, "Server S0: running J0, attempt 1, due tick 4");
    assert_eq!(slot_of(&p, "S1").label, "Server S1: running J1, attempt 1, due tick 6");
    assert!(
        slot_of(&p, "Q0").hidden && slot_of(&p, "Q0").label.is_empty(),
        "an empty slot is hidden with no name"
    );
    let p = project(&all[4], &View::default());
    assert_eq!(slot_of(&p, "Q1").label, "Position 2: J0, high priority");
    assert_eq!(slot_of(&p, "S0").label, "Server S0: down");
    let p = project(&all[7], &View::default());
    assert_eq!(
        slot_of(&p, "S0").label,
        "Server S0: running J0, attempt 2, due tick 9",
        "J0's second attempt"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A delta from any snapshot to any other, forwards or backwards, reaches what a full redraw of the other writes. This is a seek.
#[test]
fn a_delta_between_any_two_snapshots_equals_a_full_redraw() {
    let projections: Vec<Projection> = models(&fixture("E0"), &View::default())
        .iter()
        .map(|m| project(m, &View::default()))
        .collect();
    for (from, old) in projections.iter().enumerate() {
        for (to, new) in projections.iter().enumerate() {
            assert!(old.same_shape(new));
            let delta = writes(Some(old), new);
            assert_eq!(apply(old, new, &delta), *new, "delta {from} to {to}");
            assert_eq!(apply(old, new, &delta).canonical(), new.canonical());
            let full = writes(None, new);
            assert_eq!(
                full.len(),
                new.slots.len() * 4,
                "a full redraw writes every field of every slot"
            );
            assert_eq!(
                apply(
                    &Projection {
                        slots: old.slots.clone(),
                        data: [0; DATA_CELLS]
                    },
                    new,
                    &full
                ),
                *new
            );
            assert!(delta.len() <= full.len());
            if from == to {
                assert!(delta.is_empty(), "no change, no writes");
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A slice that only pops stale events still changes the clock, the stale counter and the data node's ordinal, and nothing else.
#[test]
fn a_stale_only_slice_writes_only_the_clock_and_the_stale_counter() {
    let all = models(&fixture("E0"), &View::default());
    let (before, after) = (project(&all[9], &View::default()), project(&all[10], &View::default()));
    let delta = writes(Some(&before), &after);
    let keys: Vec<(&str, Field)> = delta.iter().map(|w| (after.slots[w.slot].key.as_str(), w.field)).collect();
    assert_eq!(
        keys,
        [
            ("clock", Field::Text),
            ("clock", Field::Label),
            ("stale", Field::Text),
            ("stale", Field::Label)
        ]
    );
    assert_ne!(
        before.data, after.data,
        "the setter has something to write on a stale-only slice"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The selected job is highlighted wherever it is, and the detail line follows it. An id that is not a job selects nothing.
#[test]
fn selection_highlights_the_job_wherever_it_is() {
    let scenario = fixture("E0");
    let j2 = View { selected: Some(2) };
    let (plain_all, j2_all) = (models(&scenario, &View::default()), models(&scenario, &j2));
    let p = project(&j2_all[4], &j2);
    assert!(
        slot_of(&p, "Q0").class.ends_with("ss-selected"),
        "J2 is at the head of the queue"
    );
    assert!(!slot_of(&p, "Q1").class.contains("ss-selected"));
    assert_eq!(text_of(&p, "selected"), "J2: queued");
    let p = project(&j2_all[6], &j2);
    assert!(slot_of(&p, "S1").class.ends_with("ss-selected"), "J2 is running on S1");
    assert_eq!(text_of(&p, "selected"), "J2: running");
    assert_eq!(
        slot_of(&p, "selected").label,
        "Selected job J2, high priority, running, attempt 1"
    );
    let p = project(&j2_all[12], &j2);
    assert_eq!(
        text_of(&p, "selected"),
        "J2: completed",
        "a job that is finished is still held, since it was selected"
    );
    let nowhere = View { selected: Some(77) };
    let p = project(&models(&scenario, &nowhere)[3], &nowhere);
    assert_eq!(text_of(&p, "selected"), "none", "an id that is not a job selects nothing");
    // Changing only the selection writes only the slots that it touches.
    let (plain, chosen) = (project(&plain_all[4], &View::default()), project(&j2_all[4], &j2));
    let touched: Vec<&str> = writes(Some(&plain), &chosen)
        .iter()
        .map(|w| chosen.slots[w.slot].key.as_str())
        .collect();
    assert!(touched.iter().all(|k| *k == "Q0" || *k == "selected"), "{touched:?}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// More than twelve waiting jobs show the head of the queue and say how many more there are.
#[test]
fn a_long_queue_shows_the_head_and_says_how_many_more() {
    let config = Config {
        servers: 1,
        queue_capacity: 30,
        max_attempts: 1,
        deferral_cap: 4,
    };
    let script: Vec<ScriptEvent> = (0..20)
        .map(|i| ScriptEvent::Arrive {
            at: 0,
            key: format!("J{i}"),
            priority: 0,
            durations: vec![100],
            deadline: 500,
        })
        .collect();
    let mut engine = Engine::new("long", 0, config, &script, None, Limits::default()).expect("an engine");
    engine.step().expect("a slice");
    let p = project(&extract(&engine, &View::default()), &View::default());
    assert_eq!(text_of(&p, "queue"), "19/30");
    assert_eq!(text_of(&p, "more"), "+7 more");
    assert_eq!(slot_of(&p, "more").label, "7 more waiting jobs");
    assert!(
        (0..QUEUE_SLOTS).all(|i| !slot_of(&p, &format!("Q{i}")).hidden),
        "all twelve slots are filled"
    );
    assert_eq!(text_of(&p, "Q0"), "J1 L", "the head is the first to wait");
    assert_eq!(text_of(&p, "Q11"), "J12 L");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Over whole generated runs the slots never change shape, no key repeats, and every delta lands on the full redraw.
#[test]
fn generated_runs_keep_one_shape_and_every_delta_lands_on_the_full_redraw() {
    for seed in [1, 2, 3, 5, 8, 13] {
        for scenario in [Scenario::panel(seed).expect("panel"), Scenario::panel2(seed).expect("panel2")] {
            let servers = scenario.config.servers;
            let projections: Vec<Projection> = models(&scenario, &View::default())
                .iter()
                .map(|m| project(m, &View::default()))
                .collect();
            let keys = Projection::keys(servers);
            let mut previous: Option<&Projection> = None;
            for p in &projections {
                assert_eq!(p.slots.iter().map(|s| s.key.clone()).collect::<Vec<_>>(), keys);
                let mut sorted = keys.clone();
                sorted.sort();
                sorted.dedup();
                assert_eq!(sorted.len(), keys.len(), "no key repeats");
                if let Some(old) = previous {
                    assert_eq!(apply(old, p, &writes(Some(old), p)), *p, "seed {seed}");
                }
                for slot in &p.slots {
                    if slot.key.starts_with('Q') && slot.text.is_empty() {
                        assert!(slot.hidden && slot.label.is_empty(), "an empty queue slot is hidden");
                    }
                }
                previous = Some(p);
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same model and view give the same projection and the same hash, and a change of one field changes the hash.
#[test]
fn the_projection_is_pure_and_its_hash_notices_one_field() {
    let all = models(&fixture("E0"), &View::default());
    let (a, b) = (project(&all[5], &View::default()), project(&all[5], &View::default()));
    assert_eq!((a.canonical(), a.sha256()), (b.canonical(), b.sha256()));
    assert_eq!(a.sha256().len(), 64);
    for field in [Field::Text, Field::Class, Field::Label, Field::Hidden] {
        let mut c = a.clone();
        match field {
            Field::Text => c.slots[3].text.push('x'),
            Field::Class => c.slots[3].class.push('x'),
            Field::Label => c.slots[3].label.push('x'),
            Field::Hidden => c.slots[3].hidden = !c.slots[3].hidden,
        }
        assert_ne!(c.sha256(), a.sha256(), "{field:?}");
    }
    let mut d = a.clone();
    d.data[7] += 1;
    assert_ne!(d.sha256(), a.sha256(), "the data node is part of the hash");
    // A tab or a newline in a field cannot make two different slots look alike.
    assert_ne!(
        canonical_line("k", "a\tb", "c", "d", false),
        canonical_line("k", "a", "b\tc", "d", false)
    );
    assert_ne!(
        canonical_line("k", "a\nb", "c", "d", false),
        canonical_line("k\na", "b", "c", "d", false)
    );
}
