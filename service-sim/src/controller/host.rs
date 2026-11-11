//! What the controller draws on and says to a person, which in a page is the DOM.

use super::{DrawFailure, Presence};
use crate::projection::Projection;

/// What the controller draws on and says to a person, which in a page is the DOM.
pub trait Host {
    /// Whether a draw is allowed now: a synchronous test of the connection and of the panel's visibility, which the
    /// controller makes at the entry of every callback, before it schedules, and immediately before and after every
    /// draw.
    fn presence(&self) -> Presence;

    /// Draws `projection`. With `full` every field is written, and otherwise what differs from the last draw, as the
    /// host knows it.
    ///
    /// # Errors
    ///
    /// Returns a [`DrawFailure`] if a write failed. What is on the page is then unknown, and the next draw has to be a
    /// full one.
    fn draw(&mut self, projection: &Projection, full: bool) -> Result<(), DrawFailure>;

    /// Tells a person something, in a polite live region.
    fn announce(&mut self, text: &str);

    /// Reports a failure, once for each episode.
    fn report(&mut self, text: &str);

    /// Says that playback started or stopped, for the button that shows it.
    fn playing_changed(&mut self, playing: bool);
}
