//! The controls, drawn in a strip of their own above the panes: a selector for the speed, nine buttons and a range. The
//! buttons and the range are the library's own, and the selector is the browser's own `<select>` in a
//! `<foreignObject>`.
//!
//! Every write to a control goes through a [`Journal`], which remembers what each thing was before, so that a write
//! that fails part way can be put back.

use super::{
    dom::{add, make, set},
    projection::{Bounds, Speed},
    ui_layout::{
        BUTTONS, RANGE_LEFT, RANGE_LENGTH, RANGE_MIDDLE, ROW_HEIGHT, SPEED_AREA, SPEED_NAME_LEFT, button_area,
        fitted_font_px,
    },
};
use svg_dom_graph::controls::{
    sliders::{self, Slider},
    svg_buttons::{self, Button},
};
use wasm_bindgen::JsCast;

/// The ids of the selector and of the range's `<input>`, which the page and the tests find them by.
pub(super) const SPEED_ID: &str = "tj-speed";
pub(super) const SEEK_ID: &str = "tj-seek";

/// What the library's slider calls the range, which is also what it is found by in the strip.
const SEEK: &str = "seek";

/// The fill and the outline of a button, and of a toggle that is on.
const BUTTON: (&str, &str) = ("#ffffff", "#64748b");
const PRESSED: (&str, &str) = ("#ccfbf1", "#0f766e");

/// The XML namespace of HTML elements, which a `<foreignObject>` holds.
const XHTML: &str = "http://www.w3.org/1999/xhtml";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was written to one thing, so that it can be put back.
enum Undo {
    Attr(web_sys::Element, String, Option<String>),
    Text(web_sys::Element, Option<String>),
    Select(web_sys::HtmlSelectElement, String),
    Input(web_sys::HtmlInputElement, String),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A list of what has been written, in order, which is put back in the reverse order.
#[derive(Default)]
pub(super) struct Journal {
    undo: Vec<Undo>,
}

impl Journal {
    /// Sets an attribute, and remembers what it was. A write that fails changes nothing, and is not remembered.
    pub(super) fn attr(&mut self, element: &web_sys::Element, name: &str, value: &str) -> Result<(), String> {
        let old = element.get_attribute(name);
        set(element, name, value)?;
        self.undo.push(Undo::Attr(element.clone(), name.to_owned(), old));
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Sets the text of an element, and remembers what it was.
    pub(super) fn text(&mut self, element: &web_sys::Element, text: &str) {
        self.undo.push(Undo::Text(element.clone(), element.text_content()));
        element.set_text_content(Some(text));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn select(&mut self, select: &web_sys::HtmlSelectElement, value: &str) {
        self.undo.push(Undo::Select(select.clone(), select.value()));
        select.set_value(value);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn input(&mut self, input: &web_sys::HtmlInputElement, value: &str) {
        self.undo.push(Undo::Input(input.clone(), input.value()));
        input.set_value(value);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts everything back as far as it can be, in the reverse order. A put-back that fails is left, and the page is
    /// then repaired by the next action.
    pub(super) fn restore(&mut self) {
        while let Some(undo) = self.undo.pop() {
            match undo {
                Undo::Attr(element, name, Some(old)) => {
                    let _ = element.set_attribute(&name, &old);
                },
                Undo::Attr(element, name, None) => {
                    let _ = element.remove_attribute(&name);
                },
                Undo::Text(element, old) => element.set_text_content(old.as_deref()),
                Undo::Select(select, old) => select.set_value(&old),
                Undo::Input(input, old) => input.set_value(&old),
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controls show, so that only what differs is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ControlState {
    pub(super) bounds: Bounds,
    pub(super) follow: bool,
    pub(super) index: usize,
    pub(super) last: usize,
    pub(super) speed: Speed,
    pub(super) playing: bool,
}

/// The text that the range writes above itself.
pub(super) fn seek_text(state: &ControlState) -> String {
    format!("Step {} of {}", state.index, state.last)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The controls that are drawn in the strip. The strip is where they are found again.
pub(super) struct Controls {
    strip: web_sys::Element,
    layer: web_sys::Element,
    speed: web_sys::HtmlSelectElement,
    seek: web_sys::HtmlInputElement,
}

impl Controls {
    /// Draws the selector, the buttons and the range in `strip`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an element could not be made or added.
    pub(super) fn build(
        document: &web_sys::Document,
        strip: &web_sys::Element,
        state: &ControlState,
    ) -> Result<Self, String> {
        let layer = make(document, "g", &[("class", "tj-controls")])?;
        add(strip, &layer)?;
        let speed = add_selector(document, &layer, state.speed)?;
        for (index, &(action, label, width, toggle)) in BUTTONS.iter().enumerate() {
            let on = action == "follow" && state.follow;
            let button = Button {
                label,
                action,
                colours: if on { PRESSED } else { BUTTON },
                pressed: toggle.then_some(on),
            };
            svg_buttons::add_button(document, &layer, button_area(index), &button)?;
            let element = find_button(&layer, action)?;
            element
                .set_attribute("id", &format!("tj-{action}"))
                .map_err(|e| format!("could not name the button {action:?}: {e:?}"))?;
            // A label that is too long for its button is drawn smaller, so that it stays inside.
            if let Some(text) = element.query_selector("text").ok().flatten() {
                set(
                    &text,
                    "font-size",
                    &fitted_font_px(label.chars().count(), width, 16.0).to_string(),
                )?;
            }
        }
        let range = Slider {
            label: "Go to step",
            id: SEEK,
            left: RANGE_LEFT,
            middle: RANGE_MIDDLE,
            width: RANGE_LENGTH,
            range: (0.0, state.last as f64),
            step: 1.0,
        };
        sliders::add_slider(document, strip, &range, 0.0, &seek_text(state))?;
        let seek = strip
            .query_selector(&format!("input[data-slider=\"{SEEK}\"]"))
            .map_err(|e| format!("could not look for the range: {e:?}"))?
            .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
            .ok_or_else(|| "the range is not an <input>".to_owned())?;
        seek.set_id(SEEK_ID);
        Ok(Self {
            strip: strip.clone(),
            layer,
            speed,
            seek,
        })
    }

    pub(super) fn speed_selector(&self) -> &web_sys::HtmlSelectElement {
        &self.speed
    }

    /// The range that goes to a step, whose `input` and `change` events the page listens to.
    pub(super) fn seek(&self) -> &web_sys::HtmlInputElement {
        &self.seek
    }

    fn button(&self, action: &str) -> Result<web_sys::Element, String> {
        find_button(&self.layer, action)
    }

    fn seek_text_element(&self) -> Result<web_sys::Element, String> {
        self.strip
            .query_selector(&format!("[data-slider-host=\"{SEEK}\"] .scene-slider-value"))
            .ok()
            .flatten()
            .ok_or_else(|| "the range has no value text".to_owned())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes on the Play button what it will do next, which is Pause while the run plays.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button is missing, or an attribute could not be written.
    pub(super) fn write_play(&self, journal: &mut Journal, playing: bool) -> Result<(), String> {
        let label = if playing { "Pause" } else { "Play" };
        let button = self.button("play")?;
        journal.attr(&button, "aria-label", label)?;
        if let Some(text) = button.query_selector("text").ok().flatten() {
            journal.text(&text, label);
        }
        Ok(())
    }

    /// Writes what differs between `prev` and `next` through `journal`, with the end of the range written before its
    /// value.
    ///
    /// A dimmed button is `aria-disabled` and keeps its place in the tab order, so a button that has the focus when the
    /// run reaches an end keeps it, and a press of it is still heard, which draws a page that is out of date.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written. What was written before it is in the journal.
    pub(super) fn write_state(
        &self,
        journal: &mut Journal,
        prev: Option<&ControlState>,
        next: &ControlState,
    ) -> Result<(), String> {
        let (a, b) = (&next.bounds, prev.map(|p| p.bounds));
        let dims: [(&str, bool, Option<bool>); 7] = [
            ("restart", a.restart, b.map(|b| b.restart)),
            ("previous", a.previous, b.map(|b| b.previous)),
            ("next", a.next, b.map(|b| b.next)),
            ("play", a.play, b.map(|b| b.play)),
            ("previous-component", a.previous_component, b.map(|b| b.previous_component)),
            ("next-component", a.next_component, b.map(|b| b.next_component)),
            ("jump-final", a.last, b.map(|b| b.last)),
        ];
        for (action, dimmed, before) in dims {
            if before != Some(dimmed) {
                let button = self.button(action)?;
                journal.attr(&button, "aria-disabled", if dimmed { "true" } else { "false" })?;
                journal.attr(
                    &button,
                    "style",
                    if dimmed { "cursor: default; opacity: 0.45;" } else { "cursor: pointer;" },
                )?;
            }
        }
        if prev.map(|p| p.follow) != Some(next.follow) {
            let button = self.button("follow")?;
            journal.attr(&button, "aria-pressed", if next.follow { "true" } else { "false" })?;
            if let Some(rect) = button.query_selector("rect").ok().flatten() {
                let colours = if next.follow { PRESSED } else { BUTTON };
                journal.attr(&rect, "fill", colours.0)?;
                journal.attr(&rect, "stroke", colours.1)?;
            }
        }
        if prev.map(|p| p.playing) != Some(next.playing) {
            self.write_play(journal, next.playing)?;
        }
        if prev.map(|p| p.speed) != Some(next.speed) {
            journal.select(&self.speed, next.speed.key());
        }
        // The end of the range is written first, since a value beyond the end that it had is clamped to it.
        if prev.map(|p| p.last) != Some(next.last) {
            journal.attr(self.seek.unchecked_ref(), "max", &next.last.to_string())?;
        }
        let text = seek_text(next);
        if prev.map(|p| (p.index, p.last)) != Some((next.index, next.last)) {
            journal.input(&self.seek, &next.index.to_string());
            journal.attr(self.seek.unchecked_ref(), "aria-valuetext", &text)?;
            journal.text(&self.seek_text_element()?, &text);
        }
        Ok(())
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

/// Draws the speed selector in `layer`: its name written beside it, and the browser's own `<select>` in a
/// `<foreignObject>`.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
fn add_selector(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    chosen: Speed,
) -> Result<web_sys::HtmlSelectElement, String> {
    let area = SPEED_AREA;
    let text = make(
        document,
        "text",
        &[
            ("x", &SPEED_NAME_LEFT.to_string()),
            ("y", &(area.origin.y + ROW_HEIGHT / 2.0).to_string()),
            ("dominant-baseline", "central"),
            ("font-size", "14"),
            ("fill", "#1a1a1a"),
            ("aria-hidden", "true"),
            ("style", "pointer-events: none; user-select: none;"),
        ],
    )?;
    text.set_text_content(Some("Speed"));
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
    let longest = Speed::ALL.iter().map(|s| s.label().chars().count()).max().unwrap_or(0);
    for (name, value) in [
        ("id", SPEED_ID.to_owned()),
        ("aria-label", "Speed".to_owned()),
        (
            "style",
            format!(
                "width: 100%; height: 100%; box-sizing: border-box; font: inherit; font-size: {}px;",
                fitted_font_px(longest, area.size.width, 30.0)
            ),
        ),
    ] {
        set(&select, name, &value)?;
    }
    for speed in Speed::ALL {
        let option = document
            .create_element_ns(Some(XHTML), "option")
            .map_err(|e| format!("could not make an <option>: {e:?}"))?;
        set(&option, "value", speed.key())?;
        option.set_text_content(Some(speed.label()));
        add(&select, &option)?;
    }
    add(&object, &select)?;
    add(layer, &object)?;
    select.set_value(chosen.key());
    Ok(select)
}
