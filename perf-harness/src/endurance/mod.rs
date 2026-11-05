//! Reads an endurance series: whether live bytes keep growing, and whether memory capacity is reused, once the warm-up is over.
//!
//! The series has one point for each cycle that was sampled after its teardown. Cycles that ran a checkpoint are left out,
//! since the checking uses memory of its own, so the series has gaps. Each point therefore carries the number of its cycle and
//! whether that cycle was in the measured part, and every boundary here is drawn from those, never from where a point sits in
//! the list. Counting entries would over-skip whenever a cycle of the warm-up was one of those left out.
//!
//! Two figures are kept apart, because they answer different questions:
//!
//! * The net change is the live bytes at the last measured cycle less those at the reference, which is the last warm-up point
//!   or, with no warm-up, the first measured one. It says how far live memory moved, and one stray reading can move it.
//! * The slope is the least-squares line through the measured points, against their cycle numbers. It says how fast live
//!   memory moves each cycle, and needs at least [`MIN_POINTS_FOR_TREND`] points to be called a trend at all.

use crate::stats::slope;

/// A slope through fewer points than this is not reported, since two points always lie on a line.
pub const MIN_POINTS_FOR_TREND: usize = 3;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was counted after one teardown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeriesPoint {
    /// The number of the cycle, counting from one. A cycle left out of the series leaves a gap in these.
    pub cycle: u64,
    /// Whether the cycle was in the measured part of the run and not the warm-up.
    pub measured: bool,
    pub live_bytes: Option<f64>,
    pub capacity: Option<f64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// The cycle that the net change is measured from.
    pub reference_cycle: Option<u64>,
    pub first_measured_cycle: Option<u64>,
    pub last_measured_cycle: Option<u64>,
    pub measured_points: usize,
    pub net_live_change: Option<f64>,
    pub live_slope_per_cycle: Option<f64>,
    /// How many times the capacity changed between one sampled cycle and the next, from the reference onwards.
    pub grow_events: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Summarises `points`, which are in the order of their cycles.
pub fn summarize(points: &[SeriesPoint]) -> Summary {
    let measured: Vec<&SeriesPoint> = points.iter().filter(|p| p.measured).collect();
    let last_warmup = points.iter().rfind(|p| !p.measured);
    // With no warm-up point, the first measured one is the reference, and is already in the measured list.
    let reference = last_warmup.or(measured.first().copied());

    let net_live_change = match (reference, measured.last()) {
        (Some(reference), Some(last)) if reference.cycle != last.cycle => {
            reference.live_bytes.zip(last.live_bytes).map(|(from, to)| to - from)
        },
        _ => None,
    };

    let live_slope_per_cycle = if measured.len() >= MIN_POINTS_FOR_TREND {
        let pairs: Option<Vec<(f64, f64)>> = measured
            .iter()
            .map(|p| p.live_bytes.map(|live| (p.cycle as f64, live)))
            .collect();
        pairs.and_then(|pairs| slope(&pairs))
    } else {
        None
    };

    // The reference leads the sequence when it is a warm-up point, so that growth right after the warm-up is not missed.
    let sequence: Vec<&SeriesPoint> = last_warmup.into_iter().chain(measured.iter().copied()).collect();
    let capacities: Option<Vec<f64>> = sequence.iter().map(|p| p.capacity).collect();
    let grow_events = capacities
        .filter(|c| c.len() >= 2)
        .map(|c| c.windows(2).filter(|pair| pair[0] != pair[1]).count());

    Summary {
        reference_cycle: reference.map(|p| p.cycle),
        first_measured_cycle: measured.first().map(|p| p.cycle),
        last_measured_cycle: measured.last().map(|p| p.cycle),
        measured_points: measured.len(),
        net_live_change,
        live_slope_per_cycle,
        grow_events,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
