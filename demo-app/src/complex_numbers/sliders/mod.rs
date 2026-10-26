//! A range slider drawn inside the scene, along an axis of an Argand diagram, as long as that axis.
//!
//! The slider is an HTML `<input type="range">` in a `<foreignObject>` that is a child of the scene's own content group,
//! so it zooms and pans with the diagram it belongs to, and the browser does what it already does well: dragging, the
//! keyboard, and the accessibility tree.
//!
//! What makes a slider as long as an axis is the arithmetic in [`frame`], which touches no DOM and is tested natively.
//! A native slider's thumb travels from half its own width inside one end of the element to half its own width inside the
//! other. So the element is made one thumb wider than the axis, and then the centre of the thumb travels exactly the
//! length of the axis, from one end to the other, and a value is beneath the point of the axis it stands for. The thumb
//! is given a fixed width by the style in [`style_sheet`], since the browsers' own differ. That style is put in the page
//! by the demo itself, with the width that [`THUMB`] says, and not kept in `style.css`: the track and the thumb are what
//! make the slider as long as its axis, so they must always come from the same place as the length does, and a stylesheet
//! that a browser has kept from an earlier visit must not be able to change them.
//!
//! A vertical slider is the same element turned a quarter turn anticlockwise about its centre, so that its minimum is at
//! the bottom. A turn is something every browser can do. `writing-mode` on a range input is not.

use demo_common::util::stringify;
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlInputElement;

/// The width of a slider's thumb, which [`style_sheet`] sets for every `.argand-slider`, in user units.
pub(super) const THUMB: f64 = 16.0;

/// The thickness of a slider: its thumb, and a little room above and below it.
pub(super) const BAND: f64 = 24.0;

/// The id of the `<style>` element that [`ensure_style`] makes.
const STYLE_ID: &str = "argand-slider-style";

/// The XML namespace HTML elements are created in.
const NS_XHTML: &str = "http://www.w3.org/1999/xhtml";

/// A listener for the `input` event of a control.
pub(super) type Listener = Closure<dyn FnMut(web_sys::Event)>;

thread_local! {
    // The listeners of every slider, kept for as long as the page is, since the sliders are.
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which way a slider lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Orientation {
    /// Left to right, with the minimum on the left.
    Horizontal,
    /// Bottom to top, with the minimum at the bottom.
    Vertical,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a slider's `<foreignObject>` goes: its corner and size before any turn, and the turn if there is one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Frame {
    pub top_left: Point,
    pub size: Size,
    /// The angle in degrees the element is turned by, clockwise as SVG counts, and the point it is turned about.
    pub turn: Option<(f64, Point)>,
}

/// The frame of a slider whose thumb travels `axis_length` user units, centred on `axis_centre`.
pub(super) fn frame(axis_centre: Point, axis_length: f64, orientation: Orientation) -> Frame {
    let size = Size::new(axis_length + THUMB, BAND);
    let top_left = Point::new(axis_centre.x - size.width / 2.0, axis_centre.y - size.height / 2.0);
    Frame {
        top_left,
        size,
        turn: match orientation {
            Orientation::Horizontal => None,
            Orientation::Vertical => Some((-90.0, axis_centre)),
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `value` as the nearest multiple of `step`, and never a negative zero. A step that is not positive and finite leaves the
/// value as it is.
pub(super) fn snapped(value: f64, step: f64) -> f64 {
    if !(step.is_finite() && step > 0.0 && value.is_finite()) {
        return value;
    }
    // Adding positive zero turns a negative zero into a positive one.
    (value / step).round() * step + 0.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The style of every slider, as the text of a `<style>` element. The thumb is exactly [`THUMB`] wide in every browser, which
/// is what puts it under the value's place on the axis, and the track is drawn as the input's own background, a thumb
/// shorter than the input, so it is exactly as long as the axis and the thumb never sticks out past either end of it.
pub(super) fn style_sheet() -> String {
    let thumb = THUMB;
    format!(
        "\
.argand-slider {{ -webkit-appearance: none; appearance: none; box-sizing: border-box; width: 100%; height: 100%; \
margin: 0; padding: 0; cursor: pointer; \
background: linear-gradient(#9aa5c4, #9aa5c4) 50% 50% / calc(100% - {thumb}px) 4px no-repeat; }}
.argand-slider::-webkit-slider-runnable-track {{ height: {thumb}px; background: transparent; }}
.argand-slider::-moz-range-track {{ height: {thumb}px; background: transparent; }}
.argand-slider::-webkit-slider-thumb {{ -webkit-appearance: none; appearance: none; box-sizing: border-box; \
width: {thumb}px; height: {thumb}px; margin-top: 0; border: 2px solid #ffffff; border-radius: 50%; background: #2b6cb0; \
box-shadow: 0 0 0 1px #4a5568; }}
.argand-slider::-moz-range-thumb {{ box-sizing: border-box; width: {thumb}px; height: {thumb}px; \
border: 2px solid #ffffff; border-radius: 50%; background: #2b6cb0; box-shadow: 0 0 0 1px #4a5568; }}
.argand-slider-real::-webkit-slider-thumb {{ background: #2f855a; }}
.argand-slider-real::-moz-range-thumb {{ background: #2f855a; }}
.argand-slider-imaginary::-webkit-slider-thumb {{ background: #805ad5; }}
.argand-slider-imaginary::-moz-range-thumb {{ background: #805ad5; }}
.argand-slider:focus-visible {{ outline: none; }}
.argand-slider:focus-visible::-webkit-slider-thumb {{ outline: 2px solid #1a202c; outline-offset: 2px; }}
.argand-slider:focus-visible::-moz-range-thumb {{ outline: 2px solid #1a202c; outline-offset: 2px; }}
"
    )
}

/// Puts [`style_sheet`] in the page, once. Calling it again does nothing.
///
/// # Errors
///
/// Returns `Err` if the page has no `<head>` or the element cannot be made.
fn ensure_style(document: &web_sys::Document) -> Result<(), String> {
    if document.get_element_by_id(STYLE_ID).is_some() {
        return Ok(());
    }
    let style = document
        .create_element("style")
        .map_err(|e| format!("could not make the slider style: {e:?}"))?;
    style
        .set_attribute("id", STYLE_ID)
        .map_err(|e| format!("could not name the slider style: {e:?}"))?;
    style.set_text_content(Some(&style_sheet()));
    document
        .query_selector("head")
        .map_err(|e| format!("could not find the page's <head>: {e:?}"))?
        .ok_or_else(|| "the page has no <head>".to_owned())?
        .append_child(&style)
        .map(|_| ())
        .map_err(|e| format!("could not put the slider style in the page: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a slider is for.
pub(super) struct Spec<'a> {
    /// Its accessible name.
    pub label: &'a str,
    /// A class on the `<input>` that gives the thumb its colour, besides `argand-slider`.
    pub class: &'a str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub value: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a slider to `parent`, laid out as `frame` says, and calls `on_input` with the slider every time it changes.
///
/// # Errors
///
/// Returns `Err` if an element cannot be made or attached.
pub(super) fn add(
    svg: &SvgRoot,
    parent: &web_sys::Element,
    frame: Frame,
    spec: &Spec<'_>,
    mut on_input: impl FnMut(&HtmlInputElement) + 'static,
) -> Result<HtmlInputElement, String> {
    let fo = svg.foreign_object(frame.top_left, frame.size).map_err(stringify)?;
    if let Some((degrees, about)) = frame.turn {
        fo.set_attr("transform", &format!("rotate({degrees} {} {})", about.x, about.y))
            .map_err(stringify)?;
    }
    let document = fo
        .as_element()
        .owner_document()
        .ok_or_else(|| "a slider's element has no owner document".to_owned())?;
    let input = document
        .create_element_ns(Some(NS_XHTML), "input")
        .map_err(|e| format!("could not make a slider: {e:?}"))?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| "a slider is not an <input>".to_owned())?;
    ensure_style(&document)?;
    input.set_type("range");
    for (name, value) in [
        ("class", format!("argand-slider {}", spec.class)),
        ("aria-label", spec.label.to_owned()),
        ("min", spec.min.to_string()),
        ("max", spec.max.to_string()),
        ("step", spec.step.to_string()),
        ("value", spec.value.to_string()),
        ("aria-valuetext", format!("{:.1}", spec.value)),
    ] {
        input
            .set_attribute(name, &value)
            .map_err(|e| format!("could not set {name} on a slider: {e:?}"))?;
    }
    fo.as_element()
        .append_child(&input)
        .map_err(|e| format!("could not put a slider in its box: {e:?}"))?;
    parent
        .append_child(fo.as_element())
        .map_err(|e| format!("could not put a slider in the scene: {e:?}"))?;

    let target = input.clone();
    let listener: Listener = Closure::new(move |_event: web_sys::Event| on_input(&target));
    input
        .add_event_listener_with_callback("input", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not listen to a slider: {e:?}"))?;
    LISTENERS.with_borrow_mut(|listeners| listeners.push(listener));
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
