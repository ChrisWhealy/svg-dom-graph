//! The elements that the stage is made of: the one `<svg>` that holds the controls and, below them, the place the scene
//! is swapped into; the hidden place a scene is built in; and the status fields, the live region and the error line
//! that the panel's HTML declares.
//!
//! The buttons and the range are the library's own, and a selector or a number is the browser's own control in a
//! `<foreignObject>`. They are all outside the part that is replaced and outside the zoom, so a swap never touches them
//! or their focus, and they stay one size at any zoom.

use super::{
    dom::{add, make, set},
    layout::{
        BUTTON_HEIGHT, BUTTONS, COLUMN, CONTROLS_HEIGHT, FIELD, INSPECT, Place, RANGE_LEFT, RANGE_LENGTH, RANGE_MIDDLE,
        ROW, ROW_HEIGHT, SHOW, STAGE_HEIGHT, STAGE_WIDTH, THRESHOLD,
    },
};
use demo_common::util::{ensure_svg_in, required_element};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::controls::{
    sliders::{self, Slider},
    svg_buttons::{self, Button},
};
use wasm_bindgen::JsCast;

pub(super) const PANEL: &str = "panel-marching-squares";
pub(super) const STAGE: &str = "ms-stage";
const STAGE_SVG: &str = "ms-stage-svg";
const STAGING: &str = "ms-staging";
const STAGING_SVG: &str = "ms-staging-svg";
const LIVE: &str = "ms-live";
const ERROR: &str = "ms-error";
const INSPECTOR: &str = "ms-inspector";
/// What the library's slider calls the range, and the id of its `<input>`.
const SEEK: &str = "seek";
const SEEK_ID: &str = "ms-seek";

const XHTML: &str = "http://www.w3.org/1999/xhtml";
const BUTTON_COLOURS: (&str, &str) = ("#ffffff", "#64748b");

/// The most rows and columns of cells in any field, which the selectors have an option for.
pub(super) const MAX_ROWS: usize = 18;
pub(super) const MAX_COLUMNS: usize = 24;

/// The status fields, by id.
pub(super) const FIELDS: [&str; 6] = [
    "ms-p-step", "ms-p-status", "ms-p-levels", "ms-p-legend", "ms-p-footer", "ms-p-zoom",
];

pub(super) struct Chrome {
    pub(super) stage: web_sys::Element,
    /// The `<svg>` of the stage, whose box is the stage on the screen.
    pub(super) stage_svg: web_sys::Element,
    /// The group in the stage that each scene is swapped into.
    pub(super) host: web_sys::Element,
    /// The `<svg>` in the hidden place, in which each scene is built before it is swapped in.
    pub(super) staging: web_sys::Element,
    pub(super) live: web_sys::Element,
    pub(super) error: web_sys::Element,
    pub(super) inspector: web_sys::Element,
    pub(super) field: web_sys::HtmlSelectElement,
    pub(super) show: web_sys::HtmlSelectElement,
    pub(super) inspect: web_sys::HtmlSelectElement,
    pub(super) threshold: web_sys::HtmlInputElement,
    pub(super) row: web_sys::HtmlSelectElement,
    pub(super) column: web_sys::HtmlSelectElement,
    pub(super) seek: web_sys::HtmlInputElement,
    /// The text that the range writes above itself.
    pub(super) seek_text: web_sys::Element,
    /// In the order of [`BUTTONS`].
    pub(super) buttons: Vec<web_sys::Element>,
    /// In the order of [`FIELDS`].
    pub(super) fields: Vec<web_sys::Element>,
}

impl Chrome {
    /// Draws the controls in the stage, and finds the rest. `fields` are the options of the field selector, and `last`
    /// is the greatest value of the range.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page is missing an element, or a control could not be made.
    pub(super) fn build(
        document: &web_sys::Document,
        fields: &[(String, String)],
        last: usize,
    ) -> Result<Self, String> {
        let size = Size::new(STAGE_WIDTH, STAGE_HEIGHT);
        ensure_svg_in(document, STAGE, STAGE_SVG, size)?;
        ensure_svg_in(document, STAGING, STAGING_SVG, size)?;
        let svg = required_element(document, STAGE_SVG)?;
        let layer = make(document, "g", &[("class", "ms-controls")])?;
        add(&svg, &layer)?;
        let field = select(document, &layer, FIELD, ("Field", "ms-field"), fields)?;
        let levels = [
            ("all", "All three"),
            ("low", "Low only"),
            ("middle", "Middle only"),
            ("high", "High only"),
        ];
        let show = select(document, &layer, SHOW, ("Show", "ms-show"), &owned(&levels))?;
        let slots = [("0", "low"), ("1", "middle"), ("2", "high")];
        let inspect = select(document, &layer, INSPECT, ("Inspect", "ms-inspect"), &owned(&slots))?;
        let threshold = number(document, &layer, THRESHOLD, ("Threshold", "ms-threshold"))?;
        let mut rows = vec![(String::new(), "none".to_owned())];
        rows.extend((0..MAX_ROWS).map(|r| (r.to_string(), r.to_string())));
        let row = select(document, &layer, ROW, ("Row", "ms-row"), &rows)?;
        let mut columns = vec![(String::new(), "none".to_owned())];
        columns.extend((0..MAX_COLUMNS).map(|c| (c.to_string(), c.to_string())));
        let column = select(document, &layer, COLUMN, ("Column", "ms-column"), &columns)?;
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
                ("class", "ms-scene-host"),
                ("transform", &format!("translate(0 {CONTROLS_HEIGHT})")),
            ],
        )?;
        add(&svg, &host)?;
        Ok(Self {
            stage: required_element(document, STAGE)?,
            stage_svg: svg.clone(),
            host,
            staging: required_element(document, STAGING_SVG)?,
            live: required_element(document, LIVE)?,
            error: required_element(document, ERROR)?,
            inspector: required_element(document, INSPECTOR)?,
            field,
            show,
            inspect,
            threshold,
            row,
            column,
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

fn owned(options: &[(&str, &str)]) -> Vec<(String, String)> {
    options.iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect()
}

/// Draws the name of a control in `layer`.
fn name_of(document: &web_sys::Document, layer: &web_sys::Element, place: Place, label: &str) -> Result<(), String> {
    let text = make(
        document,
        "text",
        &[
            ("x", &place.name_left.to_string()),
            ("y", &(place.top + ROW_HEIGHT / 2.0).to_string()),
            ("dominant-baseline", "central"),
            ("font-size", "14"),
            ("fill", "#1a1a1a"),
            ("aria-hidden", "true"),
            ("style", "pointer-events: none; user-select: none;"),
        ],
    )?;
    text.set_text_content(Some(label));
    add(layer, &text)
}

/// A `<foreignObject>` at `place`, to hold an HTML control.
fn object(document: &web_sys::Document, layer: &web_sys::Element, place: Place) -> Result<web_sys::Element, String> {
    let object = make(
        document,
        "foreignObject",
        &[
            ("x", &place.left.to_string()),
            ("y", &place.top.to_string()),
            ("width", &place.width.to_string()),
            ("height", &ROW_HEIGHT.to_string()),
        ],
    )?;
    add(layer, &object)?;
    Ok(object)
}

fn html(document: &web_sys::Document, tag: &str) -> Result<web_sys::Element, String> {
    document
        .create_element_ns(Some(XHTML), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))
}

/// A control fills its box, and is set in a size that fits the longest option with room for the arrow, so no text runs
/// out of it. A text that is still too long is cut with an ellipsis.
const CONTROL_STYLE: &str = "width: 100%; height: 100%; box-sizing: border-box; font-family: inherit; font-size: 13px; \
                             text-overflow: ellipsis;";

/// Draws a selector and its name in `layer`.
fn select(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    place: Place,
    name: (&str, &str),
    options: &[(String, String)],
) -> Result<web_sys::HtmlSelectElement, String> {
    let (label, id) = name;
    name_of(document, layer, place, label)?;
    let holder = object(document, layer, place)?;
    let select: web_sys::HtmlSelectElement = html(document, "select")?
        .dyn_into()
        .map_err(|_| "a selector is not a <select>".to_owned())?;
    for (attribute, value) in [("id", id), ("aria-label", label), ("style", CONTROL_STYLE)] {
        set(&select, attribute, value)?;
    }
    for (value, shown) in options {
        let option = html(document, "option")?;
        set(&option, "value", value)?;
        option.set_text_content(Some(shown));
        add(&select, &option)?;
    }
    add(&holder, &select)?;
    Ok(select)
}

/// Draws a number and its name in `layer`.
fn number(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    place: Place,
    name: (&str, &str),
) -> Result<web_sys::HtmlInputElement, String> {
    let (label, id) = name;
    name_of(document, layer, place, label)?;
    let holder = object(document, layer, place)?;
    let input: web_sys::HtmlInputElement = html(document, "input")?
        .dyn_into()
        .map_err(|_| "a number is not an <input>".to_owned())?;
    for (attribute, value) in [
        ("id", id),
        ("type", "number"),
        ("step", "0.001"),
        ("aria-label", "Threshold of the inspected level"),
        ("style", CONTROL_STYLE),
    ] {
        set(&input, attribute, value)?;
    }
    add(&holder, &input)?;
    Ok(input)
}
