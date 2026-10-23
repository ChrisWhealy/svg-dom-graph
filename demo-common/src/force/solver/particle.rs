//! One circle, and the pin that holds it.

use super::Vec2;

/// Where a pinned particle is being pulled to, and whether it lets go on arriving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinTarget {
    /// The position of the vertex as the scene last committed it. The proxy moves towards this.
    pub committed_centre: Vec2,
    /// Whether the pin is released when the proxy arrives. Set when the pointer is released.
    pub release_on_arrival: bool,
}

/// One circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub centre: Vec2,
    pub velocity: Vec2,
    pub mass: f64,
    pub radius: f64,
    /// `Some` while the particle is kinematic: held by a pointer, or still catching up after one.
    pub pin: Option<PinTarget>,
}
