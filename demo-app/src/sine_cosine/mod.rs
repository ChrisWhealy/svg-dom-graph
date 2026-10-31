//! `panel-sine-cosine`: three scenes that show sine and cosine to be the height and the width of a point going round
//! the unit circle. See [`build_sine_cosine_demo`] for what the page does, and [`plots`] for what is drawn.

pub(crate) mod plots;

use crate::{circle_control::CircleControl, plot_scene::Plot};
use demo_common::{
    force::animation::AnimationLoop,
    svg_buttons::{self, Button},
    util::{document, required_element},
};
use plots::{circle_config, circle_items, cosine_config, cosine_items, readout, sine_config, sine_items, turned};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::root::utils::{Point, Rect, Size};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the configuration of the plots and what is drawn on them, and the circle
/// that a person can move C round, which the phase shift demo has too.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("sine_cosine/plots/mod.rs", "plots/mod.rs"),
    crate::source_file!("circle_control/mod.rs", "../circle_control/mod.rs"),
    crate::source_file!("circle_control/angles/mod.rs", "../circle_control/angles/mod.rs"),
];

/// The angle that C starts at, in degrees: on the right, where the circle meets the horizontal axis, with none of the
/// curves drawn yet. The longest gap between two frames that is counted as time passing, in milliseconds. A page
/// that was in the background for a while resumes from where it was and does not jump.
const FIRST_ANGLE: f64 = 0.0;
const LONGEST_FRAME: f64 = 100.0;

/// The button in the circle scene that stops C and starts it again: its action, where it is, and its colours. It is in
/// the bottom right corner, which the circle does not reach.
const PLAY: &str = "play";
const PLAY_AREA: Rect = Rect {
    origin: Point { x: 316.0, y: 360.0 },
    size: Size { width: 72.0, height: 28.0 },
};
const PLAY_COLOURS: (&str, &str) = ("#ffffff", "#64748b");

/// The ids of the elements that the page declares. Each stage is empty, and gets an `<svg>` of the same name.
const CIRCLE_STAGE: &str = "sine-cosine-circle-stage";
const SINE_STAGE: &str = "sine-cosine-sine-stage";
const COSINE_STAGE: &str = "sine-cosine-cosine-stage";
const READOUT: &str = "sine-cosine-readout";
const ERROR: &str = "sine-cosine-error";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the three plots, where C is, and whether it is going round.
struct Demo {
    circle: Rc<CircleControl>,
    sine: Plot,
    cosine: Plot,
    /// When the last frame came, or `None` when the next frame has no earlier one to measure from.
    last_frame: Cell<Option<f64>>,
    playing: Cell<bool>,
    /// Set once, just after the demo is made, since the loop's callback needs the demo.
    animation: OnceCell<Rc<AnimationLoop>>,
    readout: web_sys::Element,
    error: web_sys::Element,
}

thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

impl Demo {
    /// Draws all three plots and the numbers for C at `degrees`, and remembers that is where C is.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a plot could not be drawn.
    fn show(&self, degrees: f64) -> Result<(), String> {
        self.circle.plot.draw(&circle_items(degrees))?;
        self.sine.draw(&sine_items(degrees))?;
        self.cosine.draw(&cosine_items(degrees))?;
        self.readout.set_text_content(Some(&readout(degrees)));
        self.circle.place(degrees)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves C on by the time since the last frame, and asks for the next frame. A failure stops the demo.
    fn frame(&self, now: f64) {
        if !self.playing.get() {
            return;
        }
        let elapsed = self
            .last_frame
            .get()
            .map_or(0.0, |before| (now - before).clamp(0.0, LONGEST_FRAME));
        self.last_frame.set(Some(now));
        match self.show(turned(self.circle.angle(), elapsed)) {
            Ok(()) => self.request_frame(),
            Err(problem) => self.fail(&problem),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn request_frame(&self) {
        if let Some(animation) = self.animation.get() {
            animation.request();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts or stops C going round, and writes on the button what it will do next.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be relabelled.
    fn set_playing(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        self.last_frame.set(None);
        match (playing, self.animation.get()) {
            (true, _) => self.request_frame(),
            (false, Some(animation)) => animation.cancel(),
            (false, None) => {},
        }
        svg_buttons::set_label(&self.circle.layer, PLAY, if playing { "Pause" } else { "Play" })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops or starts C, and says on the page if that went wrong.
    fn toggle(&self) {
        if let Err(problem) = self.set_playing(!self.playing.get()) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves C to `degrees` for a person who has taken hold of it or pressed a key, which stops it going round itself.
    fn moved_by_person(&self, degrees: f64) {
        if self.playing.get() {
            self.toggle();
        }
        if let Err(problem) = self.show(degrees) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops C and says why on the page.
    fn fail(&self, problem: &str) {
        self.playing.set(false);
        if let Some(animation) = self.animation.get() {
            animation.cancel();
        }
        self.error
            .set_text_content(Some(&format!("The plots could not be drawn: {problem}")));
        let _ = self.error.remove_attribute("hidden");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the person has asked their system for less motion.
fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|query| query.matches())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the sine and cosine demo: the unit circle on the left, with a point C that goes round it anti-clockwise.
///
/// Sine is the height of C, and cosine is its width. The page shows that with three scenes, each of which has one plot
/// node:
///
/// 1. The circle, 400 pixels square, with the axes from -1.2 to 1.2. C casts dotted lines to the axes, where the width
///    W and the height H are. The button that stops C and starts it again is drawn in this scene too.
/// 2. The sine, to the right of the circle, 430 pixels wide. It has the circle's vertical scale, so the height of C in
///    the circle is at the same level here. The angle runs along the bottom.
/// 3. The cosine, below the circle, 430 pixels high. It has the circle's horizontal scale, so the width of C in the
///    circle is at the same place across here. The angle runs down the middle.
///
/// The circle drives the other two. Each frame, the angle moves on, and all three plots are drawn again from it with
/// [`Scene::set_plot_items`], so the grids and axes are never redrawn. C can be dragged round the circle, or moved with
/// the keys when it has the focus, which stops it going round by itself. Someone who has asked for less motion starts
/// with C still.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if a scene or a listener could not be made.
pub(crate) fn build_sine_cosine_demo() -> Result<(), String> {
    let document = document()?;
    let circle = CircleControl::new(CIRCLE_STAGE, circle_config(), FIRST_ANGLE)?;
    let button = Button {
        label: "Pause",
        action: PLAY,
        colours: PLAY_COLOURS,
        pressed: None,
    };
    svg_buttons::add_button(&document, &circle.layer, PLAY_AREA, &button)?;

    let demo = Rc::new(Demo {
        circle,
        sine: Plot::new(SINE_STAGE, sine_config())?,
        cosine: Plot::new(COSINE_STAGE, cosine_config())?,
        last_frame: Cell::new(None),
        playing: Cell::new(false),
        animation: OnceCell::new(),
        readout: required_element(&document, READOUT)?,
        error: required_element(&document, ERROR)?,
    });
    // The loop holds the demo weakly, since the demo holds the loop.
    let weak: Weak<Demo> = Rc::downgrade(&demo);
    let animation = AnimationLoop::new(move |now| {
        if let Some(demo) = weak.upgrade() {
            demo.frame(now);
        }
    })?;
    let _ = demo.animation.set(animation);

    demo.show(FIRST_ANGLE)?;

    let pressed = Rc::clone(&demo);
    svg_buttons::watch(&document, CIRCLE_STAGE, move |action| {
        if action == PLAY {
            pressed.toggle();
        }
    })?;
    let moved = Rc::clone(&demo);
    demo.circle.wire(move |degrees| moved.moved_by_person(degrees))?;

    demo.set_playing(!prefers_reduced_motion())?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
