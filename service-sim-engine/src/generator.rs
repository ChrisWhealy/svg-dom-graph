//! The generator of a seed: the arrivals one at a time, and the outages all at once.
//!
//! Every draw is an integer, and comes from one of six streams of the seed, so that changing how one thing is drawn cannot move
//! another. The scenario is described in `dess/s0/docs/generator-and-qualification.md`.

use crate::engine::ScriptEvent;
use crate::hash::sha256_hex;
use crate::params::Params;
use crate::prng::{SplitMix64, stream};

/// What one arrival draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub gap: u64,
    pub priority: u8,
    pub durations: Vec<u64>,
    pub slack: u64,
    /// Zero for a job that is not cancelled.
    pub cancel_after: u64,
}

/// A scripted outage event: a failure or a recovery of one server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutageEvent {
    pub fail: bool,
    pub at: u64,
    pub server: u64,
}

#[derive(Debug, Clone)]
pub struct Generator {
    params: Params,
    gap: SplitMix64,
    priority: SplitMix64,
    duration: SplitMix64,
    slack: SplitMix64,
    cancel: SplitMix64,
    /// How many arrivals have been drawn.
    pub made: u64,
}

impl Generator {
    #[must_use]
    pub fn new(seed: u64, params: &Params) -> Self {
        Self {
            params: params.clone(),
            gap: stream(seed, 1),
            priority: stream(seed, 2),
            duration: stream(seed, 3),
            slack: stream(seed, 4),
            cancel: stream(seed, 5),
            made: 0,
        }
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.params.arrivals
    }

    /// The draws of the next arrival, in the order of the contract.
    pub fn next_arrival(&mut self) -> Arrival {
        let p = &self.params;
        let gap = self.gap.between(p.gap[0], p.gap[1]);
        let priority = u8::from(self.priority.below(100) < p.high_percent);
        let durations = (0..p.max_attempts)
            .map(|_| self.duration.between(p.duration[0], p.duration[1]))
            .collect();
        let slack = self.slack.between(p.slack[0], p.slack[1]);
        let cancel_after = if self.cancel.below(100) < p.cancel_percent {
            self.cancel.between(p.cancel_after[0], p.cancel_after[1])
        } else {
            0
        };
        self.made += 1;
        Arrival {
            gap,
            priority,
            durations,
            slack,
            cancel_after,
        }
    }

    /// The states of the five streams, in the order of the canonical encoding.
    #[must_use]
    pub fn states(&self) -> [u64; 5] {
        [
            self.gap.state(),
            self.priority.state(),
            self.duration.state(),
            self.slack.state(),
            self.cancel.state(),
        ]
    }
}

/// The scripted outages of a seed: a failure, a recovery some ticks later, and a pause before the next.
#[must_use]
pub fn outages(seed: u64, p: &Params) -> Vec<OutageEvent> {
    let mut s: SplitMix64 = stream(seed, 6);
    let horizon = p.arrivals.saturating_mul(p.gap[0].saturating_add(p.gap[1]).div_ceil(2));
    let mut events = Vec::new();
    let mut at = s.between(p.outage_first[0], p.outage_first[1]);
    while at < horizon {
        let server = s.below(p.servers);
        let length = s.between(p.outage_length[0], p.outage_length[1]);
        events.push(OutageEvent { fail: true, at, server });
        events.push(OutageEvent {
            fail: false,
            at: at.saturating_add(length),
            server,
        });
        at = at
            .saturating_add(length)
            .saturating_add(s.between(p.outage_spacing[0], p.outage_spacing[1]));
    }
    events
}

/// The script of a disturbed panel seed (`panel/2`): scripted arrivals, and failures, recoveries and cancels at random ticks, drawn
/// from stream 8 in the order of the contract.
#[must_use]
pub fn disturbances(seed: u64, p: &Params) -> Vec<ScriptEvent> {
    let mut s = stream(seed, 8);
    let horizon = 2 * p.arrivals + 300;
    let mut script = Vec::new();
    let arrivals = s.between(0, 10);
    for i in 0..arrivals {
        let at = s.between(1, horizon);
        let priority = u8::try_from(s.below(2)).unwrap_or(0);
        let durations = (0..p.max_attempts).map(|_| s.between(p.duration[0], p.duration[1])).collect();
        let deadline = at + s.between(1, 60);
        script.push(ScriptEvent::Arrive {
            at,
            key: format!("S{i}"),
            priority,
            durations,
            deadline,
        });
    }
    let events = s.between(0, 40);
    for _ in 0..events {
        let kind = s.below(3);
        let at = s.between(1, horizon);
        match kind {
            0 => script.push(ScriptEvent::Fail { at, server: s.below(p.servers) }),
            1 => script.push(ScriptEvent::Recover { at, server: s.below(p.servers) }),
            _ => {
                let generated = s.below(2) == 0;
                let key = if generated {
                    format!("g{}", s.between(0, p.arrivals + 5))
                } else {
                    format!("S{}", s.between(0, 12))
                };
                script.push(ScriptEvent::Cancel { at, key });
            },
        }
    }
    script
}

/// Everything the generator produces for a seed, as text, and its hash.
#[must_use]
pub fn input_stream_hash(seed: u64, p: &Params) -> String {
    let mut lines: Vec<String> = outages(seed, p)
        .iter()
        .map(|e| format!("{},{},{}", if e.fail { "fail" } else { "recover" }, e.at, e.server))
        .collect();
    let mut g = Generator::new(seed, p);
    for i in 0..g.count() {
        let a = g.next_arrival();
        let durations: Vec<String> = a.durations.iter().map(u64::to_string).collect();
        lines.push(format!(
            "arrive,{i},{},{},{},{},{}",
            a.gap,
            a.priority,
            durations.join("+"),
            a.slack,
            a.cancel_after
        ));
    }
    sha256_hex(lines.join("\n").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The S0 pin of the input stream of the manifest seed, which the prototype produced and the oracle reproduced.
    #[test]
    fn the_input_stream_of_the_manifest_seed_is_the_s0_pin() {
        assert_eq!(
            input_stream_hash(20_261_106, &Params::mixed_outage_7()),
            "411e2bb54bc537a15ad2e1b58f1cc3d4cac72a360a470ea12600cfdde1355837"
        );
    }

    #[test]
    fn no_arrivals_means_no_outages() {
        let mut p = Params::mixed_outage_7();
        p.arrivals = 0;
        assert!(outages(1, &p).is_empty());
    }

    #[test]
    fn a_generator_draws_the_same_arrivals_for_the_same_seed() {
        let p = Params::mixed_outage_7();
        let (mut a, mut b) = (Generator::new(7, &p), Generator::new(7, &p));
        for _ in 0..50 {
            assert_eq!(a.next_arrival(), b.next_arrival());
        }
        assert_eq!(a.states(), b.states());
        assert_eq!(a.made, 50);
    }
}
