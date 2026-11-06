use super::*;
use crate::prng::SplitMix64;

/// Repetitions of observations around `centre`, each with its own small shift (the run-to-run part) and noise (the within-run
/// part), all from one seed so that a test is the same every time.
fn collection(seed: u64, repetitions: usize, centre: f64, shift: f64, noise: f64) -> Vec<Vec<f64>> {
    let mut rng = SplitMix64::new(seed);
    let unit = |rng: &mut SplitMix64| (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
    (0..repetitions)
        .map(|_| {
            let run = centre * (1.0 + shift * unit(&mut rng) * 2.0);
            (0..200).map(|_| run * (1.0 + noise * unit(&mut rng) * 2.0)).collect()
        })
        .collect()
}

fn settings() -> Settings {
    Settings {
        resamples: 1500,
        ..Settings::default()
    }
}

fn verdict_of(baseline: &[Vec<f64>], candidate: &[Vec<f64>]) -> Comparison {
    compare("m", "B", Statistic::P50, baseline, candidate, &settings())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two collections drawn from the same distribution are not a change. If this reported one, every run of CI would be a false alarm.
#[test]
fn the_same_distribution_is_no_material_change() -> Result<(), String> {
    for seed in 1..=8 {
        let a = collection(seed, 5, 50.0, 0.01, 0.05);
        let b = collection(seed + 100, 5, 50.0, 0.01, 0.05);
        let c = verdict_of(&a, &b);
        if c.verdict != Verdict::NoMaterialChange {
            return Err(format!("seed {seed}: {:?} {:?} {:?}", c.verdict, c.ratio_interval, c.reason));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A candidate 30% slower is a regression against a tolerance of 10%, and one 30% faster is an improvement.
#[test]
fn a_clear_slowdown_is_a_regression_and_a_clear_speedup_an_improvement() -> Result<(), String> {
    let base = collection(1, 5, 50.0, 0.01, 0.05);
    let slow = verdict_of(&base, &collection(2, 5, 65.0, 0.01, 0.05));
    if slow.verdict != Verdict::Regression {
        return Err(format!("{:?} {:?}", slow.verdict, slow.ratio_interval));
    }
    let (low, high) = slow.ratio_interval.ok_or("no interval")?;
    if !(low > 1.1 && low < 1.3 && high > 1.3 && high < 1.5) {
        return Err(format!("the interval of a ratio of 1.3 was {low} to {high}"));
    }
    let fast = verdict_of(&base, &collection(3, 5, 35.0, 0.01, 0.05));
    if fast.verdict != Verdict::Improvement {
        return Err(format!("{:?} {:?}", fast.verdict, fast.ratio_interval));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A change that is real but smaller than the tolerance is not a regression, however certain it is.
#[test]
fn a_change_inside_the_tolerance_is_not_a_regression() -> Result<(), String> {
    let base = collection(1, 5, 50.0, 0.005, 0.02);
    let c = verdict_of(&base, &collection(2, 5, 52.5, 0.005, 0.02));
    if c.verdict != Verdict::NoMaterialChange {
        return Err(format!("a 5% change against 10% was {:?} {:?}", c.verdict, c.ratio_interval));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// When the runs disagree among themselves as much as the change is worth, the interval is too wide for a verdict, which is
/// said as inconclusive and never as a pass.
#[test]
fn runs_that_disagree_give_an_inconclusive_result() -> Result<(), String> {
    let base = collection(1, 5, 50.0, 0.30, 0.05);
    let c = verdict_of(&base, &collection(2, 5, 56.0, 0.30, 0.05));
    if c.verdict != Verdict::Inconclusive || c.reason.is_none() {
        return Err(format!("{:?} {:?}", c.verdict, c.ratio_interval));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With the three repetitions of perf-smoke there is no interval and no verdict, even when the change is huge. The figures are
/// still given, with a ratio, so a reader can see what was measured.
#[test]
fn too_few_repetitions_give_a_diagnostic_and_no_verdict() -> Result<(), String> {
    let c = verdict_of(&collection(1, 3, 50.0, 0.01, 0.05), &collection(2, 3, 150.0, 0.01, 0.05));
    if c.verdict != Verdict::Diagnostic || c.ratio_interval.is_some() || c.baseline.is_none() {
        return Err(format!("{:?} {:?}", c.verdict, c.ratio_interval));
    }
    let ratio = c.ratio.ok_or("no ratio")?;
    if !(2.8..3.2).contains(&ratio) {
        return Err(format!("ratio {ratio}"));
    }
    if !c.reason.as_deref().unwrap_or("").contains("at least 5") {
        return Err(format!("{:?}", c.reason));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A percentile is only shown from enough observations: 30 for the median, 200 for p95 and 1000 for p99.
#[test]
fn a_percentile_without_enough_observations_is_a_diagnostic() -> Result<(), String> {
    let a = collection(1, 5, 50.0, 0.01, 0.05);
    let b = collection(2, 5, 65.0, 0.01, 0.05);
    let p95 = compare("m", "B", Statistic::P95, &a, &b, &settings());
    // 200 observations is exactly enough for p95.
    if p95.verdict == Verdict::Diagnostic {
        return Err(format!("p95 of 200 observations: {:?}", p95.reason));
    }
    let p99 = compare("m", "B", Statistic::P99, &a, &b, &settings());
    if p99.verdict != Verdict::Diagnostic || !p99.reason.as_deref().unwrap_or("").contains("1000") {
        return Err(format!("p99 of 200 observations: {:?} {:?}", p99.verdict, p99.reason));
    }
    let few: Vec<Vec<f64>> = a.iter().map(|r| r[..20].to_vec()).collect();
    let p50 = verdict_of(&few, &b);
    if p50.verdict != Verdict::Diagnostic {
        return Err(format!("p50 of 20 observations: {:?}", p50.verdict));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same seed gives the same interval to the last bit, and another seed gives a nearby one.
#[test]
fn the_interval_is_reproducible_from_the_seed() -> Result<(), String> {
    let a = collection(1, 5, 50.0, 0.02, 0.05);
    let b = collection(2, 5, 60.0, 0.02, 0.05);
    let first = verdict_of(&a, &b).ratio_interval;
    if first != verdict_of(&a, &b).ratio_interval {
        return Err("the same seed gave two intervals".to_string());
    }
    let other = compare("m", "B", Statistic::P50, &a, &b, &Settings { seed: 8, ..settings() });
    let ((l1, h1), (l2, h2)) = (first.ok_or("none")?, other.ratio_interval.ok_or("none")?);
    if first == other.ratio_interval || (l1 - l2).abs() > 0.05 || (h1 - h2).abs() > 0.05 {
        return Err(format!("{l1} {h1} against {l2} {h2}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The interval of a ratio is the interval a hand calculation gives for data with no spread at all: a point. A candidate that is
/// exactly 1.5 times the baseline in every observation has the interval 1.5 to 1.5.
#[test]
fn data_with_no_spread_give_a_point_interval() -> Result<(), String> {
    let a: Vec<Vec<f64>> = (0..5).map(|_| vec![10.0; 40]).collect();
    let b: Vec<Vec<f64>> = (0..5).map(|_| vec![15.0; 40]).collect();
    let c = verdict_of(&a, &b);
    let (low, high) = c.ratio_interval.ok_or("no interval")?;
    if (low - 1.5).abs() > 1e-12 || (high - 1.5).abs() > 1e-12 || c.verdict != Verdict::Regression {
        return Err(format!("{low} {high} {:?}", c.verdict));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The between-run and within-run figures are what they say, worked by hand for two runs of known values.
#[test]
fn the_sides_describe_the_runs() -> Result<(), String> {
    let a = vec![
        (1..=40).map(f64::from).collect::<Vec<_>>(),
        (11..=50).map(f64::from).collect::<Vec<_>>(),
    ];
    let c = compare(
        "m",
        "B",
        Statistic::P50,
        &a,
        &a,
        &Settings {
            min_repetitions: 2,
            ..settings()
        },
    );
    let side = c.baseline.ok_or("no side")?;
    // Nearest rank: the median of 1..=40 is the 20th value, 20, and of 11..=50 it is 30.
    if side.figures != vec![20.0, 30.0] || side.median != 20.0 || side.range != (20.0, 30.0) {
        return Err(format!("{:?}", side));
    }
    let want = (50.0f64).sqrt();
    if (side.stddev_between - want).abs() > 1e-9 || side.samples_per_repetition != vec![40, 40] {
        return Err(format!("stddev {} against {want}", side.stddev_between));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A metric one side lacks cannot be compared, and says so.
#[test]
fn a_side_with_no_observations_is_not_comparable() -> Result<(), String> {
    let a = collection(1, 5, 50.0, 0.01, 0.05);
    let c = compare("m", "B", Statistic::P50, &a, &[], &settings());
    if c.verdict != Verdict::NotComparable || c.baseline.is_some() || c.ratio.is_some() {
        return Err(format!("{:?}", c.verdict));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An entry carries the fields the schema asks for its verdict, and only those: a diagnostic has no interval, and a
/// not-comparable one has no figures.
#[test]
fn an_entry_has_the_fields_its_verdict_needs() -> Result<(), String> {
    let a = collection(1, 5, 50.0, 0.01, 0.05);
    let full = to_json(&verdict_of(&a, &collection(2, 5, 65.0, 0.01, 0.05)), 0.95);
    for key in ["baseline", "candidate", "ratio", "ratio_interval"] {
        if full.get(key).is_none() {
            return Err(format!("a regression lacks {key}"));
        }
    }
    let diagnostic = to_json(&verdict_of(&a[..3], &a[..3]), 0.95);
    if diagnostic.get("ratio_interval").is_some() || diagnostic.get("reason").is_none() {
        return Err(format!("{diagnostic}"));
    }
    let none = to_json(&not_comparable("m", "B", Statistic::P50, "another browser"), 0.95);
    if none.get("baseline").is_some() || none.get("ratio").is_some() || none["reason"] != "another browser" {
        return Err(format!("{none}"));
    }
    Ok(())
}
