//! `panel-phase-shift`: two scenes that show a phase shift as the angle of a point on the unit circle. See
//! [`build_phase_shift_demo`] for what the page does, and [`plots`] for what is drawn.

mod plots;

use crate::{circle_control::CircleControl, plot_scene::Plot};
use demo_common::util::{document, required_element};
use plots::{phase_circle_config, phase_circle_items, phase_items, phase_sine_config};
use std::{cell::RefCell, rc::Rc};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the plots and what is drawn on them, and the circle that a person can
/// move C round, which the sine and cosine demo has too.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("phase_shift/plots/mod.rs", "plots/mod.rs"),
    crate::source_file!("circle_control/mod.rs", "../circle_control/mod.rs"),
    crate::source_file!("circle_control/angles/mod.rs", "../circle_control/angles/mod.rs"),
];

/// The angle that C starts at, in degrees, and so the phase shift that the second wave starts with.
const FIRST_ANGLE: f64 = 45.0;

/// The ids of the elements that the page declares. Each stage is empty, and gets an `<svg>` of the same name.
const CIRCLE_STAGE: &str = "phase-shift-circle-stage";
const SINE_STAGE: &str = "phase-shift-sine-stage";
const ERROR: &str = "phase-shift-error";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the circle with C on it, the sine plot, and where to say an error.
struct Demo {
    circle: Rc<CircleControl>,
    sine: Plot,
    error: web_sys::Element,
}

thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

impl Demo {
    /// Draws both plots for a phase shift of `degrees`, which is where C is.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a plot could not be drawn.
    fn show(&self, degrees: f64) -> Result<(), String> {
        self.circle.plot.draw(&phase_circle_items(degrees))?;
        self.sine.draw(&phase_items(degrees))?;
        self.circle.place(degrees)
    }

    /// Moves C to `degrees` for a person who has dragged it or pressed a key, and says on the page if that went wrong.
    fn moved_by_person(&self, degrees: f64) {
        if let Err(problem) = self.show(degrees) {
            let message = format!("The plots could not be drawn: {problem}");
            self.error.set_text_content(Some(&message));
            let _ = self.error.remove_attribute("hidden");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the phase shift demo: the unit circle on the left, with a point C on it that can be dragged round.
///
/// A phase shift is how far along its cycle a wave is when it is first looked at. The page shows that with two scenes,
/// each of which has one plot node:
///
/// 1. The circle, 400 pixels square. C starts at 45 degrees, and it can be dragged round the circle, or moved with the
///    keys when it has the focus. It is not animated. An arc near the middle and the angle beside it show the angle of
///    C from the positive horizontal axis, the long way round when that is more than 180 degrees.
/// 2. The sine, to the right of the circle, 430 pixels wide. It has the circle's vertical scale. It has two waves in
///    it. The first, in blue, is `sin(x)` and never moves. The second is `sin(x + φ)`, where φ is the angle of C, so it
///    moves to the left as C goes anti-clockwise round the circle. At the left edge it is at the height of C.
///
/// Each move of C draws both plots again with [`Scene::set_plot_items`], so the grids and axes are never redrawn.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if a scene or a listener could not be made.
pub(crate) fn build_phase_shift_demo() -> Result<(), String> {
    let document = document()?;
    let demo = Rc::new(Demo {
        circle: CircleControl::new(CIRCLE_STAGE, phase_circle_config(), FIRST_ANGLE)?,
        sine: Plot::new(SINE_STAGE, phase_sine_config())?,
        error: required_element(&document, ERROR)?,
    });
    demo.show(FIRST_ANGLE)?;

    let moved = Rc::clone(&demo);
    demo.circle.wire(move |degrees| moved.moved_by_person(degrees))?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
