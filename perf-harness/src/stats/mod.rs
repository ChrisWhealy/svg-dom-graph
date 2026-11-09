//! Statistics over the observations of one run.
//!
//! Everything here describes a single run. Between-run variation and confidence intervals need several runs, so they
//! belong to the comparison, not to this module. Percentiles are nearest-rank, with no interpolation, so every one of
//! them is a value that was observed.

/// A percentile is only reported when there are enough observations for it to mean something.
pub const MIN_SAMPLES_P95: usize = 200;
pub const MIN_SAMPLES_P99: usize = 1000;

/// How many timer steps an operation, or a batch of them, has to take before one observation is worth keeping.
pub const MIN_STEPS_PER_OBSERVATION: f64 = 50.0;
/// A cap on a batch, so that a very fast operation cannot make one observation take seconds.
pub const MAX_BATCH: u32 = 100_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub min: f64,
    /// The median. Always present, since one observation or more has a median.
    pub p50: f64,
    /// Present only with at least [`MIN_SAMPLES_P95`] observations.
    pub p95: Option<f64>,
    /// Present only with at least [`MIN_SAMPLES_P99`] observations.
    pub p99: Option<f64>,
    pub max: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq)]
pub struct Within {
    pub mean: f64,
    /// The sample standard deviation, which is zero for a single observation.
    pub stddev: f64,
    pub iqr: f64,
    /// The Tukey fences, 1.5 interquartile ranges beyond the quartiles.
    pub tukey_low: f64,
    pub tukey_high: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq)]
pub struct Description {
    pub samples: usize,
    pub summary: Summary,
    pub within: Within,
    /// How many observations lie outside the fences. They are counted and never removed.
    pub outliers_flagged: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nearest-rank percentile `p` (0 to 100) of `sorted`, which must not be empty and must be in ascending order.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Describes `samples`, or says there is nothing to describe.
///
/// A NaN is not an observation, so a set that has one is refused with `None` and never silently reordered.
pub fn describe(samples: &[f64]) -> Option<Description> {
    if samples.is_empty() || samples.iter().any(|value| value.is_nan()) {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();

    let mean = sorted.iter().sum::<f64>() / n as f64;
    let stddev = if n > 1 {
        (sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        0.0
    };
    let (q1, q3) = (percentile(&sorted, 25.0), percentile(&sorted, 75.0));
    let iqr = q3 - q1;
    let (tukey_low, tukey_high) = (q1 - 1.5 * iqr, q3 + 1.5 * iqr);

    Some(Description {
        samples: n,
        summary: Summary {
            min: sorted[0],
            p50: percentile(&sorted, 50.0),
            p95: (n >= MIN_SAMPLES_P95).then(|| percentile(&sorted, 95.0)),
            p99: (n >= MIN_SAMPLES_P99).then(|| percentile(&sorted, 99.0)),
            max: sorted[n - 1],
        },
        within: Within {
            mean,
            stddev,
            iqr,
            tukey_low,
            tukey_high,
        },
        outliers_flagged: sorted.iter().filter(|v| **v < tukey_low || **v > tukey_high).count(),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The slope of the least-squares line through `points`, each an `(x, y)` pair.
///
/// It uses the real `x` of every point, so points that are not evenly spaced, such as cycles with gaps where some were
/// left out, are weighted by where they are and not by how many come before them. It is `None` for fewer than two
/// points, when every `x` is the same, or when a value is not a number.
pub fn slope(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 || points.iter().any(|(x, y)| x.is_nan() || y.is_nan()) {
        return None;
    }
    let n = points.len() as f64;
    let (mean_x, mean_y) = (
        points.iter().map(|p| p.0).sum::<f64>() / n,
        points.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let spread: f64 = points.iter().map(|p| (p.0 - mean_x).powi(2)).sum();
    if spread == 0.0 {
        return None;
    }
    Some(points.iter().map(|p| (p.0 - mean_x) * (p.1 - mean_y)).sum::<f64>() / spread)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many operations to time together so that one observation spans enough timer steps.
///
/// An operation that is already long enough is timed alone, which is a batch of one. `single_us` is a typical time for
/// one operation, and `resolution_us` is the smallest step the timer makes.
pub fn batch_size(resolution_us: f64, single_us: f64) -> u32 {
    if resolution_us.is_nan() || single_us.is_nan() || resolution_us <= 0.0 || single_us <= 0.0 {
        return 1;
    }
    let wanted = (MIN_STEPS_PER_OBSERVATION * resolution_us / single_us).ceil();
    (wanted.max(1.0) as u32).min(MAX_BATCH)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
