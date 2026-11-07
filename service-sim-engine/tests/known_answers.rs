//! The engine against the design's known answers, the S0 pins, the golden hashes that the oracle agreed on, and its own invariants.
//!
//! These run natively and for `wasm32-wasip1`, so the 32-bit build is held to the same hashes as the 64-bit one.

use serde_json::Value;
use service_sim_engine::trace::{Scenario, trace};
use service_sim_engine::{Config, Engine, Error, Limits, Params, ScriptEvent};

const FIXTURES: &str = include_str!("../../dess/s1/fixtures.json");
const GOLDEN: &str = include_str!("../../dess/s1/golden.json");
const ERRORS: &str = include_str!("../../dess/s1/error-fixtures.json");
const FROZEN: &str = include_str!("../../dess/s0/prototype/frozen.json");

fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("the file is JSON")
}

fn run_trace(s: &Scenario) -> Value {
    trace(s, Limits::default()).unwrap_or_else(|e| panic!("{}: {e}", s.config_id))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// E0, E1-RC, E2-RT, E3-NOOP and E3-CT give the counts and terminal states that the design (and S0) derived.
#[test]
fn the_fixtures_give_the_designs_known_answers() {
    for (name, fixture) in json(FIXTURES).as_object().expect("an object") {
        let t = run_trace(&Scenario::fixture(name, fixture).expect("a fixture"));
        let want = &fixture["expect"];
        let n = |v: &Value, k: &str| v[k].as_u64().unwrap_or_else(|| panic!("{name}: no {k}"));
        assert_eq!(n(&t["counters"], "pops"), n(want, "pops"), "{name} pops");
        assert_eq!(n(&t["counters"], "slices"), n(want, "slices"), "{name} slices");
        assert_eq!(
            t["slices"].as_array().map(Vec::len).unwrap_or(0) as u64 + 1,
            n(want, "snapshots"),
            "{name} snapshots"
        );
        assert_eq!(n(&t["counters"], "stale"), n(want, "stale"), "{name} stale");
        assert_eq!(n(&t["counters"], "noops"), n(want, "noops"), "{name} no-ops");
        for (key, state) in want["states"].as_object().expect("states") {
            let found = t["jobs"]
                .as_array()
                .expect("jobs")
                .iter()
                .find(|j| j[1] == *key)
                .unwrap_or_else(|| panic!("{name}: no job {key}"));
            assert_eq!(found[2], *state, "{name}: the state of {key}");
        }
        if let Some(servers) = want.get("servers") {
            assert_eq!(&t["servers"], servers, "{name} servers");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The parameter hash, the input stream and the event pops of the manifest seed are the S0 pins, which the JavaScript prototype
/// produced and the oracle reproduced. Rust is the third engine to agree.
#[test]
fn the_manifest_seed_gives_the_s0_pins() {
    let frozen = json(FROZEN);
    let seed = frozen["manifest_seed"].as_u64().expect("a seed");
    let t = run_trace(&Scenario::mixed_outage_7(seed).expect("the frozen revision"));
    assert_eq!(t["params_sha256"], frozen["parameters_sha256"]);
    assert_eq!(t["input_stream_sha256"], frozen["input_stream_sha256"]);
    assert_eq!(t["event_pop_sha256"], frozen["event_pop_sha256"]);
    let run = &frozen["manifest_seed_run"];
    for (ours, theirs) in [
        ("pops", "pops"),
        ("slices", "slices"),
        ("arrivals", "arrivals"),
        ("stale", "stale"),
        ("noops", "noops"),
    ] {
        assert_eq!(t["counters"][ours], run[theirs], "{ours}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every scenario of both panels gives the hashes on which the oracle agreed: the number of pops and slices, the event-pop hash and
/// the checkpoint of the end of the run.
#[test]
fn every_panel_scenario_gives_the_golden_hashes() {
    let golden = json(GOLDEN);
    for (name, build) in [
        ("panel", Scenario::panel as fn(u64) -> Result<Scenario, Error>),
        ("panel2", Scenario::panel2),
    ] {
        let entries = golden[name].as_array().expect("entries");
        assert!(entries.len() >= 500, "the golden file holds at least 500 seeds of {name}");
        for (seed, want) in entries.iter().enumerate() {
            let t = run_trace(&build(seed as u64).expect("a scenario"));
            let last = t["slices"]
                .as_array()
                .and_then(|s| s.last())
                .map_or(&t["initial_checkpoint"], |s| &s["checkpoint"]);
            assert_eq!(
                (&t["counters"]["pops"], &t["counters"]["slices"], &t["event_pop_sha256"], last),
                (&want[0], &want[1], &want[2], &want[3]),
                "{name} seed {seed}"
            );
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A key names one job (rule R13). Two scripted arrivals that share a key, or a scripted key that the generator uses, are refused
/// before anything runs, and keys that only look alike are accepted. The cases and their messages are in the contract's data.
#[test]
fn the_refusals_of_the_contract_are_refused_in_its_words() {
    for (name, case) in json(ERRORS).as_object().expect("an object") {
        let outcome = Scenario::fixture(name, case).and_then(|s| trace(&s, Limits::default()).map(|_| ()));
        let want = &case["expect"];
        match (outcome, want["outcome"].as_str()) {
            (Ok(()), Some("ok")) => {},
            (Err(e), Some("error")) => {
                assert_eq!(e.kind(), want["kind"].as_str().unwrap_or_default(), "{name}: the kind");
                assert_eq!(e.message(), want["message"].as_str().unwrap_or_default(), "{name}: the message");
            },
            (got, _) => panic!("{name}: {got:?}, and {want} was expected"),
        }
    }
}

/// The scenario that a duplicate key would have broken: with two keys, a cancel reaches the job it names and not the other.
#[test]
fn a_cancel_reaches_the_job_that_its_key_names() {
    let fixtures = json(FIXTURES);
    let t = run_trace(&Scenario::fixture("E3-KEY", &fixtures["E3-KEY"]).expect("a fixture"));
    let jobs = t["jobs"].as_array().expect("jobs");
    assert_eq!(jobs[0][2], "cancelled", "the running job X is the one cancelled");
    assert_eq!(jobs[1][2], "completed", "the queued job Y runs on after it");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_same_seed_gives_the_same_trace_and_another_seed_another() {
    let (a, b, c) = (
        Scenario::panel(11).unwrap(),
        Scenario::panel(11).unwrap(),
        Scenario::panel(12).unwrap(),
    );
    assert_eq!(run_trace(&a), run_trace(&b));
    assert_ne!(run_trace(&a)["event_pop_sha256"], run_trace(&c)["event_pop_sha256"]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The invariants of the design, checked after every slice of 200 seeds of each panel: conservation, capacity, and a time that
/// never goes backwards.
#[test]
fn the_invariants_hold_after_every_slice() {
    for build in [Scenario::panel as fn(u64) -> Result<Scenario, Error>, Scenario::panel2] {
        for seed in 0..200 {
            let s = build(seed).unwrap();
            let mut engine =
                Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default()).unwrap();
            let mut last_tick = 0;
            let mut popped = 0u64;
            while let Some(slice) = engine.step().unwrap() {
                assert!(slice.tick >= last_tick, "time went backwards");
                last_tick = slice.tick;
                assert!(slice.waiting <= s.config.queue_capacity, "the queue is over its capacity");
                assert!(slice.busy <= s.config.servers, "more servers are busy than there are");
                assert!(
                    slice.pops.windows(2).all(|w| w[0].kind as u8 <= w[1].kind as u8),
                    "the pops of a slice are not in rank order"
                );
                let c = engine.counters();
                assert_eq!(c.arrivals, c.admitted + c.rejected, "arrivals are not admitted plus rejected");
                let terminal = c.completed + c.timed_out + c.cancelled + c.failed_attempts + c.failed_deferrals;
                assert!(terminal <= c.admitted, "more jobs are terminal than were admitted");
                popped += slice.pops.len() as u64;
                assert_eq!(c.pops, popped, "the pops counted are not the pops of the slices");
            }
            // At the end every admitted job is in exactly one terminal class.
            let end = engine.finish_run();
            let c = end.counters;
            assert_eq!(
                c.admitted,
                c.completed + c.timed_out + c.cancelled + c.failed_attempts + c.failed_deferrals,
                "seed {seed}: a job is not terminal at the end"
            );
            assert_eq!(end.jobs.len() as u64, c.arrivals);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn config() -> Config {
    Config {
        servers: 2,
        queue_capacity: 3,
        max_attempts: 2,
        deferral_cap: 4,
    }
}

fn arrive(at: u64, key: &str, durations: Vec<u64>, deadline: u64) -> ScriptEvent {
    ScriptEvent::Arrive {
        at,
        key: key.into(),
        priority: 1,
        durations,
        deadline,
    }
}

fn engine_of(config: Config, script: &[ScriptEvent], limits: Limits) -> Result<Engine, Error> {
    Engine::new("test", 0, config, script, None, limits)
}

fn run_to_end(mut e: Engine) -> Result<(), Error> {
    while e.step()?.is_some() {}
    Ok(())
}

#[test]
fn a_configuration_outside_its_range_is_refused_before_anything_runs() {
    for bad in [
        Config { servers: 0, ..config() },
        Config { servers: 5, ..config() },
        Config { queue_capacity: 0, ..config() },
        Config { queue_capacity: 65, ..config() },
        Config { max_attempts: 0, ..config() },
        Config { max_attempts: 4, ..config() },
        Config { deferral_cap: 17, ..config() },
    ] {
        assert!(
            matches!(engine_of(bad, &[], Limits::default()), Err(Error::Invalid(_))),
            "{bad:?}"
        );
    }
}

/// A scripted event that is not well formed is refused when the engine is built (rule R14), and not when the event pops.
#[test]
fn a_scripted_event_that_is_not_well_formed_is_refused_at_construction() {
    let cases: [(ScriptEvent, &str); 6] = [
        (arrive(5, "J", vec![1], 5), "deadline"),
        (ScriptEvent::Fail { at: 1, server: 2 }, "server 2"),
        (ScriptEvent::Recover { at: 1, server: 2 }, "server 2"),
        (arrive(0, "J", vec![0], 9), "duration"),
        (arrive(0, "J", vec![2, 0], 9), "duration"),
        (arrive(0, "J", vec![], 9), "duration"),
    ];
    for (event, word) in cases {
        let got = engine_of(config(), std::slice::from_ref(&event), Limits::default()).err();
        assert!(
            matches!(&got, Some(Error::Invalid(text)) if text.contains(word)),
            "{event:?}: {got:?}"
        );
    }
}

/// A bad deadline on a job that would have been rejected on arrival is refused as well, since nothing else would ever look at it.
#[test]
fn a_bad_deadline_on_a_job_that_would_be_rejected_is_still_refused() {
    let config = Config {
        servers: 1,
        queue_capacity: 1,
        ..config()
    };
    let script = [
        arrive(0, "A", vec![9], 50),
        arrive(0, "B", vec![9], 50),
        arrive(0, "C", vec![3], 0),
    ];
    assert!(matches!(engine_of(config, &script, Limits::default()), Err(Error::Invalid(_))));
}

/// The configuration of a generated run is that of its parameters (rule R14).
#[test]
fn parameters_and_configuration_that_differ_are_refused() {
    let params = Params::panel(1);
    let given = params.config().expect("a configuration");
    let new = |config: Config| Engine::new("x", 1, config, &[], Some(&params), Limits::default());
    assert!(new(given).is_ok());
    for (name, bad) in [
        (
            "servers",
            Config {
                servers: given.servers % 4 + 1,
                ..given
            },
        ),
        (
            "queue capacity",
            Config {
                queue_capacity: given.queue_capacity % 64 + 1,
                ..given
            },
        ),
        (
            "maximum attempts",
            Config {
                max_attempts: given.max_attempts % 3 + 1,
                ..given
            },
        ),
        (
            "retry deferral cap",
            Config {
                deferral_cap: (given.deferral_cap + 1) % 17,
                ..given
            },
        ),
    ] {
        let got = new(bad).err();
        assert!(
            matches!(&got, Some(Error::Invalid(text)) if text.starts_with(&format!("the parameters give {name}"))),
            "{name}: {got:?}"
        );
    }
}

/// A tick that would pass the end of the integers is an error, and the engine does not wrap.
#[test]
fn a_tick_that_would_overflow_is_an_error() {
    let late = u64::MAX - 1;
    let e = engine_of(config(), &[arrive(late, "J", vec![5], u64::MAX)], Limits::default()).unwrap();
    assert!(matches!(run_to_end(e), Err(Error::Overflow(_))));
}

#[test]
fn a_limit_that_is_exceeded_is_an_error_and_not_a_truncation() {
    let script: Vec<ScriptEvent> = (0..20).map(|i| arrive(i, &format!("J{i}"), vec![1], i + 5)).collect();
    let e = engine_of(
        config(),
        &script,
        Limits {
            max_pops: 10,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(run_to_end(e), Err(Error::Limit("pops")));
    let e = engine_of(
        config(),
        &script,
        Limits {
            max_slices: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(run_to_end(e), Err(Error::Limit("slices")));
    assert_eq!(
        engine_of(
            config(),
            &script,
            Limits {
                max_heap: 5,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit("future events"))
    );
    let mut p = Params::mixed_outage_7();
    p.arrivals = 11;
    assert_eq!(
        p.validate(&Limits {
            max_arrivals: 10,
            ..Limits::default()
        }),
        Err(Error::Limit("arrivals"))
    );
}

/// A scenario with no events has no pops and no slices, and a checkpoint of its own.
#[test]
fn an_empty_scenario_has_nothing_to_run() {
    let mut e = engine_of(config(), &[], Limits::default()).unwrap();
    assert!(e.step().unwrap().is_none());
    assert_eq!(e.counters().pops, 0);
}
