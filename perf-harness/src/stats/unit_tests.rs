use super::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The nearest-rank method: the p-th percentile of n values is the value at rank ceil(p n / 100). For the ten values 1 to 10
/// that makes the median the 5th, the 90th percentile the 9th and the 25th percentile the 3rd.
#[test]
fn percentiles_are_nearest_rank() -> Result<(), String> {
    let sorted: Vec<f64> = (1..=10).map(f64::from).collect();
    for (p, want) in [(50.0, 5.0), (90.0, 9.0), (25.0, 3.0), (75.0, 8.0), (100.0, 10.0), (1.0, 1.0)] {
        let got = percentile(&sorted, p);
        if got != want {
            return Err(format!("p{p} of 1..=10 was {got}, expected {want}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_description_has_the_hand_worked_figures() -> Result<(), String> {
    // 2, 4, 4, 4, 5, 5, 7, 9: mean 5, and a sample variance of 32 / 7.
    let d = describe(&[9.0, 4.0, 2.0, 5.0, 4.0, 7.0, 4.0, 5.0]).ok_or("no description")?;
    if d.samples != 8 || d.summary.min != 2.0 || d.summary.max != 9.0 || d.summary.p50 != 4.0 {
        return Err(format!("{d:?}"));
    }
    if !close(d.within.mean, 5.0) || !close(d.within.stddev, (32.0f64 / 7.0).sqrt()) {
        return Err(format!("mean {} and stddev {}", d.within.mean, d.within.stddev));
    }
    // Quartiles by nearest rank: rank 2 is 4 and rank 6 is 5, so the interquartile range is 1 and the fences are 2.5 and 6.5.
    if !close(d.within.iqr, 1.0) || !close(d.within.tukey_low, 2.5) || !close(d.within.tukey_high, 6.5) {
        return Err(format!("{:?}", d.within));
    }
    // 2 is below 2.5 and 7 and 9 are above 6.5.
    if d.outliers_flagged != 3 {
        return Err(format!("{} outliers", d.outliers_flagged));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_high_percentile_needs_enough_observations() -> Result<(), String> {
    let few: Vec<f64> = (0..199).map(f64::from).collect();
    let some: Vec<f64> = (0..200).map(f64::from).collect();
    let many: Vec<f64> = (0..1000).map(f64::from).collect();
    let present = |samples: &[f64]| describe(samples).map(|d| (d.summary.p95.is_some(), d.summary.p99.is_some()));
    if present(&few) != Some((false, false))
        || present(&some) != Some((true, false))
        || present(&many) != Some((true, true))
    {
        return Err(format!("{:?} {:?} {:?}", present(&few), present(&some), present(&many)));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn one_observation_describes_itself_and_nothing_else_is_described() -> Result<(), String> {
    let d = describe(&[3.5]).ok_or("no description")?;
    if d.summary.min != 3.5
        || d.summary.p50 != 3.5
        || d.summary.max != 3.5
        || d.within.stddev != 0.0
        || d.outliers_flagged != 0
    {
        return Err(format!("{d:?}"));
    }
    if describe(&[]).is_some() || describe(&[1.0, f64::NAN]).is_some() {
        return Err("an empty set or one with a NaN was described".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn outliers_are_flagged_and_never_removed() -> Result<(), String> {
    let mut samples = vec![10.0; 99];
    samples.push(10_000.0);
    let d = describe(&samples).ok_or("no description")?;
    if d.outliers_flagged != 1 || d.summary.max != 10_000.0 || d.samples != 100 {
        return Err(format!("{d:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_batch_makes_an_observation_span_enough_timer_steps() -> Result<(), String> {
    // A 5 us timer wants 250 us per observation. An operation of 25 us needs 10 together.
    if batch_size(5.0, 25.0) != 10 {
        return Err(format!("{}", batch_size(5.0, 25.0)));
    }
    // One that is long enough is timed alone, and one that is nearly instant is capped.
    if batch_size(5.0, 5_000.0) != 1 || batch_size(5.0, 1e-12) != MAX_BATCH {
        return Err("a long operation was batched, or a short one was not capped".to_string());
    }
    if batch_size(0.0, 10.0) != 1 || batch_size(5.0, 0.0) != 1 || batch_size(f64::NAN, 1.0) != 1 {
        return Err("a bad timer reading was not answered with a batch of one".to_string());
    }
    Ok(())
}
