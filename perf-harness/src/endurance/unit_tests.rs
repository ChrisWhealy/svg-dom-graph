use super::*;

fn point(cycle: u64, warmup: u64, live: f64, capacity: f64) -> SeriesPoint {
    SeriesPoint {
        cycle,
        measured: cycle > warmup,
        live_bytes: Some(live),
        capacity: Some(capacity),
    }
}

/// Cycles 1 to 60 with a warm-up of 20, leaving out the ones that ran a checkpoint: one in the warm-up and two measured.
fn gapped(live: impl Fn(u64) -> f64, capacity: impl Fn(u64) -> f64) -> Vec<SeriesPoint> {
    (1..=60)
        .filter(|c| ![10, 30, 60].contains(c))
        .map(|c| point(c, 20, live(c), capacity(c)))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With cycle 10 left out of the warm-up, the series has 19 warm-up points. Skipping 20 entries by position, as an earlier
/// version did, would also have dropped the first measured cycle, and the interval would have begun at cycle 22.
#[test]
fn the_measured_interval_begins_at_the_first_measured_cycle_whatever_was_left_out() -> Result<(), String> {
    let summary = summarize(&gapped(|_| 1000.0, |_| 65536.0));
    if summary.first_measured_cycle != Some(21)
        || summary.last_measured_cycle != Some(59)
        || summary.reference_cycle != Some(20)
    {
        return Err(format!("{summary:?}"));
    }
    // 21 to 60 is 40 cycles, less the two left out.
    if summary.measured_points != 38 {
        return Err(format!("{} measured points", summary.measured_points));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_series_that_does_not_move_has_no_net_change_no_slope_and_no_growth() -> Result<(), String> {
    let summary = summarize(&gapped(|_| 36_240.0, |_| 1_441_792.0));
    if summary.net_live_change != Some(0.0)
        || summary.live_slope_per_cycle != Some(0.0)
        || summary.grow_events != Some(0)
    {
        return Err(format!("{summary:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live bytes of 100 + 3 a cycle: the slope is 3 a cycle whatever cycles are missing, and the net change runs from the last
/// warm-up cycle (20) to the last measured one (59).
#[test]
fn the_slope_and_the_net_change_are_two_different_figures_and_both_use_the_real_cycles() -> Result<(), String> {
    let summary = summarize(&gapped(|c| 100.0 + 3.0 * c as f64, |_| 1.0));
    let slope = summary.live_slope_per_cycle.ok_or("no slope")?;
    if (slope - 3.0).abs() > 1e-9 || summary.net_live_change != Some(3.0 * 39.0) {
        return Err(format!("slope {slope}, net {:?}", summary.net_live_change));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One reading well above the rest moves the net change by its whole height and the slope by far less, which is why the two
/// are reported apart.
#[test]
fn one_stray_reading_moves_the_net_change_and_barely_moves_the_slope() -> Result<(), String> {
    let flat = summarize(&gapped(|_| 5000.0, |_| 1.0));
    let stray = summarize(&gapped(|c| if c == 59 { 5000.0 + 4096.0 } else { 5000.0 }, |_| 1.0));
    let (flat_slope, stray_slope) = (
        flat.live_slope_per_cycle.ok_or("no slope")?,
        stray.live_slope_per_cycle.ok_or("no slope")?,
    );
    if flat.net_live_change != Some(0.0) || stray.net_live_change != Some(4096.0) {
        return Err(format!("{:?} {:?}", flat.net_live_change, stray.net_live_change));
    }
    // 4096 bytes in one of 38 points leans the line by a few bytes a cycle.
    if flat_slope != 0.0 || !(stray_slope > 0.0 && stray_slope < 4096.0 / 38.0) {
        return Err(format!("slopes {flat_slope} and {stray_slope}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Capacity that grows during the warm-up is expected and is not counted. Growth after it is, once, wherever it happens.
#[test]
fn capacity_growth_counts_after_the_warm_up_and_not_during_it() -> Result<(), String> {
    let during = summarize(&gapped(|_| 1.0, |c| if c < 15 { 65536.0 } else { 131072.0 }));
    let after = summarize(&gapped(|_| 1.0, |c| if c < 45 { 131072.0 } else { 196608.0 }));
    // Growth at cycle 21, the very first measured one, is after the warm-up, and is not missed.
    let right_after = summarize(&gapped(|_| 1.0, |c| if c <= 20 { 65536.0 } else { 131072.0 }));
    if during.grow_events != Some(0) || after.grow_events != Some(1) || right_after.grow_events != Some(1) {
        return Err(format!(
            "{:?} {:?} {:?}",
            during.grow_events, after.grow_events, right_after.grow_events
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn with_no_warm_up_the_first_measured_cycle_is_the_reference() -> Result<(), String> {
    let points: Vec<SeriesPoint> = (1..=10).map(|c| point(c, 0, 100.0 + c as f64, 1.0)).collect();
    let summary = summarize(&points);
    if summary.reference_cycle != Some(1) || summary.net_live_change != Some(9.0) || summary.measured_points != 10 {
        return Err(format!("{summary:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn too_little_data_gives_no_figure_and_never_a_made_up_one() -> Result<(), String> {
    if summarize(&[])
        != (Summary {
            reference_cycle: None,
            first_measured_cycle: None,
            last_measured_cycle: None,
            measured_points: 0,
            net_live_change: None,
            live_slope_per_cycle: None,
            grow_events: None,
        })
    {
        return Err("an empty series was summarised".to_string());
    }
    // Two measured points are a net change, and are not a trend.
    let two = summarize(&[point(1, 0, 10.0, 1.0), point(2, 0, 20.0, 1.0)]);
    if two.net_live_change != Some(10.0) || two.live_slope_per_cycle.is_some() {
        return Err(format!("{two:?}"));
    }
    // A reading that is missing makes the figure that needs it missing too.
    let mut missing: Vec<SeriesPoint> = (1..=5).map(|c| point(c, 0, 1.0, 1.0)).collect();
    missing[2].live_bytes = None;
    if summarize(&missing).live_slope_per_cycle.is_some() {
        return Err("a slope was made from a series with a missing reading".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_series_that_does_not_move_passes_and_one_that_climbs_fails() -> Result<(), String> {
    let flat = judge(&summarize(&gapped(|_| 36_240.0, |_| 1.0)), 0.0);
    if !matches!(flat, Verdict::Pass(_)) {
        return Err(format!("{flat:?}"));
    }
    // 64 bytes a cycle over the 39 cycles from 20 to 59.
    let leaking = judge(&summarize(&gapped(|c| 1000.0 + 64.0 * c as f64, |_| 1.0)), 0.0);
    match &leaking {
        Verdict::Fail(text) if text.contains("2496") => Ok(()),
        other => Err(format!("{other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The allowance is on growth over the whole interval, and the verdict turns exactly on it.
#[test]
fn the_allowance_is_the_growth_tolerated_over_the_interval() -> Result<(), String> {
    let summary = summarize(&gapped(|c| 1000.0 + 64.0 * c as f64, |_| 1.0));
    let net = summary.net_live_change.ok_or("no net change")?;
    let at = |allowance: f64| judge(&summary, allowance);
    if !matches!(at(net), Verdict::Pass(_))
        || !matches!(at(net - 1.0), Verdict::Fail(_))
        || !matches!(at(net * 100.0), Verdict::Pass(_))
    {
        return Err(format!("net {net}: {:?} {:?} {:?}", at(net), at(net - 1.0), at(net * 100.0)));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Steady growth with a low last reading has a small net change, and only the trend shows it. This is why there are two measures.
#[test]
fn the_trend_catches_growth_that_a_low_last_reading_hides_from_the_net_change() -> Result<(), String> {
    let climbing_then_dropping =
        summarize(&gapped(|c| if c == 59 { 1000.0 } else { 1000.0 + 50.0 * c as f64 }, |_| 1.0));
    let net = climbing_then_dropping.net_live_change.ok_or("no net change")?;
    let verdict = judge(&climbing_then_dropping, 0.0);
    // The last reading is below the reference, so the net change is not positive. The slope is.
    if net > 0.0 || !matches!(&verdict, Verdict::Fail(text) if text.contains("a cycle")) {
        return Err(format!("net {net}, {verdict:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn too_little_to_judge_is_inconclusive_and_never_a_pass() -> Result<(), String> {
    let two = summarize(&[point(1, 0, 10.0, 1.0), point(2, 0, 10.0, 1.0)]);
    let none = summarize(&[]);
    let mut missing: Vec<SeriesPoint> = (1..=6).map(|c| point(c, 0, 1.0, 1.0)).collect();
    missing[3].live_bytes = None;
    for (label, summary) in [("two points", two), ("none", none), ("a missing reading", summarize(&missing))] {
        if !matches!(judge(&summary, 1e9), Verdict::Inconclusive(_)) {
            return Err(format!("{label} was judged"));
        }
    }
    Ok(())
}
