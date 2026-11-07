//! The trace of a scenario: every pop, every slice with its checkpoint hash, and the final state, as JSON.
//!
//! This is what the differential run compares with the oracle's, so its shape is fixed by `dess/s1/docs/contract.md` section 6.

use crate::engine::{Counters, Engine, ScriptEvent};
use crate::generator::input_stream_hash;
use crate::hash::sha256_hex;
use crate::params::{Config, Error, Limits, Params};
use serde_json::{Value, json};

/// A scenario to run: a script, a generator, or both.
pub struct Scenario {
    pub config_id: String,
    pub seed: u64,
    pub config: Config,
    pub script: Vec<ScriptEvent>,
    pub params: Option<Params>,
}

impl Scenario {
    /// The scenario of a seed in the differential panel.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the derived parameters are refused, which would be a defect.
    pub fn panel(seed: u64) -> Result<Self, Error> {
        let params = Params::panel(seed);
        Ok(Self {
            config_id: format!("panel/1:{seed}"),
            seed,
            config: params.config()?,
            script: Vec::new(),
            params: Some(params),
        })
    }

    /// The scenario of a seed in the disturbed panel: the same parameters, and a script of scripted arrivals, duplicate outages and
    /// cancels of keys that may never arrive.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the derived parameters are refused, which would be a defect.
    pub fn panel2(seed: u64) -> Result<Self, Error> {
        let mut params = Params::panel(seed);
        params.revision = "panel/2".into();
        let script = crate::generator::disturbances(seed, &params);
        Ok(Self {
            config_id: format!("panel/2:{seed}"),
            seed,
            config: params.config()?,
            script,
            params: Some(params),
        })
    }

    /// The frozen revision `mixed_outage/7`, for a seed.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the parameters are refused, which would be a defect.
    pub fn mixed_outage_7(seed: u64) -> Result<Self, Error> {
        let params = Params::mixed_outage_7();
        Ok(Self {
            config_id: "mixed_outage/7".into(),
            seed,
            config: params.config()?,
            script: Vec::new(),
            params: Some(params),
        })
    }

    /// A fixture of `dess/s1/fixtures.json`: a configuration and a script.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the fixture is not in the shape of the file.
    pub fn fixture(name: &str, v: &Value) -> Result<Self, Error> {
        let bad = |what: &str| Error::Invalid(format!("fixture {name}: {what}"));
        let num = |v: &Value, what: &str| v.as_u64().ok_or_else(|| bad(what));
        let c = v.get("config").ok_or_else(|| bad("no config"))?;
        let get = |k: &str| num(c.get(k).unwrap_or(&Value::Null), k);
        let config = Config {
            servers: usize::try_from(get("servers")?).map_err(|_| bad("servers"))?,
            queue_capacity: usize::try_from(get("queueCapacity")?).map_err(|_| bad("queueCapacity"))?,
            max_attempts: u32::try_from(get("maxAttempts")?).map_err(|_| bad("maxAttempts"))?,
            deferral_cap: u32::try_from(get("deferralCap")?).map_err(|_| bad("deferralCap"))?,
        };
        let mut script = Vec::new();
        for e in v.get("script").and_then(Value::as_array).ok_or_else(|| bad("no script"))? {
            let at = num(e.get("at").unwrap_or(&Value::Null), "at")?;
            let key = || {
                e.get("key")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| bad("key"))
            };
            let server = || num(e.get("server").unwrap_or(&Value::Null), "server");
            script.push(match e.get("kind").and_then(Value::as_str) {
                Some("arrive") => ScriptEvent::Arrive {
                    at,
                    key: key()?,
                    priority: u8::try_from(num(e.get("priority").unwrap_or(&Value::Null), "priority")?)
                        .map_err(|_| bad("priority"))?,
                    durations: e
                        .get("durations")
                        .and_then(Value::as_array)
                        .ok_or_else(|| bad("durations"))?
                        .iter()
                        .map(|d| num(d, "a duration"))
                        .collect::<Result<_, _>>()?,
                    deadline: num(e.get("deadline").unwrap_or(&Value::Null), "deadline")?,
                },
                Some("cancel") => ScriptEvent::Cancel { at, key: key()? },
                Some("fail") => ScriptEvent::Fail { at, server: server()? },
                Some("recover") => ScriptEvent::Recover { at, server: server()? },
                _ => return Err(bad("an event of an unknown kind")),
            });
        }
        let params = v.get("params").map(Params::from_json).transpose()?;
        Ok(Self {
            config_id: format!("fixture/{name}"),
            seed: 0,
            config,
            script,
            params,
        })
    }
}

/// Runs a scenario to its end.
///
/// # Errors
///
/// Returns an [`Error`] if the scenario is refused, or a limit is exceeded, or an addition overflows.
pub fn trace(s: &Scenario, limits: Limits) -> Result<Value, Error> {
    let mut engine = Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), limits)?;
    let initial = engine.checkpoint(0, 0);
    let mut slices = Vec::new();
    let mut pops = Vec::new();
    while let Some(slice) = engine.step()? {
        slices.push(json!({
            "index": slice.index, "tick": slice.tick, "pops": slice.pops.len(), "waiting": slice.waiting,
            "busy": slice.busy, "checkpoint": slice.checkpoint
        }));
        for p in &slice.pops {
            pops.push(json!([p.tick, p.kind as u8, p.seq, p.outcome.code(), p.subject]));
        }
    }
    let end = engine.finish_run();
    let counters: serde_json::Map<String, Value> = Counters::NAMES
        .iter()
        .zip(end.counters.as_array())
        .map(|(n, v)| ((*n).to_owned(), json!(v)))
        .collect();
    Ok(json!({
        "seed": s.seed,
        "config_id": s.config_id,
        "params_sha256": s.params.as_ref().map(|p| sha256_hex(p.canonical_json().as_bytes())),
        "input_stream_sha256": s.params.as_ref().map(|p| input_stream_hash(s.seed, p)),
        "event_pop_sha256": end.event_pop_sha256,
        "initial_checkpoint": initial,
        "counters": counters,
        "slices": slices,
        "pops": pops,
        "jobs": end.jobs.iter().map(|(id, key, state)| json!([id, key, state])).collect::<Vec<_>>(),
        "servers": end.servers,
    }))
}
