//! Range sliders drawn inside a scene, in a `<foreignObject>` of the scene's own `<svg>`, and the listeners that make
//! them work.
//!
//! A slider is the browser's own `<input type="range">`. So the browser does what it already does well: dragging with a
//! pointer or a finger, the keyboard, and the accessibility tree, with the role of a slider and its value. What this
//! module adds is where the slider goes in a scene, and how it looks.
//!
//! There are two ways to add one. [`add_range`] puts a slider along an axis of a diagram, as long as the axis, with no
//! writing on it, and hands the input to the caller, who listens to it. [`add_slider`] puts a slider with its name and
//! its value written above it, found again by its id, which [`watch`] listens to for every slider of a stage at once.
//!
//! What makes a slider as long as an axis is the arithmetic in [`frame`], which touches no DOM and is tested natively.
//! A native slider's thumb travels from half its own width inside one end of the element to half its own width inside
//! the other. So the element is made one thumb wider than the axis, and then the centre of the thumb travels exactly
//! the length of the axis, from one end to the other, and a value is beneath the point of the axis it stands for. The
//! thumb is given a fixed width by the style in [`style_sheet`], since the browsers' own differ. That style is put in
//! the page by this module, with the width that [`THUMB`] says, and not kept in `style.css`: the track and the thumb
//! are what make the slider as long as its axis, so they must always come from the same place as the length does, and a
//! stylesheet that a browser has kept from an earlier visit must not be able to change them.
//!
//! A vertical slider is the same element turned a quarter turn anticlockwise about its centre, so that its minimum is
//! at the bottom. A turn is something every browser can do. `writing-mode` on a range input is not.
//!
//! A demo that draws its scene again, for every move of a slider, draws the slider again with it, and a slider that is
//! taken away while a person is dragging it stops being dragged. So [`add_slider`] puts the slider in a group of its
//! own, directly in the `<svg>`, and [`clear_scene`] takes everything out of the `<svg>` but those groups. The slider
//! is then the same element before the scene is drawn again and after, and the drag and the keyboard focus carry on.

use super::required_element;
use std::cell::RefCell;
use svg_dom::root::utils::{Point, Size};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlInputElement;

/// The width of a slider's thumb, which [`style_sheet`] sets for every `.scene-slider`, in user units.
pub const THUMB: f64 = 16.0;

/// The thickness of a slider: its thumb, and a little room above and below it.
pub const BAND: f64 = 24.0;

/// How far above the middle of the track the name and the value of a slider with writing are, in user units.
const TEXT_RISE: f64 = 13.0;

/// The colour of a thumb that is not given one.
const THUMB_COLOUR: &str = "#2b6cb0";

/// The id of the `<style>` element that [`ensure_style`] makes.
const STYLE_ID: &str = "scene-slider-style";

/// The XML namespaces that SVG and HTML elements are made in.
const NS_SVG: &str = "http://www.w3.org/2000/svg";
const NS_XHTML: &str = "http://www.w3.org/1999/xhtml";

/// A listener for the `input` event of a control.
pub type Listener = Closure<dyn FnMut(web_sys::Event)>;

thread_local! {
    // The listeners of every slider made by `add_range`, kept for as long as the page is, since the sliders are.
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which way a slider lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Left to right, with the minimum on the left.
    Horizontal,
    /// Bottom to top, with the minimum at the bottom.
    Vertical,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a slider's `<foreignObject>` goes: its corner and size before any turn, and the turn if there is one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub top_left: Point,
    pub size: Size,
    /// The angle in degrees the element is turned by, clockwise as SVG counts, and the point it is turned about.
    pub turn: Option<(f64, Point)>,
}

/// The frame of a slider whose thumb travels `axis_length` user units, centred on `axis_centre`.
#[must_use]
pub fn frame(axis_centre: Point, axis_length: f64, orientation: Orientation) -> Frame {
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
/// `value` as the nearest multiple of `step`, and never a negative zero. A step that is not positive and finite leaves
/// the value as it is.
#[must_use]
pub fn snapped(value: f64, step: f64) -> f64 {
    if !(step.is_finite() && step > 0.0 && value.is_finite()) {
        return value;
    }
    // Adding positive zero turns a negative zero into a positive one.
    (value / step).round() * step + 0.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The style of every slider, as the text of a `<style>` element. The thumb is exactly [`THUMB`] wide in every browser,
/// which is what puts it under the value's place on the axis, and the track is drawn as the input's own background, a
/// thumb shorter than the input, so it is exactly as long as the axis and the thumb never sticks out past either end of
/// it.
///
/// The colour of a thumb is the custom property `--thumb` of its input, which [`Spec::colour`] sets.
#[must_use]
pub fn style_sheet() -> String {
    let thumb = THUMB;
    let colour = format!("var(--thumb, {THUMB_COLOUR})");
    format!(
        "\
.scene-slider {{ -webkit-appearance: none; appearance: none; box-sizing: border-box; width: 100%; height: 100%; \
margin: 0; padding: 0; cursor: pointer; \
background: linear-gradient(#9aa5c4, #9aa5c4) 50% 50% / calc(100% - {thumb}px) 4px no-repeat; }}
.scene-slider::-webkit-slider-runnable-track {{ height: {thumb}px; background: transparent; }}
.scene-slider::-moz-range-track {{ height: {thumb}px; background: transparent; }}
.scene-slider::-webkit-slider-thumb {{ -webkit-appearance: none; appearance: none; box-sizing: border-box; \
width: {thumb}px; height: {thumb}px; margin-top: 0; border: 2px solid #ffffff; border-radius: 50%; \
background: {colour}; box-shadow: 0 0 0 1px #4a5568; }}
.scene-slider::-moz-range-thumb {{ box-sizing: border-box; width: {thumb}px; height: {thumb}px; \
border: 2px solid #ffffff; border-radius: 50%; background: {colour}; box-shadow: 0 0 0 1px #4a5568; }}
.scene-slider:focus-visible {{ outline: none; }}
.scene-slider:focus-visible::-webkit-slider-thumb {{ outline: 2px solid #1a202c; outline-offset: 2px; }}
.scene-slider:focus-visible::-moz-range-thumb {{ outline: 2px solid #1a202c; outline-offset: 2px; }}
.scene-slider:disabled {{ cursor: default; opacity: 0.45; }}
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
/// Makes an element of the namespace `namespace`, called `tag`, with `attributes`.
fn make(
    document: &web_sys::Document,
    namespace: &str,
    tag: &str,
    attributes: &[(&str, String)],
) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some(namespace), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        element
            .set_attribute(name, value)
            .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
    }
    Ok(element)
}

/// What a slider is for.
pub struct Spec<'a> {
    /// Its accessible name.
    pub label: &'a str,
    /// A class on the `<input>`, besides `scene-slider`, for a caller that wants to find it again.
    pub class: &'a str,
    /// The colour of its thumb, as any CSS colour, or `None` for the usual blue.
    pub colour: Option<&'a str>,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub value: f64,
}

/// A `<foreignObject>` for `frame`, with an `<input type="range">` in it for `spec`, which is not yet in a scene.
///
/// # Errors
///
/// Returns `Err` if an element cannot be made.
fn input_in_box(
    document: &web_sys::Document,
    frame: Frame,
    spec: &Spec<'_>,
) -> Result<(web_sys::Element, HtmlInputElement), String> {
    ensure_style(document)?;
    let mut attributes = vec![
        ("x", frame.top_left.x.to_string()),
        ("y", frame.top_left.y.to_string()),
        ("width", frame.size.width.to_string()),
        ("height", frame.size.height.to_string()),
    ];
    if let Some((degrees, about)) = frame.turn {
        attributes.push(("transform", format!("rotate({degrees} {} {})", about.x, about.y)));
    }
    let object = make(document, NS_SVG, "foreignObject", &attributes)?;
    let input = make(
        document,
        NS_XHTML,
        "input",
        &[
            ("type", "range".to_owned()),
            ("class", format!("scene-slider {}", spec.class).trim_end().to_owned()),
            ("aria-label", spec.label.to_owned()),
            ("min", spec.min.to_string()),
            ("max", spec.max.to_string()),
            ("step", spec.step.to_string()),
            ("value", spec.value.to_string()),
            ("aria-valuetext", format!("{:.1}", spec.value)),
            (
                "style",
                spec.colour.map_or_else(String::new, |colour| format!("--thumb: {colour};")),
            ),
        ],
    )?
    .dyn_into::<HtmlInputElement>()
    .map_err(|_| "a slider is not an <input>".to_owned())?;
    object
        .append_child(&input)
        .map_err(|e| format!("could not put a slider in its box: {e:?}"))?;
    Ok((object, input))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a slider to `parent`, laid out as `frame` says, and calls `on_input` with the slider every time it changes. It
/// has no writing of its own, so it is for a slider that lies along an axis that is named already.
///
/// # Errors
///
/// Returns `Err` if an element cannot be made or attached.
pub fn add_range(
    document: &web_sys::Document,
    parent: &web_sys::Element,
    frame: Frame,
    spec: &Spec<'_>,
    mut on_input: impl FnMut(&HtmlInputElement) + 'static,
) -> Result<HtmlInputElement, String> {
    let (object, input) = input_in_box(document, frame, spec)?;
    parent
        .append_child(&object)
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
/// Describes a slider with its name and its value written above it: what it is called, where it is, and what values it
/// takes.
pub struct Slider<'a> {
    /// What it is called. It is drawn above the left end of the track, and is the accessible name.
    pub label: &'a str,
    /// Names the slider for the functions of this module and for the function given to [`watch`].
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

/// The `<svg>` of the scene drawn in `#svg_id`, which [`add_slider`] adds a slider to.
///
/// # Errors
///
/// Returns `Err` if the page has no such element.
pub fn scene(document: &web_sys::Document, svg_id: &str) -> Result<web_sys::Element, String> {
    required_element(document, svg_id)
}

/// Makes a text element at `(x, y)` that says `text`, and does not take the pointer.
fn text(
    document: &web_sys::Document,
    class: &str,
    at: (f64, f64),
    anchor: &str,
    text: &str,
) -> Result<web_sys::Element, String> {
    let element = make(
        document,
        NS_SVG,
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

/// Draws `slider` in `scene`, which is the `<svg>` that [`scene`] gives, with its thumb at `value`, and the text
/// `shown` for that value above the right end of the track. It is a group of its own, directly in the `<svg>`, so that
/// it stays when [`clear_scene`] draws the scene again. Adding a slider that is there already changes nothing.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
pub fn add_slider(
    document: &web_sys::Document,
    scene: &web_sys::Element,
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
    if host_of(scene, id).is_some() {
        return Ok(());
    }
    let host = make(
        document,
        NS_SVG,
        "g",
        &[("class", "scene-slider-host".to_owned()), ("data-slider-host", id.to_owned())],
    )?;
    let above = middle - TEXT_RISE;
    let name = text(document, "scene-slider-label", (left, above), "start", label)?;
    let current = text(document, "scene-slider-value", (left + width, above), "end", shown)?;
    let (object, input) = input_in_box(
        document,
        frame(Point::new(left + width / 2.0, middle), width, Orientation::Horizontal),
        &Spec {
            label,
            class: "",
            colour: None,
            min: range.0,
            max: range.1,
            step,
            value,
        },
    )?;
    input
        .set_attribute("data-slider", id)
        .map_err(|e| format!("could not name the slider {id:?}: {e:?}"))?;
    for child in [&name, &current, &object] {
        host.append_child(child)
            .map_err(|e| format!("could not add a part of the slider: {e:?}"))?;
    }
    scene
        .append_child(&host)
        .map_err(|e| format!("could not add the slider: {e:?}"))?;
    set_value(scene, id, value, shown)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The group of the slider `id` in `scene`.
fn host_of(scene: &web_sys::Element, id: &str) -> Option<web_sys::Element> {
    scene.query_selector(&format!("[data-slider-host=\"{id}\"]")).ok().flatten()
}

/// The `<input>` of the slider `id` in `scene`.
fn input_of(scene: &web_sys::Element, id: &str) -> Result<HtmlInputElement, String> {
    scene
        .query_selector(&format!("input[data-slider=\"{id}\"]"))
        .map_err(|e| format!("could not look for the slider {id:?}: {e:?}"))?
        .ok_or_else(|| format!("there is no slider {id:?}"))?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("the slider {id:?} is not an <input>"))
}

/// Puts the thumb of the slider `id` at `value`, writes `shown` as the text of its value, and says it to assistive
/// technology. A person who moves the slider moves the thumb themselves, so this is for the text, and for a demo that
/// moves a slider that is not being held.
///
/// # Errors
///
/// Returns `Err` if `scene` has no slider with that id, or an attribute could not be set.
pub fn set_value(scene: &web_sys::Element, id: &str, value: f64, shown: &str) -> Result<(), String> {
    let input = input_of(scene, id)?;
    input.set_value(&value.to_string());
    input
        .set_attribute("aria-valuetext", shown)
        .map_err(|e| format!("could not set the text of the slider {id:?}: {e:?}"))?;
    host_of(scene, id)
        .and_then(|host| host.query_selector(".scene-slider-value").ok().flatten())
        .ok_or_else(|| format!("the slider {id:?} has no value"))?
        .set_text_content(Some(shown));
    Ok(())
}

/// The value that the slider `id` is at.
///
/// # Errors
///
/// Returns `Err` if `scene` has no slider with that id.
pub fn value(scene: &web_sys::Element, id: &str) -> Result<f64, String> {
    Ok(input_of(scene, id)?.value_as_number())
}

/// Changes the least and the greatest value of the slider `id`, for a slider whose room changes with something else.
/// The browser puts a value that is now beyond an end at that end, without saying so, so the caller is to read the
/// value again, and write it, if it matters.
///
/// # Errors
///
/// Returns `Err` if `scene` has no slider with that id, or an attribute could not be set.
pub fn set_range(scene: &web_sys::Element, id: &str, range: (f64, f64)) -> Result<(), String> {
    let input = input_of(scene, id)?;
    for (name, value) in [("min", range.0), ("max", range.1)] {
        input
            .set_attribute(name, &value.to_string())
            .map_err(|e| format!("could not set {name} on the slider {id:?}: {e:?}"))?;
    }
    Ok(())
}

/// Dims the slider `id` and takes it out of the keyboard's tab order, or restores it. A slider that is dimmed does not
/// answer the pointer or the keys.
///
/// # Errors
///
/// Returns `Err` if `scene` has no slider with that id.
pub fn set_disabled(scene: &web_sys::Element, id: &str, disabled: bool) -> Result<(), String> {
    input_of(scene, id)?.set_disabled(disabled);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the sliders drawn by [`add_slider`] work. One listener on the stage `stage_id`, which outlives every scene
/// that is drawn in it, hears the `input` event of every slider in it. `on_change` is called with the id of the slider
/// and the value that a person has moved it to, each time they do. It is to draw that, and to call [`set_value`] for
/// the text.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or the listener could not be attached.
pub fn watch(
    document: &web_sys::Document,
    stage_id: &str,
    on_change: impl Fn(&str, f64) + 'static,
) -> Result<(), String> {
    let stage = required_element(document, stage_id)?;
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        let Some(input) = event.target().and_then(|target| target.dyn_into::<HtmlInputElement>().ok()) else {
            return;
        };
        if let Some(id) = input.get_attribute("data-slider") {
            on_change(&id, input.value_as_number());
        }
    });
    stage
        .add_event_listener_with_callback("input", listener.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the input listener: {e:?}"))?;
    listener.forget();
    Ok(())
}

/// Takes everything out of `scene`, which is the `<svg>` that [`scene`] gives, but the sliders that [`add_slider`] put
/// there. A demo that draws its scene again calls it in place of emptying the `<svg>`, so that a slider that a person
/// is holding is not taken away from them.
///
/// # Errors
///
/// Returns `Err` if a child could not be taken out.
pub fn clear_scene(scene: &web_sys::Element) -> Result<(), String> {
    let children = scene.children();
    for index in (0..children.length()).rev() {
        if let Some(child) = children.item(index) {
            if !child.class_list().contains("scene-slider-host") {
                child.remove();
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
