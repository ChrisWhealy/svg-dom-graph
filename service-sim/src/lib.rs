//! The discrete-event service simulator's view of the engine: the projection that says what the state looks like, and the surface that
//! draws it on a long-lived `svg-dom-graph` `Scene`.
//!
//! The boundaries are those of design revision 6 section 2. The engine (`service-sim-engine`) owns the clock and the heap. The
//! [`projection`] is a pure function from the engine's state to fixed slots, and has no browser in it, so it is tested natively. The
//! [`surface`] writes those slots into a scene that is built once, and never changes shape. The controller and the host scheduler are
//! stage S3, and are not here.

pub mod layout;
pub mod projection;

#[cfg(target_arch = "wasm32")]
pub mod surface;
