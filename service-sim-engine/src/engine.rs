//! The engine: an integer-tick event heap, and the rules of design revision 6 section 3.
//!
//! An event is `(tick, rank, sequence, payload)` in a min-heap. A slice is every event at the smallest tick, popped in rank
//! order, followed by one dispatch. A pop that finds its job or attempt already gone is stale, and a pop that has nothing to act
//! on is a no-op. Both are still pops, and both are in the event-pop hash. The rules the design leaves open are fixed in
//! `dess/s1/docs/contract.md`, and each is named there as R1 to R12.

use crate::generator::{Generator, OutageEvent, outages};
use crate::hash::Writer;
use crate::params::{Config, Error, Limits, Params};
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The kind of an event. The number is its rank and also its code in the canonical encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Complete = 0,
    Fail = 1,
    Cancel = 2,
    Deadline = 3,
    Recover = 4,
    Retry = 5,
    Arrive = 6,
}

impl Kind {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Fail => "fail",
            Self::Cancel => "cancel",
            Self::Deadline => "deadline",
            Self::Recover => "recover",
            Self::Retry => "retry",
            Self::Arrive => "arrive",
        }
    }
}

/// What a pop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It changed something.
    Effective,
    /// It carries a token, and found its job or attempt gone.
    Stale,
    /// It named something that had nothing to act on.
    NoOp,
}

impl Outcome {
    fn letter(self) -> char {
        match self {
            Self::Effective => 'e',
            Self::Stale => 's',
            Self::NoOp => 'n',
        }
    }

    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Effective => 0,
            Self::Stale => 1,
            Self::NoOp => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Payload {
    Complete {
        job: u64,
        token: u64,
    },
    Fail {
        server: u64,
    },
    Cancel {
        key: String,
    },
    Deadline {
        job: u64,
    },
    Recover {
        server: u64,
    },
    Retry {
        job: u64,
        token: u64,
    },
    Arrive {
        key: String,
        generated: bool,
        priority: u8,
        durations: Vec<u64>,
        deadline_or_slack: u64,
        cancel_delay: u64,
    },
}

#[derive(Debug, Clone)]
struct Event {
    tick: u64,
    rank: u8,
    seq: u64,
    kind: Kind,
    payload: Payload,
}

/// The heap orders events by `(tick, rank, sequence)`, which is unique since the sequence is.
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        (self.tick, self.rank, self.seq) == (other.tick, other.rank, other.seq)
    }
}
impl Eq for Event {}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.tick, self.rank, self.seq).cmp(&(other.tick, other.rank, other.seq))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    AttemptsExhausted,
    DeferralsExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Queued,
    Running,
    RetryWait,
    Completed,
    TimedOut,
    Cancelled,
    Rejected,
    Failed(Failure),
}

impl State {
    fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::RetryWait)
    }

    fn text(self) -> String {
        match self {
            Self::Queued => "queued".into(),
            Self::Running => "running".into(),
            Self::RetryWait => "retrywait".into(),
            Self::Completed => "completed".into(),
            Self::TimedOut => "timedout".into(),
            Self::Cancelled => "cancelled".into(),
            Self::Rejected => "rejected".into(),
            Self::Failed(Failure::AttemptsExhausted) => "failed(attempts_exhausted)".into(),
            Self::Failed(Failure::DeferralsExhausted) => "failed(retry_deferrals_exhausted)".into(),
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Queued => 1,
            Self::Running => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct Job {
    id: u64,
    key: String,
    priority: u8,
    durations: Vec<u64>,
    state: State,
    attempt: u32,
    failures: u32,
    token: u64,
    defers: u32,
    entered: u64,
    server: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Server {
    Idle,
    Busy { job: u64, due: u64 },
    Down,
}

/// The twelve counters, in the order of the canonical encoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    pub pops: u64,
    pub stale: u64,
    pub noops: u64,
    pub slices: u64,
    pub arrivals: u64,
    pub admitted: u64,
    pub rejected: u64,
    pub completed: u64,
    pub timed_out: u64,
    pub cancelled: u64,
    pub failed_attempts: u64,
    pub failed_deferrals: u64,
}

impl Counters {
    #[must_use]
    pub fn as_array(&self) -> [u64; 12] {
        [
            self.pops,
            self.stale,
            self.noops,
            self.slices,
            self.arrivals,
            self.admitted,
            self.rejected,
            self.completed,
            self.timed_out,
            self.cancelled,
            self.failed_attempts,
            self.failed_deferrals,
        ]
    }

    /// The names the trace uses, in the same order.
    pub const NAMES: [&'static str; 12] = [
        "pops",
        "stale",
        "noops",
        "slices",
        "arrivals",
        "admitted",
        "rejected",
        "completed",
        "timedOut",
        "cancelled",
        "failedAttempts",
        "failedDeferrals",
    ];
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An event of a script, which is known before the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptEvent {
    Arrive {
        at: u64,
        key: String,
        priority: u8,
        durations: Vec<u64>,
        deadline: u64,
    },
    Cancel {
        at: u64,
        key: String,
    },
    Fail {
        at: u64,
        server: u64,
    },
    Recover {
        at: u64,
        server: u64,
    },
}

/// One pop, as it is recorded for comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopRecord {
    pub tick: u64,
    pub kind: Kind,
    pub seq: u64,
    pub outcome: Outcome,
    /// The job number, or the server index, or -1 for a cancel of a key with no job.
    pub subject: i64,
}

/// One slice, as it is recorded for comparison.
#[derive(Debug, Clone)]
pub struct SliceRecord {
    pub index: u64,
    pub tick: u64,
    pub pops: Vec<PopRecord>,
    pub waiting: usize,
    pub busy: usize,
    pub checkpoint: String,
}

/// The state of every job and server at the end of a run.
#[derive(Debug, Clone)]
pub struct Final {
    pub counters: Counters,
    pub jobs: Vec<(u64, String, String)>,
    pub servers: Vec<&'static str>,
    pub event_pop_sha256: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub struct Engine {
    config_id: String,
    seed: u64,
    config: Config,
    limits: Limits,
    heap: BinaryHeap<Reverse<Event>>,
    next_seq: u64,
    jobs: Vec<Job>,
    by_key: BTreeMap<String, u64>,
    servers: Vec<Server>,
    waiting: Vec<u64>,
    counters: Counters,
    generator: Option<Generator>,
    pop_hash: Sha256,
    hash_pops: bool,
}

fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow("a tick"))
}

impl Engine {
    /// Builds an engine for a scenario, and checks it before it allocates what the scenario asks for.
    ///
    /// A generated run gives `params`, and then `config` is the configuration of those parameters.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the configuration or the parameters are refused, or a scripted event is out of range.
    pub fn new(
        config_id: &str,
        seed: u64,
        config: Config,
        script: &[ScriptEvent],
        params: Option<&Params>,
        limits: Limits,
    ) -> Result<Self, Error> {
        config.validate()?;
        if let Some(p) = params {
            p.validate(&limits)?;
            Self::validate_agreement(&config, p)?;
        }
        Self::validate_keys(script, params)?;
        Self::validate_script(script, &config)?;
        let mut engine = Self {
            config_id: config_id.to_owned(),
            seed,
            config,
            limits,
            heap: BinaryHeap::new(),
            next_seq: 0,
            jobs: Vec::new(),
            by_key: BTreeMap::new(),
            servers: vec![Server::Idle; config.servers],
            waiting: Vec::new(),
            counters: Counters::default(),
            generator: params.map(|p| Generator::new(seed, p)),
            pop_hash: Sha256::new(),
            hash_pops: true,
        };
        for event in script {
            match event {
                ScriptEvent::Arrive {
                    at,
                    key,
                    priority,
                    durations,
                    deadline,
                } => engine.create(
                    *at,
                    Kind::Arrive,
                    Payload::Arrive {
                        key: key.clone(),
                        generated: false,
                        priority: *priority,
                        durations: durations.clone(),
                        deadline_or_slack: *deadline,
                        cancel_delay: 0,
                    },
                )?,
                ScriptEvent::Cancel { at, key } => {
                    engine.create(*at, Kind::Cancel, Payload::Cancel { key: key.clone() })?
                },
                ScriptEvent::Fail { at, server } => {
                    engine.create(*at, Kind::Fail, Payload::Fail { server: *server })?
                },
                ScriptEvent::Recover { at, server } => {
                    engine.create(*at, Kind::Recover, Payload::Recover { server: *server })?
                },
            }
        }
        if let Some(p) = params {
            for OutageEvent { fail, at, server } in outages(seed, p) {
                if fail {
                    engine.create(at, Kind::Fail, Payload::Fail { server })?;
                } else {
                    engine.create(at, Kind::Recover, Payload::Recover { server })?;
                }
            }
            engine.create_generated_arrival(0)?;
        }
        Ok(engine)
    }

    /// The configuration of a generated run is that of its parameters (rule R14), so that a generated outage names a server that exists.
    fn validate_agreement(config: &Config, params: &Params) -> Result<(), Error> {
        let given = params.config()?;
        let pairs = [
            ("servers", given.servers as u64, config.servers as u64),
            ("queue capacity", given.queue_capacity as u64, config.queue_capacity as u64),
            (
                "maximum attempts",
                u64::from(given.max_attempts),
                u64::from(config.max_attempts),
            ),
            (
                "retry deferral cap",
                u64::from(given.deferral_cap),
                u64::from(config.deferral_cap),
            ),
        ];
        for (name, from_params, from_config) in pairs {
            if from_params != from_config {
                return Err(Error::Invalid(format!(
                    "the parameters give {name} {from_params}, and the configuration gives {from_config}"
                )));
            }
        }
        Ok(())
    }

    /// A scripted event has to be well formed (rule R14), and is refused here and not when it pops.
    fn validate_script(script: &[ScriptEvent], config: &Config) -> Result<(), Error> {
        for event in script {
            match event {
                ScriptEvent::Fail { server, .. } | ScriptEvent::Recover { server, .. }
                    if usize::try_from(*server).map_or(true, |s| s >= config.servers) =>
                {
                    return Err(Error::Invalid(format!("server {server} does not exist")));
                },
                ScriptEvent::Arrive {
                    at, key, durations, deadline, ..
                } => {
                    if durations.is_empty() || durations.contains(&0) {
                        return Err(Error::Invalid("a duration is not positive".into()));
                    }
                    if deadline <= at {
                        return Err(Error::Invalid(format!("the deadline of {key} is not after its admission")));
                    }
                },
                _ => {},
            }
        }
        Ok(())
    }

    /// The keys of the script and the generator have to name one job each (rule R13), and are checked before anything runs.
    fn validate_keys(script: &[ScriptEvent], params: Option<&Params>) -> Result<(), Error> {
        let mut seen = BTreeSet::new();
        for event in script {
            let ScriptEvent::Arrive { key, .. } = event else { continue };
            if !seen.insert(key.as_str()) {
                return Err(Error::Invalid(format!("the arrival key {key} is used more than once")));
            }
            // A generated key is `g` and a number without a leading zero, so `g01` is not one.
            let generated = key
                .strip_prefix('g')
                .and_then(|rest| rest.parse::<u64>().ok())
                .filter(|n| format!("g{n}") == *key);
            if let (Some(p), Some(n)) = (params, generated) {
                if n < p.arrivals {
                    return Err(Error::Invalid(format!("the arrival key {key} is one that the generator uses")));
                }
            }
        }
        Ok(())
    }

    /// Creates an event with the next sequence number.
    fn create(&mut self, tick: u64, kind: Kind, payload: Payload) -> Result<(), Error> {
        if self.heap.len() >= self.limits.max_heap {
            return Err(Error::Limit("future events"));
        }
        let seq = self.next_seq;
        self.next_seq = self.next_seq.checked_add(1).ok_or(Error::Overflow("the event sequence"))?;
        self.heap.push(Reverse(Event {
            tick,
            rank: kind as u8,
            seq,
            kind,
            payload,
        }));
        Ok(())
    }

    fn create_generated_arrival(&mut self, from: u64) -> Result<(), Error> {
        let Some(generator) = self.generator.as_mut() else { return Ok(()) };
        if generator.made >= generator.count() {
            return Ok(());
        }
        let index = generator.made;
        let a = generator.next_arrival();
        let tick = add(from, a.gap)?;
        self.create(
            tick,
            Kind::Arrive,
            Payload::Arrive {
                key: format!("g{index}"),
                generated: true,
                priority: a.priority,
                durations: a.durations,
                deadline_or_slack: a.slack,
                cancel_delay: a.cancel_after,
            },
        )
    }

    fn finish(&mut self, id: usize, state: State) {
        let job = &mut self.jobs[id];
        job.state = state;
        job.token += 1;
        match state {
            State::Completed => self.counters.completed += 1,
            State::TimedOut => self.counters.timed_out += 1,
            State::Cancelled => self.counters.cancelled += 1,
            State::Failed(Failure::AttemptsExhausted) => self.counters.failed_attempts += 1,
            State::Failed(Failure::DeferralsExhausted) => self.counters.failed_deferrals += 1,
            _ => {},
        }
    }

    /// Takes an active job out of the queue, or frees its server.
    fn leave(&mut self, id: usize) {
        match self.jobs[id].state {
            State::Queued => {
                if let Some(at) = self.waiting.iter().position(|w| *w == id as u64) {
                    self.waiting.remove(at);
                }
            },
            State::Running => self.servers[self.jobs[id].server] = Server::Idle,
            _ => {},
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // One function for each kind of event.

    fn arrive(&mut self, tick: u64, payload: Payload) -> Result<(Outcome, i64), Error> {
        let Payload::Arrive {
            key,
            generated,
            priority,
            durations,
            deadline_or_slack,
            cancel_delay,
        } = payload
        else {
            unreachable!("an arrival carries an arrival");
        };
        if self.by_key.contains_key(&key) {
            return Err(Error::Invalid(format!("the arrival key {key} is already bound to a job")));
        }
        let id = self.jobs.len();
        self.jobs.push(Job {
            id: id as u64,
            key: key.clone(),
            priority,
            durations,
            state: State::Rejected,
            attempt: 0,
            failures: 0,
            token: 0,
            defers: 0,
            entered: 0,
            server: 0,
        });
        self.by_key.insert(key.clone(), id as u64);
        self.counters.arrivals += 1;
        if generated {
            self.create_generated_arrival(tick)?;
        }
        if self.waiting.len() < self.config.queue_capacity {
            self.jobs[id].state = State::Queued;
            self.jobs[id].entered = tick;
            self.waiting.push(id as u64);
            self.counters.admitted += 1;
            let deadline = if generated { add(tick, deadline_or_slack)? } else { deadline_or_slack };
            if deadline <= tick {
                return Err(Error::Invalid(format!("the deadline of {key} is not after its admission")));
            }
            self.create(deadline, Kind::Deadline, Payload::Deadline { job: id as u64 })?;
            if generated && cancel_delay > 0 {
                self.create(add(tick, cancel_delay)?, Kind::Cancel, Payload::Cancel { key })?;
            }
        } else {
            self.counters.rejected += 1;
        }
        Ok((Outcome::Effective, id as i64))
    }

    fn complete(&mut self, job: u64, token: u64) -> (Outcome, i64) {
        let id = job as usize;
        if self.jobs[id].state == State::Running && self.jobs[id].token == token {
            self.servers[self.jobs[id].server] = Server::Idle;
            self.finish(id, State::Completed);
            (Outcome::Effective, job as i64)
        } else {
            (Outcome::Stale, job as i64)
        }
    }

    fn deadline(&mut self, job: u64) -> (Outcome, i64) {
        let id = job as usize;
        if self.jobs[id].state.active() {
            self.leave(id);
            self.finish(id, State::TimedOut);
            (Outcome::Effective, job as i64)
        } else {
            (Outcome::Stale, job as i64)
        }
    }

    fn cancel(&mut self, key: &str) -> (Outcome, i64) {
        match self.by_key.get(key).copied() {
            Some(job) if self.jobs[job as usize].state.active() => {
                self.leave(job as usize);
                self.finish(job as usize, State::Cancelled);
                (Outcome::Effective, job as i64)
            },
            Some(job) => (Outcome::NoOp, job as i64),
            None => (Outcome::NoOp, -1),
        }
    }

    fn fail(&mut self, tick: u64, server: u64) -> Result<(Outcome, i64), Error> {
        let index = server as usize;
        match self.servers[index] {
            Server::Busy { job, .. } => {
                self.servers[index] = Server::Down;
                let id = job as usize;
                self.jobs[id].token += 1;
                self.jobs[id].failures += 1;
                if self.jobs[id].failures < self.config.max_attempts {
                    self.jobs[id].state = State::RetryWait;
                    let backoff = 2u64 << (self.jobs[id].failures - 1);
                    let token = self.jobs[id].token;
                    self.create(add(tick, backoff)?, Kind::Retry, Payload::Retry { job, token })?;
                } else {
                    self.finish(id, State::Failed(Failure::AttemptsExhausted));
                }
                Ok((Outcome::Effective, server as i64))
            },
            Server::Idle => {
                self.servers[index] = Server::Down;
                Ok((Outcome::Effective, server as i64))
            },
            Server::Down => Ok((Outcome::NoOp, server as i64)),
        }
    }

    fn recover(&mut self, server: u64) -> (Outcome, i64) {
        let index = server as usize;
        if self.servers[index] == Server::Down {
            self.servers[index] = Server::Idle;
            (Outcome::Effective, server as i64)
        } else {
            (Outcome::NoOp, server as i64)
        }
    }

    fn retry(&mut self, tick: u64, job: u64, token: u64) -> Result<(Outcome, i64), Error> {
        let id = job as usize;
        if self.jobs[id].state == State::RetryWait && self.jobs[id].token == token {
            if self.waiting.len() < self.config.queue_capacity {
                self.jobs[id].state = State::Queued;
                self.jobs[id].entered = tick;
                self.jobs[id].defers = 0;
                self.waiting.push(job);
            } else {
                self.jobs[id].defers += 1;
                if self.jobs[id].defers > self.config.deferral_cap {
                    self.finish(id, State::Failed(Failure::DeferralsExhausted));
                } else {
                    let token = self.jobs[id].token;
                    self.create(add(tick, 1)?, Kind::Retry, Payload::Retry { job, token })?;
                }
            }
            Ok((Outcome::Effective, job as i64))
        } else {
            Ok((Outcome::Stale, job as i64))
        }
    }

    /// After the pops of a slice: the waiting jobs go to the idle servers, once.
    fn dispatch(&mut self, tick: u64) -> Result<(), Error> {
        let jobs = &self.jobs;
        self.waiting.sort_by(|a, b| {
            let (a, b) = (&jobs[*a as usize], &jobs[*b as usize]);
            b.priority
                .cmp(&a.priority)
                .then(a.entered.cmp(&b.entered))
                .then(a.id.cmp(&b.id))
        });
        for index in 0..self.servers.len() {
            if self.servers[index] != Server::Idle || self.waiting.is_empty() {
                continue;
            }
            let id = self.waiting.remove(0) as usize;
            let job = &mut self.jobs[id];
            job.attempt += 1;
            job.state = State::Running;
            job.server = index;
            let nth = (job.attempt - 1) as usize;
            let duration = *job.durations.get(nth).or_else(|| job.durations.last()).unwrap_or(&0);
            if duration < 1 {
                return Err(Error::Invalid("a duration is not positive".into()));
            }
            let (job_id, token) = (job.id, job.token);
            let due = add(tick, duration)?;
            self.servers[index] = Server::Busy { job: job_id, due };
            self.create(due, Kind::Complete, Payload::Complete { job: job_id, token })?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The canonical state, and its hash. `tick` and `batch` are those of the slice just done, or 0 and 0.
    #[must_use]
    pub fn checkpoint(&self, tick: u64, batch: u64) -> String {
        let mut w = Writer::new();
        w.u8(1).string(&self.config_id).u64(self.seed).u64(tick).u64(batch);
        match &self.generator {
            Some(g) => {
                w.u8(1).u64(g.made);
                for state in g.states() {
                    w.u64(state);
                }
            },
            None => {
                w.u8(0);
            },
        }
        for c in self.counters.as_array() {
            w.u64(c);
        }
        let live: Vec<&Job> = self.jobs.iter().filter(|j| j.state.active()).collect();
        w.u32(live.len());
        for j in live {
            w.u64(j.id)
                .u8(j.state.code())
                .string(&j.key)
                .u8(j.priority)
                .u32(j.attempt as usize)
                .u32(j.failures as usize);
            w.u64(j.token).u32(j.defers as usize).u64(j.entered);
            if j.state == State::Running {
                w.u64(j.server as u64);
            }
            w.u64_list(&j.durations);
        }
        w.u32(self.servers.len());
        for s in &self.servers {
            match s {
                Server::Idle => w.u8(0),
                Server::Busy { job, due } => w.u8(1).u64(*job).u64(*due),
                Server::Down => w.u8(2),
            };
        }
        w.u64_list(&self.waiting);
        let mut events: Vec<&Event> = self.heap.iter().map(|r| &r.0).collect();
        events.sort();
        w.u32(events.len());
        for e in events {
            w.u64(e.tick).u8(e.rank).u64(e.seq).u8(e.kind as u8);
            match &e.payload {
                Payload::Complete { job, token } | Payload::Retry { job, token } => {
                    w.u64(*job).u64(*token);
                },
                Payload::Deadline { job } => {
                    w.u64(*job);
                },
                Payload::Fail { server } | Payload::Recover { server } => {
                    w.u64(*server);
                },
                Payload::Cancel { key } => {
                    w.string(key);
                },
                Payload::Arrive {
                    key,
                    generated,
                    priority,
                    durations,
                    deadline_or_slack,
                    cancel_delay,
                } => {
                    w.string(key)
                        .u8(u8::from(*generated))
                        .u8(*priority)
                        .u64_list(durations)
                        .u64(*deadline_or_slack);
                    w.u64(if *generated { *cancel_delay } else { 0 });
                },
            }
        }
        w.u64(self.next_seq);
        w.digest()
    }

    #[must_use]
    pub fn counters(&self) -> Counters {
        self.counters
    }

    /// Turns off the hash of the pops, for a replay that only needs the state at a cursor. The hash that
    /// [`finish_run`](Self::finish_run) gives is then not the hash of the run.
    #[must_use]
    pub fn without_pop_hash(mut self) -> Self {
        self.hash_pops = false;
        self
    }

    /// Runs one slice with nothing recorded and no checkpoint, which is all that a replay to a cursor needs. Gives `false` when the
    /// future heap is empty and the run is over.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a limit is exceeded, an addition overflows, or an event is out of range.
    pub fn advance(&mut self) -> Result<bool, Error> {
        Ok(self.run_slice(false)?.is_some())
    }

    /// Runs one slice. Gives `None` when the future heap is empty and the run is over.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if a limit is exceeded, an addition overflows, or an event is out of range.
    pub fn step(&mut self) -> Result<Option<SliceRecord>, Error> {
        let Some((tick, pops)) = self.run_slice(true)? else { return Ok(None) };
        let busy = self.servers.iter().filter(|s| matches!(s, Server::Busy { .. })).count();
        Ok(Some(SliceRecord {
            index: self.counters.slices,
            tick,
            pops,
            waiting: self.waiting.len(),
            busy,
            checkpoint: self.checkpoint(tick, self.counters.slices),
        }))
    }

    /// The pops of the slice at the smallest tick, and then one dispatch. `record` keeps each pop's record.
    fn run_slice(&mut self, record: bool) -> Result<Option<(u64, Vec<PopRecord>)>, Error> {
        let Some(first) = self.heap.peek() else { return Ok(None) };
        let tick = first.0.tick;
        let mut batch = Vec::new();
        while self.heap.peek().is_some_and(|top| top.0.tick == tick) {
            if let Some(Reverse(event)) = self.heap.pop() {
                batch.push(event);
            }
        }
        if self.counters.slices >= self.limits.max_slices {
            return Err(Error::Limit("slices"));
        }
        let mut pops = Vec::with_capacity(if record { batch.len() } else { 0 });
        for event in batch {
            if self.counters.pops >= self.limits.max_pops {
                return Err(Error::Limit("pops"));
            }
            let servers = self.servers.len() as u64;
            let (outcome, subject) = match event.payload {
                Payload::Arrive { .. } => self.arrive(tick, event.payload)?,
                Payload::Complete { job, token } => self.complete(job, token),
                Payload::Deadline { job } => self.deadline(job),
                Payload::Cancel { ref key } => self.cancel(key),
                Payload::Fail { server } | Payload::Recover { server } if server >= servers => {
                    return Err(Error::Invalid(format!("server {server} does not exist")));
                },
                Payload::Fail { server } => self.fail(tick, server)?,
                Payload::Recover { server } => self.recover(server),
                Payload::Retry { job, token } => self.retry(tick, job, token)?,
            };
            self.counters.pops += 1;
            match outcome {
                Outcome::Stale => self.counters.stale += 1,
                Outcome::NoOp => self.counters.noops += 1,
                Outcome::Effective => {},
            }
            if self.hash_pops {
                self.pop_hash.update(
                    format!("{}:{}:{}:{};", event.tick, event.kind.name(), event.seq, outcome.letter()).as_bytes(),
                );
            }
            if record {
                pops.push(PopRecord {
                    tick: event.tick,
                    kind: event.kind,
                    seq: event.seq,
                    outcome,
                    subject,
                });
            }
        }
        self.dispatch(tick)?;
        self.counters.slices += 1;
        Ok(Some((tick, pops)))
    }

    /// The end of the run: the counters, every job's state, every server's, and the hash of the pops.
    #[must_use]
    pub fn finish_run(&self) -> Final {
        Final {
            counters: self.counters,
            jobs: self.jobs.iter().map(|j| (j.id, j.key.clone(), j.state.text())).collect(),
            servers: self
                .servers
                .iter()
                .map(|s| match s {
                    Server::Idle => "idle",
                    Server::Busy { .. } => "busy",
                    Server::Down => "down",
                })
                .collect(),
            event_pop_sha256: sha256_hex_of_digest(self.pop_hash.clone()),
        }
    }
}

fn sha256_hex_of_digest(hasher: Sha256) -> String {
    crate::hash::hex(&hasher.finalize())
}
