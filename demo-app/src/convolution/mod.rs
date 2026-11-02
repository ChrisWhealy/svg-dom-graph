//! `panel-convolution`: how a sine and a cosine of one frequency find out how much of that frequency is in a signal. See
//! [`build_convolution_demo`] for what the page does, [`analysis`] for the sums, and [`plots`] for what is drawn.

mod analysis;
mod plots;

use crate::plot_scene::Plot;
use analysis::{FIRST_PROBE, FREQUENCIES, FREQUENCY_STEP, Finding, Probe, samples, spectrum};
use demo_common::util::{document, required_element};
use plots::{
    STRIP, frequency_text, product_config, product_description, product_items, signal_config, signal_description,
    signal_items, spectrum_config, spectrum_description, spectrum_items,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use svg_dom_graph::controls::sliders::{self, Slider};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the sums, and what is drawn on the four plots. The sliders are in
/// `src/controls/sliders`, which the other demos with sliders in the scene share.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("convolution/analysis/mod.rs", "analysis/mod.rs"),
    crate::source_file!("convolution/plots/mod.rs", "plots/mod.rs"),
];

/// The ids of the stages that the page declares. Each is empty, and gets an `<svg>` of the same name with `-svg` added.
const SIGNAL_STAGE: &str = "convolution-stage";
const COSINE_STAGE: &str = "convolution-cosine-stage";
const SINE_STAGE: &str = "convolution-sine-stage";
const SPECTRUM_STAGE: &str = "convolution-spectrum-stage";
const ERROR: &str = "convolution-error";

/// The id of the slider, which is also what [`sliders::watch`] says moved.
const PROBE: &str = "probe";

/// Where the slider is, in the strip at the top of the signal scene: the left end, the height of the middle, and how
/// long it is.
const SLIDER_LEFT: f64 = 20.0;
const SLIDER_MIDDLE: f64 = STRIP / 2.0 + 8.0;
const SLIDER_LENGTH: f64 = 320.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the four plots, the signal and what was found at every frequency, which
/// never change, and the probe, which does.
struct Demo {
    signal: Plot,
    cosine: Plot,
    sine: Plot,
    spectrum: Plot,
    scene: web_sys::Element,
    samples: Vec<f64>,
    amplitudes: Vec<(f64, f64)>,
    frequency: Cell<f64>,
    error: web_sys::Element,
}

thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

impl Demo {
    /// Probes the signal with the cosine and the sine of `frequency`, and draws everything that depends on it.
    ///
    /// Each plot is drawn on its own, and a plot that fails leaves what it showed before. So a failure part way leaves
    /// the plots before it at `frequency` and the plots after it at the frequency before. The demo accepts that as an
    /// error, which [`Demo::moved_by_person`] reports, and the next successful call draws all four again. The frequency
    /// that is remembered is only changed once all four are drawn.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a plot could not be drawn.
    fn show(&self, frequency: f64) -> Result<(), String> {
        let finding = Finding::new(&self.samples, frequency);
        let describe =
            |plot: &Plot, text: String| plot.scene.set_plot_description(plot.node, &text).map_err(|e| e.to_string());

        describe(&self.signal, signal_description(frequency))?;
        self.signal.draw(&signal_items(&self.samples, frequency))?;
        for (plot, probe) in [(&self.cosine, Probe::Cosine), (&self.sine, Probe::Sine)] {
            describe(plot, product_description(probe, &finding))?;
            plot.draw(&product_items(probe, &finding))?;
        }
        describe(&self.spectrum, spectrum_description(&finding))?;
        self.spectrum.draw(&spectrum_items(&self.amplitudes, &finding))?;
        self.frequency.set(frequency);
        Ok(())
    }

    /// Does what a person has asked of the slider, and says on the page if that went wrong. A success hides what an
    /// earlier failure said.
    fn moved_by_person(&self, frequency: f64) {
        let result = sliders::set_value(&self.scene, PROBE, frequency, &frequency_text(frequency))
            .and_then(|()| self.show(frequency));
        match result {
            Ok(()) => {
                let _ = self.error.set_attribute("hidden", "");
            },
            Err(problem) => {
                self.error.set_text_content(Some(&format!(
                    "The signal could not be probed at {}: {problem}. The four scenes may now disagree with each \
                     other and with the slider. Move the slider again to draw them all again.",
                    frequency_text(frequency)
                )));
                let _ = self.error.remove_attribute("hidden");
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the convolution demo: a signal of two sine waves, probed with a cosine and a sine of one frequency.
///
/// The page shows four scenes that have one plot node each, over one second of time:
///
/// 1. At the top is the signal, which is a 2 Hz wave and a 5 Hz wave added together. The cosine and the sine of the
///    probe are drawn thinly behind it. A slider in the scene sets the frequency of the probe, from 0 to 10 Hz.
/// 2. Below it, on the left, is the signal multiplied by the cosine. The area between that and the axis is shaded blue
///    where it is above the axis and orange where it is below, and the three areas are written: above, below, and the
///    integral, which is the one less the other.
/// 3. On the right is the same for the sine.
/// 4. At the bottom is the amplitude that is found at every frequency, which has a peak at each wave of the signal, and
///    a dot at the frequency of the probe. Under it is what was found there: twice each integral, the amplitude that
///    those two make, and the phase.
///
/// A move of the slider draws all but the spectrum's curve again with [`Scene::set_plot_items`], so the grids and the
/// axes are never redrawn.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if a scene or a listener could not be made.
pub(crate) fn build_convolution_demo() -> Result<(), String> {
    let document = document()?;
    let samples = samples();
    let amplitudes = spectrum(&samples);
    let finding = Finding::new(&samples, FIRST_PROBE);

    let signal = Plot::new(SIGNAL_STAGE, signal_config(FIRST_PROBE))?;
    let cosine = Plot::new(COSINE_STAGE, product_config(Probe::Cosine, &finding))?;
    let sine = Plot::new(SINE_STAGE, product_config(Probe::Sine, &finding))?;
    let spectrum_plot = Plot::new(SPECTRUM_STAGE, spectrum_config(&finding))?;

    let scene = sliders::scene(&document, &format!("{SIGNAL_STAGE}-svg"))?;
    let slider = Slider {
        label: "Probe frequency",
        id: PROBE,
        left: SLIDER_LEFT,
        middle: SLIDER_MIDDLE,
        width: SLIDER_LENGTH,
        range: FREQUENCIES,
        step: FREQUENCY_STEP,
    };
    sliders::add_slider(&document, &scene, &slider, FIRST_PROBE, &frequency_text(FIRST_PROBE))?;

    let demo = Rc::new(Demo {
        signal,
        cosine,
        sine,
        spectrum: spectrum_plot,
        scene,
        samples,
        amplitudes,
        frequency: Cell::new(FIRST_PROBE),
        error: required_element(&document, ERROR)?,
    });
    demo.show(FIRST_PROBE)?;

    let moved = Rc::clone(&demo);
    sliders::watch(&document, SIGNAL_STAGE, move |_, frequency| moved.moved_by_person(frequency))?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
