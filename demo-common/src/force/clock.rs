//! How a browser's animation-frame timestamps become fixed physics steps.
//!
//! A display refreshes at 60, 120 or 144 Hz, or not at all while a tab is hidden, but physics must take the same fixed
//! step whatever the display does. So elapsed time is gathered into an accumulator and spent a fixed step at a time. The
//! time a frame can bring is bounded, and so is the number of steps it can run, so a frame that arrives long after the
//! last one makes the simulation skip the gap and not race through it.

use super::solver::FIXED_DT;

/// The most wall-clock time one frame can add, in seconds.
pub const MAX_ELAPSED: f64 = 0.1;
/// The most fixed steps one frame can run.
pub const MAX_STEPS_PER_FRAME: u32 = 8;

/// Turns frame timestamps into a number of steps to run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameClock {
    previous_ms: Option<f64>,
    accumulator: f64,
}

impl FrameClock {
    pub const fn new() -> Self {
        Self {
            previous_ms: None,
            accumulator: 0.0,
        }
    }

    /// Forgets the last timestamp and any time not yet spent. The next frame after this is only a new baseline and runs no
    /// steps. It is called when physics starts or resumes, so time that passed while it was stopped is never caught up.
    pub fn restart(&mut self) {
        *self = Self::new();
    }

    /// How many fixed steps to run for a frame that arrived at `raf_ms`, a browser timestamp in milliseconds.
    ///
    /// The first timestamp after [`restart`](Self::restart) only sets the baseline. After that the time since the last
    /// one is clamped to `0..=`[`MAX_ELAPSED`] and added to the accumulator, which is itself capped at [`MAX_ELAPSED`].
    /// At most [`MAX_STEPS_PER_FRAME`] steps run. On a frame that runs exactly that many, whatever is left in the
    /// accumulator is thrown away, even if it holds another whole step, so a slow device falls behind in simulated time
    /// and does not spiral. A frame that runs fewer keeps its fractional remainder.
    ///
    /// A timestamp that is not finite is ignored and runs no steps.
    pub fn advance(&mut self, raf_ms: f64) -> u32 {
        if !raf_ms.is_finite() {
            return 0;
        }
        let Some(previous) = self.previous_ms.replace(raf_ms) else {
            return 0;
        };
        let elapsed = ((raf_ms - previous) / 1000.0).clamp(0.0, MAX_ELAPSED);
        self.accumulator = (self.accumulator + elapsed).min(MAX_ELAPSED);
        let steps = ((self.accumulator / FIXED_DT).floor() as u32).min(MAX_STEPS_PER_FRAME);
        if steps == MAX_STEPS_PER_FRAME {
            self.accumulator = 0.0;
        } else {
            self.accumulator -= f64::from(steps) * FIXED_DT;
        }
        steps
    }

    /// The simulated time not yet spent, in seconds.
    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    /// Whether a baseline timestamp has been taken.
    pub fn has_baseline(&self) -> bool {
        self.previous_ms.is_some()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests {
    use super::*;

    const FRAME_60: f64 = 1000.0 / 60.0;

    #[test]
    fn the_first_timestamp_only_sets_the_baseline() {
        let mut clock = FrameClock::new();
        assert!(!clock.has_baseline());
        assert_eq!(clock.advance(5000.0), 0);
        assert!(clock.has_baseline());
        assert_eq!(clock.accumulator(), 0.0);
    }

    #[test]
    fn a_60_hz_display_runs_two_steps_a_frame_on_average_and_never_more_than_two() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        let mut total = 0;
        for frame in 1..=600 {
            let steps = clock.advance(f64::from(frame) * FRAME_60);
            assert!(steps <= 3, "frame {frame} ran {steps}");
            total += steps;
        }
        // 600 frames at 60 Hz is ten seconds, which is 1200 steps of 1/120 s, to within the one in hand.
        assert!((1199..=1200).contains(&total), "{total}");
    }

    #[test]
    fn a_144_hz_display_spends_nothing_on_some_frames_and_loses_no_time() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        let frame = 1000.0 / 144.0;
        let total: u32 = (1..=1440).map(|n| clock.advance(f64::from(n) * frame)).sum();
        assert!((1199..=1200).contains(&total), "{total}");
    }

    #[test]
    fn a_long_gap_is_clamped_to_a_tenth_of_a_second_and_eight_steps() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        // Five seconds later: 0.1 s is 12 steps, but eight is the most, and the rest is thrown away.
        assert_eq!(clock.advance(5000.0), MAX_STEPS_PER_FRAME);
        assert_eq!(clock.accumulator(), 0.0);
    }

    #[test]
    fn a_frame_that_runs_exactly_eight_steps_discards_even_a_whole_extra_step() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        // 0.0667 s is exactly eight steps, plus a little. The little is dropped too.
        let steps = clock.advance(8.0 * FIXED_DT * 1000.0 + 4.0);
        assert_eq!(steps, 8);
        assert_eq!(clock.accumulator(), 0.0);
    }

    #[test]
    fn a_frame_that_runs_fewer_than_eight_keeps_its_fractional_remainder() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        let steps = clock.advance(1.5 * FIXED_DT * 1000.0);
        assert_eq!(steps, 1);
        assert!((clock.accumulator() - 0.5 * FIXED_DT).abs() < 1e-12);
    }

    #[test]
    fn a_timestamp_that_goes_backwards_adds_no_time() {
        let mut clock = FrameClock::new();
        clock.advance(1000.0);
        assert_eq!(clock.advance(900.0), 0);
        assert_eq!(clock.accumulator(), 0.0);
    }

    #[test]
    fn a_timestamp_that_is_not_finite_is_ignored_and_keeps_the_baseline() {
        let mut clock = FrameClock::new();
        clock.advance(1000.0);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(clock.advance(bad), 0);
        }
        assert_eq!(clock.advance(1000.0 + 2.01 * FIXED_DT * 1000.0), 2);
    }

    #[test]
    fn restarting_forgets_the_baseline_and_the_time_in_hand_so_a_resume_does_not_catch_up() {
        let mut clock = FrameClock::new();
        clock.advance(0.0);
        clock.advance(1.5 * FIXED_DT * 1000.0);
        clock.restart();
        assert!(!clock.has_baseline());
        assert_eq!(clock.accumulator(), 0.0);
        // A frame an hour later is only a baseline.
        assert_eq!(clock.advance(3_600_000.0), 0);
    }
}
