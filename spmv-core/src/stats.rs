//! Latency statistics, which the browser probes report: the audit asks for p50, p95 and p99.

/// The summary of a list of timings in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Latency {
    pub n: usize,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

impl Latency {
    /// The summary of `samples`, by the nearest-rank rule: the quantile `q` of `n` samples is the `ceil(q·n)`th smallest. `None` for
    /// no samples. A sample that is not a number is not allowed, and sorts last.
    #[must_use]
    pub fn from_samples(samples: &[f64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let n = sorted.len();
        let rank = |q: f64| sorted[((q * n as f64).ceil() as usize).clamp(1, n) - 1];
        Some(Self {
            n,
            mean: sorted.iter().sum::<f64>() / n as f64,
            p50: rank(0.50),
            p95: rank(0.95),
            p99: rank(0.99),
            max: sorted[n - 1],
        })
    }

    /// As a JSON object, rounded to a microsecond.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let r = |v: f64| (v * 1000.0).round() / 1000.0;
        serde_json::json!({ "n": self.n, "mean": r(self.mean), "p50": r(self.p50), "p95": r(self.p95), "p99": r(self.p99), "max": r(self.max) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_quantiles_of_a_known_list() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        let l = Latency::from_samples(&v).expect("samples");
        assert_eq!((l.n, l.p50, l.p95, l.p99, l.max), (100, 50.0, 95.0, 99.0, 100.0));
        assert!((l.mean - 50.5).abs() < 1e-9);
    }

    #[test]
    fn one_sample_is_every_quantile_and_none_is_none() {
        let l = Latency::from_samples(&[7.5]).expect("a sample");
        assert_eq!((l.p50, l.p95, l.p99, l.max), (7.5, 7.5, 7.5, 7.5));
        assert!(Latency::from_samples(&[]).is_none());
    }

    #[test]
    fn the_order_of_the_samples_does_not_matter() {
        let a = Latency::from_samples(&[5.0, 1.0, 9.0, 3.0, 7.0]).expect("a");
        let b = Latency::from_samples(&[1.0, 3.0, 5.0, 7.0, 9.0]).expect("b");
        assert_eq!(a, b);
        assert_eq!(a.p50, 5.0);
    }
}
