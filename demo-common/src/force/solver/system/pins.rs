//! Holding a circle with a pointer, and letting go.

use super::*;

impl SpringSystem {
    /// Pins particle `index` for a held vertex whose circle is now at `actual`.
    ///
    /// A particle that is free is pinned where `actual` is, with no velocity. One that is already pinned, as when a new
    /// press comes while the proxy is still catching up, keeps its proxy where it is and only has its target changed,
    /// so nothing jumps. Either way a pending release is cancelled.
    ///
    /// Returns `false`, and changes nothing, for an unknown particle or a position that is not finite.
    pub fn begin_pin(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() {
            return false;
        }
        let target = self.bounds.clamp_centre(actual, self.particles[index].radius);
        let particle = &mut self.particles[index];
        if particle.pin.is_none() {
            particle.centre = target;
        }
        particle.velocity = Vec2::ZERO;
        particle.pin = Some(PinTarget {
            committed_centre: target,
            release_on_arrival: false,
        });
        self.disturb();
        true
    }

    /// Tells a held particle that its vertex is now at `actual`, which is where the scene committed it.
    ///
    /// If that is the target it already has, the quiet run is kept and the system is asked for one frame to check, and
    /// a target that has changed begins a new run. A particle that is not held is left alone.
    ///
    /// Returns whether a held pin was updated.
    pub fn update_pin_target(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() {
            return false;
        }
        let radius = self.particles[index].radius;
        let target = self.bounds.clamp_centre(actual, radius);
        let Some(pin) = self.particles[index].pin.as_mut() else {
            return false;
        };
        if pin.release_on_arrival {
            return false;
        }
        if pin.committed_centre == target {
            self.rearmed = true;
        } else {
            pin.committed_centre = target;
            self.quiet_streak = 0;
            self.asleep = false;
            self.rearmed = false;
        }
        true
    }

    /// The pointer was released and the vertex is at `actual`. The proxy keeps its pin while it catches up, and lets go
    /// when it arrives. Returns whether a pin was there.
    pub fn end_pin(&mut self, index: usize, actual: Vec2) -> bool {
        self.release_pin(index, actual)
    }

    /// The drag was cancelled and the last committed position of the vertex is `actual`. It behaves like
    /// [`end_pin`](Self::end_pin): the proxy catches up to that position and then lets go, and no position newer than
    /// it is used.
    pub fn cancel_pin(&mut self, index: usize, actual: Vec2) -> bool {
        self.release_pin(index, actual)
    }

    pub(super) fn release_pin(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() || self.particles[index].pin.is_none() {
            return false;
        }
        let target = self.bounds.clamp_centre(actual, self.particles[index].radius);
        self.particles[index].pin = Some(PinTarget {
            committed_centre: target,
            release_on_arrival: true,
        });
        self.disturb();
        true
    }
}
