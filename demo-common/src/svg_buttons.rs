//! Buttons drawn as SVG in a scene's own layer, so they zoom and pan with it, and the listeners that make them work.
//!
//! Used by the demos that put their own controls in the scene beside the library's nodes.

use crate::util::required_element;
use std::rc::Rc;
use svg_dom::root::utils::Rect;
use wasm_bindgen::{JsCast, closure::Closure};

/// The XML namespace SVG elements are created in.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Describes all the attributes of a button: what it is called, what it does, and how it looks etc
pub struct Button<'a> {
    /// Its text, and its accessible name.
    pub label: &'a str,
    /// What [`watch`] reports when it is pressed.
    pub action: &'a str,
    /// Its fill and its outline.
    pub colours: (&'a str, &'a str),
    /// `Some(on)` makes it a toggle button, and sets `aria-pressed` to say which state it is in. `None` is a plain
    /// button.
    pub pressed: Option<bool>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layer of the scene drawn in `#diagram_id`, which a button is added to.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer.
pub fn layer(document: &web_sys::Document, diagram_id: &str) -> Result<web_sys::Element, String> {
    document
        .query_selector(&format!("#{diagram_id} > g.svg-dom-graph-content"))
        .map_err(|e| format!("could not look for the scene's layer: {e:?}"))?
        .ok_or_else(|| "the scene has no content layer".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws `button` in the scene's own layer `layer`, in `area`, that [`watch`] will find by its action. It is a
/// rectangle with a label, in a group that has the role of a button and can take the keyboard focus.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
pub fn add_button(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    area: Rect,
    button: &Button,
) -> Result<(), String> {
    let (top_left, size) = (area.origin, area.size);
    let Button {
        label,
        action,
        colours: (fill, stroke),
        pressed,
    } = *button;
    let make = |tag: &str, attributes: &[(&str, String)]| -> Result<web_sys::Element, String> {
        let element = document
            .create_element_ns(Some(SVG_NAMESPACE), tag)
            .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
        for (name, value) in attributes {
            element
                .set_attribute(name, value)
                .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
        }
        Ok(element)
    };
    let mut attributes = vec![
        ("role", "button".to_owned()),
        ("tabindex", "0".to_owned()),
        ("aria-label", label.to_owned()),
        ("data-action", action.to_owned()),
        ("style", "cursor: pointer;".to_owned()),
    ];
    if let Some(on) = pressed {
        attributes.push(("aria-pressed", on.to_string()));
    }
    let group = make("g", &attributes)?;
    let rect = make(
        "rect",
        &[
            ("x", top_left.x.to_string()),
            ("y", top_left.y.to_string()),
            ("width", size.width.to_string()),
            ("height", size.height.to_string()),
            ("rx", "6".to_owned()),
            ("fill", fill.to_owned()),
            ("stroke", stroke.to_owned()),
            ("stroke-width", "1.5".to_owned()),
        ],
    )?;
    let text = make(
        "text",
        &[
            ("x", (top_left.x + size.width / 2.0).to_string()),
            ("y", (top_left.y + size.height / 2.0).to_string()),
            ("text-anchor", "middle".to_owned()),
            ("dominant-baseline", "central".to_owned()),
            ("font-size", "14".to_owned()),
            ("fill", "#1a1a1a".to_owned()),
            ("style", "pointer-events: none; user-select: none;".to_owned()),
        ],
    )?;
    text.set_text_content(Some(label));
    for (parent, child) in [(&group, &rect), (&group, &text), (layer, &group)] {
        parent
            .append_child(child)
            .map_err(|e| format!("could not add the button: {e:?}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The button in `layer` whose action is `action`.
fn find(layer: &web_sys::Element, action: &str) -> Result<web_sys::Element, String> {
    layer
        .query_selector(&format!("[data-action=\"{action}\"]"))
        .map_err(|e| format!("could not look for the button {action:?}: {e:?}"))?
        .ok_or_else(|| format!("there is no button {action:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a toggle button as pressed or not: its `aria-pressed`, and the fill and outline `colours` it is now drawn in.
///
/// # Errors
///
/// Returns `Err` if `layer` has no button with that action, or an attribute could not be set.
pub fn set_pressed(layer: &web_sys::Element, action: &str, pressed: bool, colours: (&str, &str)) -> Result<(), String> {
    let button = find(layer, action)?;
    button
        .set_attribute("aria-pressed", &pressed.to_string())
        .map_err(|e| format!("could not set aria-pressed on {action:?}: {e:?}"))?;
    if let Some(rect) = button
        .query_selector("rect")
        .map_err(|e| format!("could not find the rectangle of {action:?}: {e:?}"))?
    {
        for (name, value) in [("fill", colours.0), ("stroke", colours.1)] {
            rect.set_attribute(name, value)
                .map_err(|e| format!("could not set {name} on {action:?}: {e:?}"))?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `label` on a button as its text and its accessible name, for a button whose meaning changes, such as one that
/// says "Pause" and then "Play".
///
/// # Errors
///
/// Returns `Err` if `layer` has no button with that action, or an attribute could not be set.
pub fn set_label(layer: &web_sys::Element, action: &str, label: &str) -> Result<(), String> {
    let button = find(layer, action)?;
    button
        .set_attribute("aria-label", label)
        .map_err(|e| format!("could not set aria-label on {action:?}: {e:?}"))?;
    if let Some(text) = button
        .query_selector("text")
        .map_err(|e| format!("could not find the text of {action:?}: {e:?}"))?
    {
        text.set_text_content(Some(label));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dims a button and takes it out of the keyboard's tab order, or restores it. A disabled button still receives
/// clicks, so the handler given to [`watch`] must ignore its action while the button is disabled.
///
/// # Errors
///
/// Returns `Err` if `layer` has no button with that action, or an attribute could not be set.
pub fn set_disabled(layer: &web_sys::Element, action: &str, disabled: bool) -> Result<(), String> {
    let button = find(layer, action)?;
    let (tab_index, style) = if disabled {
        ("-1", "cursor: default; opacity: 0.45;")
    } else {
        ("0", "cursor: pointer;")
    };
    for (name, value) in [
        ("aria-disabled", if disabled { "true" } else { "false" }),
        ("tabindex", tab_index),
        ("style", style),
    ] {
        button
            .set_attribute(name, value)
            .map_err(|e| format!("could not set {name} on {action:?}: {e:?}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons drawn by [`add_button`] work. One listener for clicks and one for the Enter and Space keys are on
/// the stage `stage_id`, which outlives every scene that is drawn in it, and find the button by its `data-action`. So a
/// scene drawn again needs no listeners of its own. `on_press` is called with the action of the button.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
pub fn watch(document: &web_sys::Document, stage_id: &str, on_press: impl Fn(&str) + 'static) -> Result<(), String> {
    let stage = required_element(document, stage_id)?;
    let on_press = Rc::new(on_press);
    let action_of = |target: Option<web_sys::EventTarget>| {
        target
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-action]").ok().flatten())
            .and_then(|button| button.get_attribute("data-action"))
    };
    let on_click = Rc::clone(&on_press);
    let clicked = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        if let Some(action) = action_of(event.target()) {
            on_click(&action);
        }
    });
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" || event.key() == " " {
            if let Some(action) = action_of(event.target()) {
                event.prevent_default();
                on_press(&action);
            }
        }
    });
    stage
        .add_event_listener_with_callback("click", clicked.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the click listener: {e:?}"))?;
    stage
        .add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the keydown listener: {e:?}"))?;
    clicked.forget();
    key.forget();
    Ok(())
}
