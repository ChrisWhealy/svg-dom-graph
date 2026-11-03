//! The controls, drawn in the scene: two selectors, six buttons and a range. The buttons and the range are the
//! library's own, and a selector is the browser's own `<select>` in a `<foreignObject>`, as the range is.

use super::{
    dom::{add, make, set},
    layout::{
        BUTTONS, RANGE_LEFT, RANGE_LENGTH, RANGE_MIDDLE, ROW_HEIGHT, SIZE_SELECTOR, SPEED_SELECTOR, SelectorPlace,
        button_area,
    },
};
use svg_dom_graph::controls::{
    sliders::{self, Slider},
    svg_buttons::{self, Button},
};
use wasm_bindgen::JsCast;

/// The ids of the two selectors and of the range's `<input>`, which the page and the tests find them by.
pub(super) const SIZE_ID: &str = "n-queens-size";
pub(super) const SPEED_ID: &str = "n-queens-speed";
pub(super) const SEEK_ID: &str = "n-queens-seek";

/// What the library's slider calls the range, which is also what it is found by in the scene.
const SEEK: &str = "seek";

/// The fill and the outline of a button.
const BUTTON: (&str, &str) = ("#ffffff", "#64748b");

/// The XML namespace of HTML elements, which a `<foreignObject>` holds.
const XHTML: &str = "http://www.w3.org/1999/xhtml";

/// The sizes of board that the selector offers, and the speeds, in events for each second.
pub(super) const SIZES: [usize; 6] = [4, 5, 6, 7, 8, 9];
pub(super) const SPEEDS: [u32; 4] = [1, 5, 20, 50];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The controls that are drawn in the scene. The scene is where they are found again.
pub(super) struct Controls {
    scene: web_sys::Element,
    layer: web_sys::Element,
    size: web_sys::HtmlSelectElement,
    speed: web_sys::HtmlSelectElement,
    seek: web_sys::HtmlInputElement,
}

impl Controls {
    /// Draws the selectors, the buttons and the range in `scene`. The size and the speed that are chosen at first are
    /// `size` and `speed`, and the range goes from 0 to `last`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be made or added.
    pub(super) fn build(
        document: &web_sys::Document,
        scene: &web_sys::Element,
        size: usize,
        speed: u32,
        last: usize,
    ) -> Result<Self, String> {
        let layer = make(document, "g", &[("class", "nq-controls")])?;
        add(scene, &layer)?;
        let size_select = add_selector(
            document,
            &layer,
            SIZE_SELECTOR,
            ("Board size", SIZE_ID),
            &SIZES.map(|n| (n.to_string(), format!("{n} by {n}"))),
            &size.to_string(),
        )?;
        let speed_select = add_selector(
            document,
            &layer,
            SPEED_SELECTOR,
            ("Speed", SPEED_ID),
            &SPEEDS.map(|n| {
                (
                    n.to_string(),
                    format!("{n} {} a second", if n == 1 { "event" } else { "events" }),
                )
            }),
            &speed.to_string(),
        )?;
        for (index, &(action, label, _)) in BUTTONS.iter().enumerate() {
            let button = Button {
                label,
                action,
                colours: BUTTON,
                pressed: None,
            };
            svg_buttons::add_button(document, &layer, button_area(index), &button)?;
            // The tests, and a person who inspects the page, find a button by the name that it had as HTML.
            find_button(&layer, action)?
                .set_attribute("id", &format!("n-queens-{action}"))
                .map_err(|e| format!("could not name the button {action:?}: {e:?}"))?;
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
        sliders::add_slider(document, scene, &range, 0.0, &format!("Step 0 of {last}"))?;
        let seek = scene
            .query_selector(&format!("input[data-slider=\"{SEEK}\"]"))
            .map_err(|e| format!("could not look for the range: {e:?}"))?
            .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
            .ok_or_else(|| "the range is not an <input>".to_owned())?;
        seek.set_id(SEEK_ID);
        Ok(Self {
            scene: scene.clone(),
            layer,
            size: size_select,
            speed: speed_select,
            seek,
        })
    }

    /// The selector for the size of the board.
    pub(super) fn size_selector(&self) -> &web_sys::HtmlSelectElement {
        &self.size
    }

    /// The selector for the speed.
    pub(super) fn speed_selector(&self) -> &web_sys::HtmlSelectElement {
        &self.speed
    }

    /// The range that goes to a step, whose `input` and `change` events the page listens to.
    pub(super) fn seek(&self) -> &web_sys::HtmlInputElement {
        &self.seek
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes on the Play button what it will do next.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button is missing, or an attribute could not be set.
    pub(super) fn set_playing(&self, playing: bool) -> Result<(), String> {
        svg_buttons::set_label(&self.layer, "play", if playing { "Pause" } else { "Play" })
    }

    /// Dims the button for `action`, or makes it plain again.
    ///
    /// A dimmed button is `aria-disabled`, and keeps its place in the tab order. So a button that has the focus when
    /// the search reaches an end keeps it, and a press of it is still heard, which draws a page that is out of date.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button is missing, or an attribute could not be set.
    pub(super) fn dim(&self, action: &str, dimmed: bool) -> Result<(), String> {
        let button = find_button(&self.layer, action)?;
        set(&button, "aria-disabled", if dimmed { "true" } else { "false" })?;
        set(
            &button,
            "style",
            if dimmed { "cursor: default; opacity: 0.45;" } else { "cursor: pointer;" },
        )
    }

    /// Puts the range at `index`, and writes `shown` above it and as its value for assistive technology.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be set.
    pub(super) fn set_seek(&self, index: usize, shown: &str) -> Result<(), String> {
        sliders::set_value(&self.scene, SEEK, index as f64, shown)
    }

    /// Makes the range go from 0 to `last`, for a board of another size.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be set.
    pub(super) fn set_last(&self, last: usize) -> Result<(), String> {
        sliders::set_range(&self.scene, SEEK, (0.0, last as f64))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The button in `layer` whose action is `action`.
fn find_button(layer: &web_sys::Element, action: &str) -> Result<web_sys::Element, String> {
    layer
        .query_selector(&format!("[data-action=\"{action}\"]"))
        .map_err(|e| format!("could not look for the button {action:?}: {e:?}"))?
        .ok_or_else(|| format!("there is no button {action:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a selector in `layer`: its name written beside it, and the browser's own `<select>` in a `<foreignObject>`.
/// `name` is the text and the id, `options` are value and text, and `chosen` is the value that starts chosen.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
fn add_selector(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    place: SelectorPlace,
    name: (&str, &str),
    options: &[(String, String)],
    chosen: &str,
) -> Result<web_sys::HtmlSelectElement, String> {
    let (label, id) = name;
    let area = place.area;
    let text = make(
        document,
        "text",
        &[
            ("x", &place.name_left.to_string()),
            ("y", &(area.origin.y + ROW_HEIGHT / 2.0).to_string()),
            ("dominant-baseline", "central"),
            ("font-size", "14"),
            ("fill", "#1a1a1a"),
            ("aria-hidden", "true"),
            ("style", "pointer-events: none; user-select: none;"),
        ],
    )?;
    text.set_text_content(Some(label));
    add(layer, &text)?;
    let object = make(
        document,
        "foreignObject",
        &[
            ("x", &area.origin.x.to_string()),
            ("y", &area.origin.y.to_string()),
            ("width", &area.size.width.to_string()),
            ("height", &area.size.height.to_string()),
        ],
    )?;
    let select = document
        .create_element_ns(Some(XHTML), "select")
        .map_err(|e| format!("could not make a <select>: {e:?}"))?
        .dyn_into::<web_sys::HtmlSelectElement>()
        .map_err(|_| "a selector is not a <select>".to_owned())?;
    for (name, value) in [
        ("id", id),
        ("aria-label", label),
        ("style", "width: 100%; height: 100%; box-sizing: border-box; font: inherit;"),
    ] {
        set(&select, name, value)?;
    }
    for (value, shown) in options {
        let option = document
            .create_element_ns(Some(XHTML), "option")
            .map_err(|e| format!("could not make an <option>: {e:?}"))?;
        set(&option, "value", value)?;
        option.set_text_content(Some(shown));
        add(&select, &option)?;
    }
    add(&object, &select)?;
    add(layer, &object)?;
    select.set_value(chosen);
    Ok(select)
}
