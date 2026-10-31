//! `panel-amplitude-frequency`: one sine wave, and three sliders in the scene that set its amplitude, its frequency and
//! its phase. See [`build_amplitude_frequency_demo`] for what the page does, and [`plots`] for what is drawn.

mod plots;

use crate::plot_scene::Plot;
use demo_common::{
    sliders::{self, Slider},
    util::{document, required_element},
};
use plots::{
    AMPLITUDE_STEP, AMPLITUDES, FREQUENCIES, FREQUENCY_STEP, PHASE_STEP, PHASES, Wave, amplitude_text, config,
    description, frequency_text, items, phase_text,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the configuration of the plot and what is drawn on it. The sliders are in
/// `demo-common/src/sliders`, which the other demos with sliders in the scene share.
pub(crate) const FILES: &[crate::SourceFile] =
    &[crate::source_file!("amplitude_frequency/plots/mod.rs", "plots/mod.rs")];

/// The ids of the elements that the page declares. The stage is empty, and gets an `<svg>` of the same name.
const STAGE: &str = "amplitude-frequency-stage";
const ERROR: &str = "amplitude-frequency-error";

/// The ids of the three sliders. They are also what [`sliders::watch`] says moved.
const AMPLITUDE: &str = "amplitude";
const FREQUENCY: &str = "frequency";
const PHASE: &str = "phase";

/// Where the sliders are, in the strip at the top of the scene: the left end of each, the height of the middle of all,
/// and how long each is. There is a gap between them, and a margin at each side.
const LEFTS: [f64; 3] = [20.0, 290.0, 560.0];
const MIDDLE: f64 = 56.0;
const LENGTH: f64 = 220.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the plot, the wave that is on it, and where to say an error.
struct Demo {
    plot: Plot,
    scene: web_sys::Element,
    wave: Cell<Wave>,
    error: web_sys::Element,
}

thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

impl Demo {
    /// Draws the wave, and tells assistive technology what it is.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the plot could not be drawn.
    fn show(&self) -> Result<(), String> {
        let wave = self.wave.get();
        self.plot
            .scene
            .set_plot_description(self.plot.node, &description(&wave))
            .map_err(|e| e.to_string())?;
        self.plot.draw(&items(&wave))
    }

    /// Sets the slider `id` to `value` in the wave, and on the screen. Draws the wave again.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `id` is not a slider of this page, or if the slider or the plot could not be written.
    fn set(&self, id: &str, value: f64) -> Result<(), String> {
        let mut wave = self.wave.get();
        let shown = match id {
            AMPLITUDE => {
                wave.amplitude = value;
                amplitude_text(value)
            },
            FREQUENCY => {
                wave.frequency = value;
                frequency_text(value)
            },
            PHASE => {
                wave.phase = value;
                phase_text(value)
            },
            other => return Err(format!("there is no slider {other:?}")),
        };
        self.wave.set(wave);
        sliders::set_value(&self.scene, id, value, &shown)?;
        self.show()
    }

    /// Does what a person has asked of a slider, and says on the page if that went wrong.
    fn moved_by_person(&self, id: &str, value: f64) {
        if let Err(problem) = self.set(id, value) {
            self.error
                .set_text_content(Some(&format!("The wave could not be drawn: {problem}")));
            let _ = self.error.remove_attribute("hidden");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the amplitude, frequency and phase demo: one sine wave, and three sliders that set how it looks.
///
/// The wave is `y = A · sin(2π · f · t + φ)`. The page shows it with one scene, 800 pixels wide and 400 high, that has
/// one plot node in it:
///
/// 1. At the top is a strip with three sliders, drawn in the scene: the amplitude `A`, from 0 to 1, the frequency `f`,
///    from 1 Hz to 8 Hz, and the phase `φ`, from 0 to 360 degrees. They are moved with the pointer, or with the keys
///    when they have the focus.
/// 2. Below it is the plot, with time along the bottom from 0 to 1 second, which does not change, and the height from
///    -1.2 to 1.2. A higher frequency packs more cycles into that second. Dotted lines mark the amplitude, and the
///    formula for the wave, with its numbers, is written at the bottom left.
///
/// Each move of a slider draws the wave again with [`Scene::set_plot_items`], so the grid and the axes are never
/// redrawn.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the scene or a listener could not be made.
pub(crate) fn build_amplitude_frequency_demo() -> Result<(), String> {
    let document = document()?;
    let wave = Wave::first();
    let plot = Plot::new(STAGE, config(&wave))?;
    let scene = sliders::scene(&document, &format!("{STAGE}-svg"))?;
    for (left, (id, label, range, step, value, shown)) in LEFTS.into_iter().zip([
        (
            AMPLITUDE,
            "Amplitude",
            AMPLITUDES,
            AMPLITUDE_STEP,
            wave.amplitude,
            amplitude_text(wave.amplitude),
        ),
        (
            FREQUENCY,
            "Frequency",
            FREQUENCIES,
            FREQUENCY_STEP,
            wave.frequency,
            frequency_text(wave.frequency),
        ),
        (PHASE, "Phase", PHASES, PHASE_STEP, wave.phase, phase_text(wave.phase)),
    ]) {
        let slider = Slider {
            label,
            id,
            left,
            middle: MIDDLE,
            width: LENGTH,
            range,
            step,
        };
        sliders::add_slider(&document, &scene, &slider, value, &shown)?;
    }

    let demo = Rc::new(Demo {
        plot,
        scene,
        wave: Cell::new(wave),
        error: required_element(&document, ERROR)?,
    });
    demo.show()?;

    let moved = Rc::clone(&demo);
    sliders::watch(&document, STAGE, move |id, value| moved.moved_by_person(id, value))?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
