//! Compares a candidate collection of runs with a baseline one, one metric at a time, and says whether it got slower.
//!
//! The unit is the repetition, because the observations inside one run share a page, a heap and a JIT and so are not independent
//! of each other. A run gives one figure for a metric, a percentile of its own observations. A collection gives the median of its
//! runs' figures. The statistic is the ratio of the candidate's to the baseline's.
//!
//! The interval on that ratio is a hierarchical bootstrap. Each resample draws repetitions with replacement from each side, then
//! draws the observations of each chosen repetition with replacement, and computes the ratio again. The interval is the middle
//! `confidence` of those ratios. Resampling only the repetitions would treat each run's figure as exact, and resampling only
//! the observations would treat the runs as one, so both levels are resampled.
//!
//! The verdict follows from the interval and a ratio tolerance `t`. It is a regression if the interval lies wholly above
//! `1 + t`, an improvement if it lies wholly below `1 / (1 + t)`, no material change if it lies wholly inside both bounds, and
//! inconclusive otherwise. Inconclusive is never a pass. With too few repetitions no verdict is given at all, and the result is
//! labelled diagnostic.
//!
//! Nothing here reads a file or a clock. A seed fixes every draw, so the same figures always give the same interval.

pub mod report;

use crate::prng::SplitMix64;
use crate::stats::{MIN_SAMPLES_P95, MIN_SAMPLES_P99, describe, percentile};
use serde_json::{Value, json};

/// The fewest observations a repetition needs before its median is shown.
pub const MIN_SAMPLES_P50: usize = 30;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The percentile of a repetition's observations that is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Statistic {
    P50,
    P95,
    P99,
}

impl Statistic {
    pub const ALL: [Self; 3] = [Self::P50, Self::P95, Self::P99];

    pub fn name(self) -> &'static str {
        match self {
            Self::P50 => "p50",
            Self::P95 => "p95",
            Self::P99 => "p99",
        }
    }

    fn rank(self) -> f64 {
        match self {
            Self::P50 => 50.0,
            Self::P95 => 95.0,
            Self::P99 => 99.0,
        }
    }

    /// How many observations a repetition needs before this percentile is shown.
    pub fn min_samples(self) -> usize {
        match self {
            Self::P50 => MIN_SAMPLES_P50,
            Self::P95 => MIN_SAMPLES_P95,
            Self::P99 => MIN_SAMPLES_P99,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a comparison is made. Every figure in it is recorded in the report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub resamples: u32,
    pub seed: u64,
    /// The ratio of candidate to baseline that is too small to matter, as a fraction: 0.1 is ten per cent.
    pub tolerance: f64,
    pub confidence: f64,
    /// The fewest repetitions on either side for a verdict.
    pub min_repetitions: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            resamples: 4000,
            seed: 7,
            tolerance: 0.1,
            confidence: 0.95,
            min_repetitions: 5,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Regression,
    Improvement,
    NoMaterialChange,
    Inconclusive,
    Diagnostic,
    NotComparable,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Self::Regression => "regression",
            Self::Improvement => "improvement",
            Self::NoMaterialChange => "no_material_change",
            Self::Inconclusive => "inconclusive",
            Self::Diagnostic => "diagnostic",
            Self::NotComparable => "not_comparable",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One side of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Side {
    pub samples_per_repetition: Vec<usize>,
    /// Each repetition's own figure.
    pub figures: Vec<f64>,
    /// The median of the repetitions' figures.
    pub median: f64,
    /// The interval of that median over the bootstrap, when there was one.
    pub interval: Option<(f64, f64)>,
    pub stddev_between: f64,
    pub range: (f64, f64),
    pub median_within_stddev: f64,
    pub median_within_iqr: f64,
    pub outliers_flagged: usize,
}

/// The comparison of one statistic of one metric.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub metric: String,
    pub layer: String,
    pub statistic: Statistic,
    pub verdict: Verdict,
    pub reason: Option<String>,
    pub baseline: Option<Side>,
    pub candidate: Option<Side>,
    pub ratio: Option<f64>,
    pub ratio_interval: Option<(f64, f64)>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn sorted(values: &[f64]) -> Vec<f64> {
    let mut copy = values.to_vec();
    copy.sort_by(f64::total_cmp);
    copy
}

fn figure(observations: &[f64], statistic: Statistic) -> f64 {
    percentile(&sorted(observations), statistic.rank())
}

/// The median of the repetitions' figures.
fn median_of(figures: &[f64]) -> f64 {
    percentile(&sorted(figures), 50.0)
}

fn quantile(sorted_values: &[f64], fraction: f64) -> f64 {
    percentile(sorted_values, fraction * 100.0)
}

/// One resample of one side: repetitions drawn with replacement, then each one's observations.
fn resample_side(repetitions: &[Vec<f64>], statistic: Statistic, rng: &mut SplitMix64, scratch: &mut Vec<f64>) -> f64 {
    let n = repetitions.len() as u64;
    let figures: Vec<f64> = (0..repetitions.len())
        .map(|_| {
            let chosen = &repetitions[rng.below(n) as usize];
            scratch.clear();
            let m = chosen.len() as u64;
            scratch.extend((0..chosen.len()).map(|_| chosen[rng.below(m) as usize]));
            figure(scratch, statistic)
        })
        .collect();
    median_of(&figures)
}

fn side_of(repetitions: &[Vec<f64>], statistic: Statistic, interval: Option<(f64, f64)>) -> Side {
    let figures: Vec<f64> = repetitions.iter().map(|r| figure(r, statistic)).collect();
    let described: Vec<_> = repetitions.iter().filter_map(|r| describe(r)).collect();
    let n = figures.len() as f64;
    let mean = figures.iter().sum::<f64>() / n;
    let stddev_between = if figures.len() > 1 {
        (figures.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else {
        0.0
    };
    let ordered = sorted(&figures);
    Side {
        samples_per_repetition: repetitions.iter().map(Vec::len).collect(),
        median: median_of(&figures),
        interval,
        stddev_between,
        range: (ordered[0], ordered[ordered.len() - 1]),
        median_within_stddev: median_of(&described.iter().map(|d| d.within.stddev).collect::<Vec<_>>()),
        median_within_iqr: median_of(&described.iter().map(|d| d.within.iqr).collect::<Vec<_>>()),
        outliers_flagged: described.iter().map(|d| d.outliers_flagged).sum(),
        figures,
    }
}

fn blank(metric: &str, layer: &str, statistic: Statistic, verdict: Verdict, reason: String) -> Comparison {
    Comparison {
        metric: metric.to_string(),
        layer: layer.to_string(),
        statistic,
        verdict,
        reason: Some(reason),
        baseline: None,
        candidate: None,
        ratio: None,
        ratio_interval: None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A comparison that cannot be made because the two sides were not measured alike.
pub fn not_comparable(metric: &str, layer: &str, statistic: Statistic, reason: &str) -> Comparison {
    blank(metric, layer, statistic, Verdict::NotComparable, reason.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Compares `statistic` of one metric. Each element of `baseline` and `candidate` is the observations of one repetition.
///
/// Gives a diagnostic result, with no interval and no verdict, when either side has fewer repetitions than
/// `settings.min_repetitions` or a repetition has fewer observations than the statistic needs.
pub fn compare(
    metric: &str,
    layer: &str,
    statistic: Statistic,
    baseline: &[Vec<f64>],
    candidate: &[Vec<f64>],
    settings: &Settings,
) -> Comparison {
    if baseline.is_empty() || candidate.is_empty() || baseline.iter().chain(candidate).any(Vec::is_empty) {
        return blank(
            metric,
            layer,
            statistic,
            Verdict::NotComparable,
            "a side has no observations of this metric".to_string(),
        );
    }
    let mut result = Comparison {
        metric: metric.to_string(),
        layer: layer.to_string(),
        statistic,
        verdict: Verdict::Diagnostic,
        reason: None,
        baseline: Some(side_of(baseline, statistic, None)),
        candidate: Some(side_of(candidate, statistic, None)),
        ratio: None,
        ratio_interval: None,
    };
    let base_median = result.baseline.as_ref().map_or(0.0, |s| s.median);
    let cand_median = result.candidate.as_ref().map_or(0.0, |s| s.median);
    if base_median > 0.0 {
        result.ratio = Some(cand_median / base_median);
    }

    let thin = baseline.iter().chain(candidate).map(Vec::len).min().unwrap_or(0);
    if thin < statistic.min_samples() {
        result.reason = Some(format!(
            "a repetition has {thin} observations, and {} needs at least {}",
            statistic.name(),
            statistic.min_samples()
        ));
        return result;
    }
    if baseline.len() < settings.min_repetitions || candidate.len() < settings.min_repetitions {
        result.reason = Some(format!(
            "{} baseline and {} candidate repetitions, and a verdict needs at least {} on each side. The figures are shown and no interval or verdict is given",
            baseline.len(),
            candidate.len(),
            settings.min_repetitions
        ));
        return result;
    }
    if base_median <= 0.0 {
        result.reason = Some("the baseline's median is not above zero, so there is no ratio".to_string());
        result.verdict = Verdict::Inconclusive;
        return result;
    }

    // One generator for the whole comparison, so that the draws of the two sides are tied to the seed and to nothing else.
    let mut rng = SplitMix64::derive(settings.seed, 0x5245_4752);
    let mut scratch = Vec::new();
    let (mut ratios, mut base_draws, mut cand_draws) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..settings.resamples {
        let b = resample_side(baseline, statistic, &mut rng, &mut scratch);
        let c = resample_side(candidate, statistic, &mut rng, &mut scratch);
        base_draws.push(b);
        cand_draws.push(c);
        if b > 0.0 {
            ratios.push(c / b);
        }
    }
    let tail = (1.0 - settings.confidence) / 2.0;
    let interval_of = |draws: &mut Vec<f64>| {
        draws.sort_by(f64::total_cmp);
        (quantile(draws, tail), quantile(draws, 1.0 - tail))
    };
    if ratios.len() < (settings.resamples as usize) / 2 {
        result.reason =
            Some("most resamples of the baseline had a median of zero, so the ratio has no interval".to_string());
        result.verdict = Verdict::Inconclusive;
        return result;
    }
    let (low, high) = interval_of(&mut ratios);
    result.baseline = result.baseline.take().map(|s| Side {
        interval: Some(interval_of(&mut base_draws)),
        ..s
    });
    result.candidate = result.candidate.take().map(|s| Side {
        interval: Some(interval_of(&mut cand_draws)),
        ..s
    });
    result.ratio_interval = Some((low, high));

    let (upper, lower) = (1.0 + settings.tolerance, 1.0 / (1.0 + settings.tolerance));
    (result.verdict, result.reason) = if low > upper {
        (Verdict::Regression, None)
    } else if high < lower {
        (Verdict::Improvement, None)
    } else if low >= lower && high <= upper {
        (Verdict::NoMaterialChange, None)
    } else {
        (
            Verdict::Inconclusive,
            Some(format!(
                "the interval {low:.3} to {high:.3} crosses a bound ({lower:.3} or {upper:.3}), so the data neither show a change nor rule one out"
            )),
        )
    };
    result
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn interval_json(interval: (f64, f64), confidence: f64) -> Value {
    json!({"low": interval.0, "high": interval.1, "confidence": confidence})
}

fn side_json(side: &Side, confidence: f64) -> Value {
    let mut value = json!({
        "repetitions": side.figures.len(),
        "samples_per_repetition": side.samples_per_repetition,
        "median": side.median,
        "between_run": {"stddev": side.stddev_between, "range": [side.range.0, side.range.1]},
        "within_run": {"median_stddev": side.median_within_stddev, "median_iqr": side.median_within_iqr},
        "outliers_flagged": side.outliers_flagged,
    });
    if let Some(interval) = side.interval {
        value["statistic_interval"] = interval_json(interval, confidence);
    }
    value
}

/// A comparison as an entry of the report that `comparison.schema.json` describes.
pub fn to_json(comparison: &Comparison, confidence: f64) -> Value {
    let mut value = json!({
        "metric": comparison.metric,
        "layer": comparison.layer,
        "statistic": comparison.statistic.name(),
        "verdict": comparison.verdict.name(),
    });
    if let Some(side) = &comparison.baseline {
        value["baseline"] = side_json(side, confidence);
    }
    if let Some(side) = &comparison.candidate {
        value["candidate"] = side_json(side, confidence);
    }
    if let Some(ratio) = comparison.ratio {
        value["ratio"] = json!(ratio);
    }
    if let Some(interval) = comparison.ratio_interval {
        value["ratio_interval"] = interval_json(interval, confidence);
    }
    if let Some(reason) = &comparison.reason {
        value["reason"] = json!(reason);
    }
    value
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
