//! The elements that the stage is made of: the one `<svg>` that holds the controls and, below them, the place the tree
//! is swapped into; the hidden place the tree is built in; and the status fields, the live region and the error line
//! that the panel's HTML declares.
//!
//! The buttons and the range are drawn in the stage, as SVG, and the range is the browser's own `<input>` in a
//! `<foreignObject>`. They are outside the part that is replaced, so that a swap never touches them or their focus.

use super::{
    dom::{add, make, set},
    layout::{
        BUTTON_HEIGHT, BUTTONS, CONTROLS_HEIGHT, RANGE_LEFT, RANGE_LENGTH, RANGE_MIDDLE, STAGE_HEIGHT, VIEW_WIDTH,
    },
};
use demo_common::util::{ensure_svg_in, required_element};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::controls::{
    sliders::{self, Slider},
    svg_buttons::{self, Button},
};
use wasm_bindgen::JsCast;

pub(super) const PANEL: &str = "panel-red-black-tree";
pub(super) const STAGE: &str = "rbt-stage";
const STAGE_SVG: &str = "rbt-stage-svg";
const STAGING: &str = "rbt-staging";
const STAGING_SVG: &str = "rbt-staging-svg";
const LIVE: &str = "rbt-live";
const ERROR: &str = "rbt-error";
/// What the library's slider calls the range, and the id of its `<input>`, which the tests find it by.
const SEEK: &str = "seek";
const SEEK_ID: &str = "rbt-seek";

/// The size the library draws a button's label at, the least it is made, and the clear space kept at each end.
const LABEL_SIZE: f64 = 14.0;
const LABEL_SIZE_LEAST: f64 = 9.0;
const LABEL_MARGIN: f64 = 8.0;

/// The fill and the outline of a button.
const BUTTON_COLOURS: (&str, &str) = ("#ffffff", "#64748b");

/// The status fields, by id.
pub(super) const FIELDS: [&str; 7] = [
    "rbt-p-step", "rbt-p-state", "rbt-p-event", "rbt-p-size", "rbt-p-root", "rbt-p-selected", "rbt-p-note",
];

pub(super) struct Chrome {
    pub(super) stage: web_sys::Element,
    /// The group in the stage that each tree is swapped into.
    pub(super) host: web_sys::Element,
    /// The `<svg>` in the hidden place, in which each tree is built before it is swapped in.
    pub(super) staging: web_sys::Element,
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    pub(super) seek: web_sys::HtmlInputElement,
    /// The text that the range writes above itself.
    pub(super) seek_text: web_sys::Element,
    /// In the order of [`BUTTONS`].
    pub(super) buttons: Vec<web_sys::Element>,
    /// In the order of [`FIELDS`].
    pub(super) fields: Vec<web_sys::Element>,
}

impl Chrome {
    /// Draws the controls in the stage, and finds the rest. `last` is the greatest value of the range.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page is missing an element, or a control could not be made.
    pub(super) fn build(document: &web_sys::Document, last: usize) -> Result<Self, String> {
        let size = Size::new(VIEW_WIDTH, STAGE_HEIGHT);
        ensure_svg_in(document, STAGE, STAGE_SVG, size)?;
        ensure_svg_in(document, STAGING, STAGING_SVG, size)?;
        let svg = required_element(document, STAGE_SVG)?;
        let layer = make(document, "g", &[("class", "rbt-controls")])?;
        add(&svg, &layer)?;
        for &(action, label, x, y, width) in &BUTTONS {
            let area = Rect {
                origin: Point::new(x, y),
                size: Size::new(width, BUTTON_HEIGHT),
            };
            let button = Button {
                label,
                action,
                colours: BUTTON_COLOURS,
                pressed: None,
            };
            svg_buttons::add_button(document, &layer, area, &button)?;
            fit_label(&layer, action, width)?;
        }
        let range = Slider {
            label: "Go to step",
            id: SEEK,
            left: RANGE_LEFT,
            middle: RANGE_MIDDLE,
            width: RANGE_LENGTH,
            range: (0.0, last as f64),
            step: 1.0,
        };
        sliders::add_slider(document, &layer, &range, 0.0, &format!("Step 0 of {last}"))?;
        let seek: web_sys::HtmlInputElement = layer
            .query_selector(&format!("input[data-slider=\"{SEEK}\"]"))
            .map_err(|e| format!("could not look for the range: {e:?}"))?
            .and_then(|input| input.dyn_into().ok())
            .ok_or_else(|| "the range is not an <input>".to_owned())?;
        set(&seek, "id", SEEK_ID)?;
        let seek_text = layer
            .query_selector(&format!("[data-slider-host=\"{SEEK}\"] .scene-slider-value"))
            .ok()
            .flatten()
            .ok_or_else(|| "the range has no value text".to_owned())?;
        let buttons = BUTTONS
            .iter()
            .map(|&(action, ..)| {
                layer
                    .query_selector(&format!("[data-action=\"{action}\"]"))
                    .ok()
                    .flatten()
                    .ok_or_else(|| format!("the stage has no button {action:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let host = make(
            document,
            "g",
            &[
                ("class", "rbt-board-host"),
                ("transform", &format!("translate(0 {CONTROLS_HEIGHT})")),
            ],
        )?;
        add(&svg, &host)?;
        Ok(Self {
            stage: required_element(document, STAGE)?,
            host,
            staging: required_element(document, STAGING_SVG)?,
            live: required_element(document, LIVE)?,
            error: required_element(document, ERROR)?,
            seek,
            seek_text,
            buttons,
            fields: FIELDS
                .iter()
                .map(|id| required_element(document, id))
                .collect::<Result<_, _>>()?,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the label of the button `action` in `layer` smaller, if it would not fit between the ends of a button `width`
/// wide. The library draws every label at one size, and a long one runs out of its button. The length is measured in
/// the page, where the label is already, so the button's own font is the one that is measured.
///
/// # Errors
///
/// Returns `Err` if the label is missing or its size could not be written.
fn fit_label(layer: &web_sys::Element, action: &str, width: f64) -> Result<(), String> {
    let label = layer
        .query_selector(&format!("[data-action=\"{action}\"] text"))
        .ok()
        .flatten()
        .ok_or_else(|| format!("the button {action:?} has no label"))?;
    let measure = js_sys::Reflect::get(&label, &"getComputedTextLength".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call0(&label).ok())
        .and_then(|length| length.as_f64())
        .filter(|length| length.is_finite() && *length > 0.0);
    let Some(length) = measure else { return Ok(()) };
    let room = width - 2.0 * LABEL_MARGIN;
    if length > room {
        let size = (LABEL_SIZE * room / length).max(LABEL_SIZE_LEAST);
        set(&label, "font-size", &format!("{size:.2}"))?;
    }
    Ok(())
}
