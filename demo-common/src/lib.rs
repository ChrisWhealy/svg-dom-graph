//! What the `svg-dom-graph` demos share, so that a demo's own crate holds only what is particular to the demo.
//!
//! - [`util`] — small DOM and error helpers, the layout of the stepping toolbar, and the helpers that dim the parts of
//!   a scene a walk has not reached.
//! - [`svg_buttons`] — buttons drawn in SVG, for demos that need a row of choices inside the scene.
//! - [`sorting`] — everything the sorting demos have in common: the bars, the buttons and the stepping. An algorithm
//!   supplies only the frames of its walk.
//!
//! Nothing here knows which demos exist. A demo depends on this crate, never the other way round.

pub mod sorting;
pub mod svg_buttons;
pub mod util;
