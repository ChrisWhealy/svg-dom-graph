//! Sliders drawn as SVG in a scene's own layer, so they zoom and pan with it, and the listeners that make them work.
//!
//! Used by the demos that put their own controls in the scene beside the library's nodes. A slider is a group with the
//! role of a slider, which the keyboard can reach. The pointer drags its thumb, and the arrow, page, Home and End keys
//! move it, as they do a slider of the browser's. Nothing here knows what a slider is for: the function given to
//! [`watch`] is told which slider moved and to what, and says what that means.

use crate::util::required_element;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};

/// The XML namespace SVG elements are created in.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The height of the track, the radius of the thumb, and how far above the track the label and the value are, in the
/// units of the scene.
const TRACK_HEIGHT: f64 = 6.0;
const THUMB_RADIUS: f64 = 9.0;
const TEXT_RISE: f64 = 16.0;

/// How many steps a press of Page Up or Page Down moves a slider.
const BIG_STEPS: f64 = 10.0;

/// The colours of the track, of the part of it up to the thumb, and of the thumb.
const TRACK: &str = "#cbd5e1";
const FILL: &str = "#2563eb";
const THUMB: &str = "#ffffff";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Describes a slider: what it is called, where it is, and what values it takes.
pub struct Slider<'a> {
    /// What it is called. It is drawn above the left end of the track, and is the accessible name.
    pub label: &'a str,
    /// Names the slider for [`set_value`] and for the function given to [`watch`].
    pub id: &'a str,
    /// The left end of the track, and the height of its middle, in the units of the scene.
    pub left: f64,
    pub middle: f64,
    /// How long the track is.
    pub width: f64,
    /// The least and the greatest value, and the distance between the values that it can take.
    pub range: (f64, f64),
    pub step: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `value` as the nearest value that the slider can take: a whole number of steps from the least, and not beyond
/// either end. A value that is not a number is the least.
#[must_use]
pub fn snap(range: (f64, f64), step: f64, value: f64) -> f64 {
    let (least, greatest) = range;
    if !value.is_finite() || step.is_nan() || step <= 0.0 {
        return least;
    }
    let steps = ((value - least) / step).round();
    // Rounded to a few places, so that steps of 0.1 give 0.3 and not 0.30000000000000004.
    let snapped = ((least + steps * step) * 1e6).round() / 1e6;
    snapped.clamp(least, greatest)
}

/// The value of the slider for a pointer that is `fraction` of the way along its track, where 0 is the left end and 1
/// is the right. A pointer beyond either end gives that end.
#[must_use]
pub fn value_at(range: (f64, f64), step: f64, fraction: f64) -> f64 {
    let fraction = if fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.0 };
    snap(range, step, range.0 + fraction * (range.1 - range.0))
}

/// How far along its track, from 0 to 1, a slider with `value` has its thumb.
#[must_use]
pub fn fraction_of(range: (f64, f64), value: f64) -> f64 {
    let span = range.1 - range.0;
    if span > 0.0 { ((value - range.0) / span).clamp(0.0, 1.0) } else { 0.0 }
}

/// The value that `key` takes a slider from `value` to, or `None` if the key does nothing to it. The arrow keys move it
/// a step, Right and Up to more and Left and Down to less. Page Up and Page Down move it ten steps. Home and End take
/// it to the least and the greatest.
#[must_use]
pub fn key_value(key: &str, range: (f64, f64), step: f64, value: f64) -> Option<f64> {
    let moved = match key {
        "ArrowRight" | "ArrowUp" => value + step,
        "ArrowLeft" | "ArrowDown" => value - step,
        "PageUp" => value + BIG_STEPS * step,
        "PageDown" => value - BIG_STEPS * step,
        "Home" => range.0,
        "End" => range.1,
        _ => return None,
    };
    Some(snap(range, step, moved))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layer of the scene drawn in `#diagram_id`, which a slider is added to.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer.
pub fn layer(document: &web_sys::Document, diagram_id: &str) -> Result<web_sys::Element, String> {
    crate::svg_buttons::layer(document, diagram_id)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes an SVG element called `tag` with `attributes`.
fn make(document: &web_sys::Document, tag: &str, attributes: &[(&str, String)]) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some(SVG_NAMESPACE), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        element
            .set_attribute(name, value)
            .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
    }
    Ok(element)
}

/// Makes a text element at `(x, y)`, with the anchor `anchor`, saying `text`.
fn text(
    document: &web_sys::Document,
    class: &str,
    at: (f64, f64),
    anchor: &str,
    text: &str,
) -> Result<web_sys::Element, String> {
    let element = make(
        document,
        "text",
        &[
            ("class", class.to_owned()),
            ("x", at.0.to_string()),
            ("y", at.1.to_string()),
            ("text-anchor", anchor.to_owned()),
            ("font-size", "14".to_owned()),
            ("fill", "#1a1a1a".to_owned()),
            ("style", "pointer-events: none; user-select: none;".to_owned()),
        ],
    )?;
    element.set_text_content(Some(text));
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws `slider` in the scene's own layer `layer`, with its thumb at `value`, and the text `shown` for that value
/// above the right end of the track. It is a group with the role of a slider that can take the keyboard focus.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
pub fn add_slider(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    slider: &Slider,
    value: f64,
    shown: &str,
) -> Result<(), String> {
    let Slider {
        label,
        id,
        left,
        middle,
        width,
        range,
        step,
    } = *slider;
    let group = make(
        document,
        "g",
        &[
            ("class", "svg-slider".to_owned()),
            ("role", "slider".to_owned()),
            ("tabindex", "0".to_owned()),
            ("aria-label", label.to_owned()),
            ("aria-valuemin", range.0.to_string()),
            ("aria-valuemax", range.1.to_string()),
            ("data-slider", id.to_owned()),
            ("data-min", range.0.to_string()),
            ("data-max", range.1.to_string()),
            ("data-step", step.to_string()),
            ("data-left", left.to_string()),
            ("data-width", width.to_string()),
            ("style", "cursor: pointer; touch-action: none;".to_owned()),
        ],
    )?;
    // Room round the track that the pointer can take hold of, which is invisible.
    let reach = make(
        document,
        "rect",
        &[
            ("x", (left - THUMB_RADIUS).to_string()),
            ("y", (middle - THUMB_RADIUS - 2.0).to_string()),
            ("width", (width + 2.0 * THUMB_RADIUS).to_string()),
            ("height", (2.0 * THUMB_RADIUS + 4.0).to_string()),
            ("fill", "transparent".to_owned()),
        ],
    )?;
    let track = make(
        document,
        "rect",
        &[
            ("x", left.to_string()),
            ("y", (middle - TRACK_HEIGHT / 2.0).to_string()),
            ("width", width.to_string()),
            ("height", TRACK_HEIGHT.to_string()),
            ("rx", (TRACK_HEIGHT / 2.0).to_string()),
            ("fill", TRACK.to_owned()),
        ],
    )?;
    let filled = make(
        document,
        "rect",
        &[
            ("class", "svg-slider-filled".to_owned()),
            ("x", left.to_string()),
            ("y", (middle - TRACK_HEIGHT / 2.0).to_string()),
            ("width", "0".to_owned()),
            ("height", TRACK_HEIGHT.to_string()),
            ("rx", (TRACK_HEIGHT / 2.0).to_string()),
            ("fill", FILL.to_owned()),
        ],
    )?;
    let thumb = make(
        document,
        "circle",
        &[
            ("class", "svg-slider-thumb".to_owned()),
            ("cx", left.to_string()),
            ("cy", middle.to_string()),
            ("r", THUMB_RADIUS.to_string()),
            ("fill", THUMB.to_owned()),
            ("stroke", FILL.to_owned()),
            ("stroke-width", "2".to_owned()),
        ],
    )?;
    let above = middle - THUMB_RADIUS - TEXT_RISE + 12.0;
    let name = text(document, "svg-slider-label", (left, above), "start", label)?;
    let current = text(document, "svg-slider-value", (left + width, above), "end", shown)?;
    for child in [&reach, &track, &filled, &thumb, &name, &current] {
        group
            .append_child(child)
            .map_err(|e| format!("could not add a part of the slider: {e:?}"))?;
    }
    layer
        .append_child(&group)
        .map_err(|e| format!("could not add the slider: {e:?}"))?;
    set_value(layer, id, value, shown)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The slider in `layer` whose id is `id`.
fn find(layer: &web_sys::Element, id: &str) -> Result<web_sys::Element, String> {
    layer
        .query_selector(&format!("[data-slider=\"{id}\"]"))
        .map_err(|e| format!("could not look for the slider {id:?}: {e:?}"))?
        .ok_or_else(|| format!("there is no slider {id:?}"))
}

/// A number written in an attribute of `element`.
fn number(element: &web_sys::Element, name: &str) -> Result<f64, String> {
    element
        .get_attribute(name)
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| format!("the slider has no number in {name}"))
}

/// The range, the step, the left end and the length of the track of the slider `group`.
fn geometry(group: &web_sys::Element) -> Result<((f64, f64), f64, f64, f64), String> {
    Ok((
        (number(group, "data-min")?, number(group, "data-max")?),
        number(group, "data-step")?,
        number(group, "data-left")?,
        number(group, "data-width")?,
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the thumb of the slider `id` at `value`, writes `shown` as the text of its value, and says the value to
/// assistive technology.
///
/// # Errors
///
/// Returns `Err` if `layer` has no slider with that id, or an attribute could not be set.
pub fn set_value(layer: &web_sys::Element, id: &str, value: f64, shown: &str) -> Result<(), String> {
    let group = find(layer, id)?;
    let (range, _, left, width) = geometry(&group)?;
    let x = left + fraction_of(range, value) * width;
    let part = |class: &str| -> Result<web_sys::Element, String> {
        group
            .query_selector(&format!(".{class}"))
            .map_err(|e| format!("could not look for a part of the slider {id:?}: {e:?}"))?
            .ok_or_else(|| format!("the slider {id:?} has no {class}"))
    };
    let set = |element: &web_sys::Element, name: &str, text: String| -> Result<(), String> {
        element
            .set_attribute(name, &text)
            .map_err(|e| format!("could not set {name} on the slider {id:?}: {e:?}"))
    };
    set(&part("svg-slider-thumb")?, "cx", x.to_string())?;
    set(&part("svg-slider-filled")?, "width", (x - left).to_string())?;
    set(&group, "aria-valuenow", value.to_string())?;
    set(&group, "aria-valuetext", shown.to_owned())?;
    set(&group, "data-value", value.to_string())?;
    part("svg-slider-value")?.set_text_content(Some(shown));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The slider that `target` is part of, if it is part of one.
fn slider_of(target: Option<web_sys::EventTarget>) -> Option<web_sys::Element> {
    target
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|e| e.closest("[data-slider]").ok().flatten())
}

/// The value of `group` for a pointer at `client_x`, in the units of the scene it is in. A scene is never zoomed or
/// panned here, so that is the position in the `<svg>`, scaled by however much the page has shrunk it.
fn value_for_pointer(group: &web_sys::Element, client_x: f64) -> Result<f64, String> {
    let (range, step, left, width) = geometry(group)?;
    let svg = group.closest("svg").ok().flatten().ok_or("the slider is not in a scene")?;
    let rect = svg.get_bounding_client_rect();
    let units = svg
        .get_attribute("width")
        .and_then(|w| w.parse::<f64>().ok())
        .unwrap_or(rect.width());
    let scale = if rect.width() > 0.0 { units / rect.width() } else { 1.0 };
    Ok(value_at(range, step, ((client_x - rect.left()) * scale - left) / width))
}

/// Makes the sliders drawn by [`add_slider`] work. Listeners on the stage `stage_id`, which outlives every scene that
/// is drawn in it, find the slider by its `data-slider`. So a scene drawn again needs no listeners of its own.
/// `on_change` is called with the id of the slider and the value that a person has moved it to, each time they do. It
/// is to draw that, and to call [`set_value`].
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
pub fn watch(
    document: &web_sys::Document,
    stage_id: &str,
    on_change: impl Fn(&str, f64) + 'static,
) -> Result<(), String> {
    let stage = required_element(document, stage_id)?;
    let on_change = Rc::new(on_change);
    // The slider that the pointer is holding, if it is holding one.
    let held: Rc<RefCell<Option<web_sys::Element>>> = Rc::new(RefCell::new(None));

    let (changed, holding) = (Rc::clone(&on_change), Rc::clone(&held));
    let down = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |event: web_sys::PointerEvent| {
        let Some(group) = slider_of(event.target()) else { return };
        event.prevent_default();
        let _ = group.set_pointer_capture(event.pointer_id());
        if let (Ok(value), Some(id)) = (
            value_for_pointer(&group, f64::from(event.client_x())),
            group.get_attribute("data-slider"),
        ) {
            changed(&id, value);
        }
        if let Some(element) = group.dyn_ref::<web_sys::SvgElement>() {
            let _ = element.focus();
        }
        *holding.borrow_mut() = Some(group);
    });
    let (changed, holding) = (Rc::clone(&on_change), Rc::clone(&held));
    let moved = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |event: web_sys::PointerEvent| {
        let Some(group) = holding.borrow().clone() else { return };
        if let (Ok(value), Some(id)) = (
            value_for_pointer(&group, f64::from(event.client_x())),
            group.get_attribute("data-slider"),
        ) {
            changed(&id, value);
        }
    });
    let holding = Rc::clone(&held);
    let up = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |event: web_sys::PointerEvent| {
        if let Some(group) = holding.borrow_mut().take() {
            let _ = group.release_pointer_capture(event.pointer_id());
        }
    });
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
        let Some(group) = slider_of(event.target()) else { return };
        let (Ok((range, step, _, _)), Some(id), Ok(value)) = (
            geometry(&group),
            group.get_attribute("data-slider"),
            number(&group, "data-value"),
        ) else {
            return;
        };
        if let Some(moved_to) = key_value(&event.key(), range, step, value) {
            event.prevent_default();
            on_change(&id, moved_to);
        }
    });
    for (event, listener) in [
        ("pointerdown", down.as_ref().unchecked_ref()),
        ("pointermove", moved.as_ref().unchecked_ref()),
        ("pointerup", up.as_ref().unchecked_ref()),
        ("pointercancel", up.as_ref().unchecked_ref()),
        ("keydown", key.as_ref().unchecked_ref()),
    ] {
        stage
            .add_event_listener_with_callback(event, listener)
            .map_err(|e| format!("could not attach the {event} listener: {e:?}"))?;
    }
    down.forget();
    moved.forget();
    up.forget();
    key.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
