//! A scenario's configuration and the parameters of its generator, and the revisions that are fixed.

use crate::prng::stream;
use serde_json::Value;

/// Why a scenario was refused. Every check is made before anything is allocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A figure is outside what the engine accepts.
    Invalid(String),
    /// An addition overflowed. Ticks and counters are checked, and never wrap.
    Overflow(&'static str),
    /// A limit was exceeded, which is an error and not a truncation.
    Limit(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(text) => write!(f, "invalid: {text}"),
            Self::Overflow(what) => write!(f, "overflow: {what}"),
            Self::Limit(what) => write!(f, "limit exceeded: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// The kind of the error, as the differential run compares it.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid",
            Self::Overflow(_) => "overflow",
            Self::Limit(_) => "limit",
        }
    }

    /// The message of the error, without its kind.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Invalid(text) => text.clone(),
            Self::Overflow(what) | Self::Limit(what) => (*what).to_owned(),
        }
    }
}

/// The part of a scenario that the engine's rules use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub servers: usize,
    pub queue_capacity: usize,
    pub max_attempts: u32,
    pub deferral_cap: u32,
}

impl Config {
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a figure is outside its range.
    pub fn validate(&self) -> Result<(), Error> {
        let check = |name: &str, v: u64, lo: u64, hi: u64| {
            if (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(Error::Invalid(format!("{name} {v} is outside {lo} to {hi}")))
            }
        };
        check("servers", self.servers as u64, 1, 4)?;
        check("queue capacity", self.queue_capacity as u64, 1, 64)?;
        check("maximum attempts", u64::from(self.max_attempts), 1, 3)?;
        check("retry deferral cap", u64::from(self.deferral_cap), 0, 16)
    }
}

/// Limits that are checked before a run and while it goes on, so that a scenario cannot ask for unbounded work.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_arrivals: u64,
    pub max_heap: usize,
    pub max_pops: u64,
    pub max_slices: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_arrivals: 10_000_000,
            max_heap: 1 << 20,
            max_pops: 100_000_000,
            max_slices: 100_000_000,
        }
    }
}

/// The parameters of a generator, and the configuration that goes with them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    pub revision: String,
    pub servers: u64,
    pub queue_capacity: u64,
    pub max_attempts: u64,
    pub deferral_cap: u64,
    pub arrivals: u64,
    pub gap: [u64; 2],
    pub high_percent: u64,
    pub duration: [u64; 2],
    pub slack: [u64; 2],
    pub cancel_percent: u64,
    pub cancel_after: [u64; 2],
    pub outage_first: [u64; 2],
    pub outage_length: [u64; 2],
    pub outage_spacing: [u64; 2],
}

/// The largest tick that a range of the generator may reach, so that no sum of ranges can overflow.
pub const MAX_RANGE: u64 = 1_000_000;

fn pair(p: [u64; 2]) -> String {
    format!("[{},{}]", p[0], p[1])
}

impl Params {
    /// The engine's configuration of these parameters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a figure does not fit, or is outside its range.
    pub fn config(&self) -> Result<Config, Error> {
        let narrow =
            |name: &str, v: u64| u32::try_from(v).map_err(|_| Error::Invalid(format!("{name} {v} is too large")));
        let config = Config {
            servers: usize::try_from(self.servers).map_err(|_| Error::Invalid("servers is too large".into()))?,
            queue_capacity: usize::try_from(self.queue_capacity)
                .map_err(|_| Error::Invalid("queue capacity is too large".into()))?,
            max_attempts: narrow("maximum attempts", self.max_attempts)?,
            deferral_cap: narrow("retry deferral cap", self.deferral_cap)?,
        };
        config.validate()?;
        Ok(config)
    }

    /// The parameters as canonical JSON: every key sorted, and no spaces. Its hash is the parameter hash of a revision.
    #[must_use]
    pub fn canonical_json(&self) -> String {
        let revision = serde_json::to_string(&self.revision).unwrap_or_default();
        format!(
            "{{\"arrivals\":{},\"cancelAfter\":{},\"cancelPercent\":{},\"deferralCap\":{},\"duration\":{},\"gap\":{},\"highPercent\":{},\"maxAttempts\":{},\"outage\":{{\"first\":{},\"length\":{},\"spacing\":{}}},\"queueCapacity\":{},\"revision\":{},\"servers\":{},\"slack\":{}}}",
            self.arrivals,
            pair(self.cancel_after),
            self.cancel_percent,
            self.deferral_cap,
            pair(self.duration),
            pair(self.gap),
            self.high_percent,
            self.max_attempts,
            pair(self.outage_first),
            pair(self.outage_length),
            pair(self.outage_spacing),
            self.queue_capacity,
            revision,
            self.servers,
            pair(self.slack),
        )
    }

    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a range is reversed, or a percentage is above 100, or a duration or a gap is not positive.
    pub fn validate(&self, limits: &Limits) -> Result<(), Error> {
        self.config()?;
        let ranges = [
            ("gap", self.gap, 1),
            ("duration", self.duration, 1),
            ("slack", self.slack, 1),
            ("cancel after", self.cancel_after, 1),
            ("first outage", self.outage_first, 1),
            ("outage length", self.outage_length, 1),
            ("outage pause", self.outage_spacing, 1),
        ];
        for (name, [lo, hi], least) in ranges {
            if lo < least || hi < lo {
                return Err(Error::Invalid(format!(
                    "the {name} range {lo} to {hi} is not a range of positive ticks"
                )));
            }
            if hi > MAX_RANGE {
                return Err(Error::Limit("a range of ticks"));
            }
        }
        if self.high_percent > 100 || self.cancel_percent > 100 {
            return Err(Error::Invalid("a percentage is above 100".into()));
        }
        if self.arrivals > limits.max_arrivals {
            return Err(Error::Limit("arrivals"));
        }
        Ok(())
    }

    /// The parameters of the frozen revision `mixed_outage/7`, which stage S0 fixed.
    #[must_use]
    pub fn mixed_outage_7() -> Self {
        Self {
            revision: "mixed_outage/7".into(),
            servers: 3,
            queue_capacity: 12,
            max_attempts: 2,
            deferral_cap: 4,
            arrivals: 4000,
            gap: [1, 2],
            high_percent: 30,
            duration: [2, 12],
            slack: [6, 30],
            cancel_percent: 4,
            cancel_after: [1, 40],
            outage_first: [100, 200],
            outage_length: [5, 30],
            outage_spacing: [80, 240],
        }
    }

    /// The scenario of a seed in the differential panel, derived from stream 7 in the order the contract gives.
    #[must_use]
    pub fn panel(seed: u64) -> Self {
        let mut s = stream(seed, 7);
        let servers = s.between(2, 4);
        let queue_capacity = s.between(1, 16);
        let max_attempts = s.between(1, 3);
        let deferral_cap = s.between(0, 6);
        let arrivals = s.between(0, 200);
        let gap_hi = s.between(1, 6);
        let high_percent = s.between(0, 100);
        let dur_lo = s.between(1, 4);
        let dur_hi = s.between(dur_lo, 12);
        let slack_lo = s.between(1, 10);
        let slack_hi = s.between(slack_lo, 60);
        let cancel_percent = s.between(0, 20);
        let cancel_hi = s.between(1, 40);
        let first_lo = s.between(1, 30);
        let first_hi = s.between(first_lo, 200);
        let len_lo = s.between(1, 5);
        let len_hi = s.between(len_lo, 30);
        let sp_lo = s.between(1, 20);
        let sp_hi = s.between(sp_lo, 100);
        Self {
            revision: "panel/1".into(),
            servers,
            queue_capacity,
            max_attempts,
            deferral_cap,
            arrivals,
            gap: [1, gap_hi],
            high_percent,
            duration: [dur_lo, dur_hi],
            slack: [slack_lo, slack_hi],
            cancel_percent,
            cancel_after: [1, cancel_hi],
            outage_first: [first_lo, first_hi],
            outage_length: [len_lo, len_hi],
            outage_spacing: [sp_lo, sp_hi],
        }
    }

    /// Reads parameters from JSON in the shape of `canonical_json`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a field is missing or is not a whole number.
    pub fn from_json(v: &Value) -> Result<Self, Error> {
        let num = |v: &Value, what: &str| {
            v.as_u64()
                .ok_or_else(|| Error::Invalid(format!("{what} is not a whole number")))
        };
        let field = |name: &str| v.get(name).ok_or_else(|| Error::Invalid(format!("{name} is missing")));
        let two = |name: &str, v: &Value| -> Result<[u64; 2], Error> {
            let a = v
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| Error::Invalid(format!("{name} is not a pair")))?;
            Ok([num(&a[0], name)?, num(&a[1], name)?])
        };
        let outage = field("outage")?;
        let o = |name: &str| {
            outage
                .get(name)
                .ok_or_else(|| Error::Invalid(format!("outage.{name} is missing")))
        };
        Ok(Self {
            revision: field("revision")?.as_str().unwrap_or_default().to_owned(),
            servers: num(field("servers")?, "servers")?,
            queue_capacity: num(field("queueCapacity")?, "queueCapacity")?,
            max_attempts: num(field("maxAttempts")?, "maxAttempts")?,
            deferral_cap: num(field("deferralCap")?, "deferralCap")?,
            arrivals: num(field("arrivals")?, "arrivals")?,
            gap: two("gap", field("gap")?)?,
            high_percent: num(field("highPercent")?, "highPercent")?,
            duration: two("duration", field("duration")?)?,
            slack: two("slack", field("slack")?)?,
            cancel_percent: num(field("cancelPercent")?, "cancelPercent")?,
            cancel_after: two("cancelAfter", field("cancelAfter")?)?,
            outage_first: two("outage.first", o("first")?)?,
            outage_length: two("outage.length", o("length")?)?,
            outage_spacing: two("outage.spacing", o("spacing")?)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256_hex;

    /// The S0 pin of the parameter hash, which the JavaScript prototype produced and the oracle reproduced.
    #[test]
    fn the_parameter_hash_of_mixed_outage_7_is_the_s0_pin() {
        assert_eq!(
            sha256_hex(Params::mixed_outage_7().canonical_json().as_bytes()),
            "b7919ffc9ffcdaf97bb293b8f4506585e255cc56c1e50ad37b29cc0f5721e4de"
        );
    }

    #[test]
    fn a_panel_seed_derives_valid_parameters_and_the_same_ones_each_time() {
        for seed in 0..200 {
            let p = Params::panel(seed);
            assert_eq!(p, Params::panel(seed));
            p.validate(&Limits::default()).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        }
    }

    #[test]
    fn a_bad_range_is_refused_before_anything_runs() {
        let mut p = Params::mixed_outage_7();
        p.duration = [5, 4];
        assert!(matches!(p.validate(&Limits::default()), Err(Error::Invalid(_))));
        let mut p = Params::mixed_outage_7();
        p.arrivals = Limits::default().max_arrivals + 1;
        assert_eq!(p.validate(&Limits::default()), Err(Error::Limit("arrivals")));
        let mut p = Params::mixed_outage_7();
        p.servers = 5;
        assert!(matches!(p.validate(&Limits::default()), Err(Error::Invalid(_))));
    }
}
