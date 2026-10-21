//! The buttons in the scene that introduce an error, and what they do.

use super::{
    ANNOUNCEMENT_ID, DEMO, DIAGRAM_ID, FLIPS, INITIAL_TEXT, INPUT_LEN, STAGE_ID, SVG_NAMESPACE, build, flips::Flips,
    report, walk::Walk,
};
use crate::{
    base64::encode::utf8_prefix,
    util::{document, required_element, stringify},
};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{NodeOptions, Scene},
};
use wasm_bindgen::{JsCast, closure::Closure};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the buttons for introducing an error, and the gap between one and the next.
const BUTTON_WIDTH: f64 = 180.0;
const BUTTON_HEIGHT: f64 = 36.0;
const STATUS_WIDTH: f64 = 300.0;
pub(super) const CONTROL_GAP: f64 = 20.0;

/// The fill and the outline of each button. Flipping a bit is a warning amber and putting it back is a calm green, so
/// neither is mistaken for the light blue of the notes, or for the white of the status line.
const FLIP_COLOURS: (&str, &str) = ("#fde3b0", "#a8650a");
const REMOVE_COLOURS: (&str, &str) = ("#d4edd6", "#2f7a3a");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds the two buttons starting at `left`, centred on `middle`, and after them the status line that says what has been
/// done, drawn with `status_options`. The buttons are drawn as SVG in the scene's own layer, so they zoom and pan with
/// it. Returns the status line's node.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer, or if an element or a node could not be made.
pub(super) fn add_controls(
    scene: &Scene,
    left: f64,
    middle: f64,
    status: &str,
    status_options: NodeOptions,
) -> Result<NodeId, String> {
    let document = document()?;
    let layer = document
        .query_selector(&format!("#{DIAGRAM_ID} > g.svg-dom-graph-content"))
        .map_err(|e| format!("could not look for the scene's layer: {e:?}"))?
        .ok_or("the scene has no content layer")?;
    let mut left = left;
    // The button that flips one bit at random, then the button that puts it back.
    for (label, action, colours) in [
        ("Flip a random bit", "introduce", FLIP_COLOURS),
        ("Remove all errors", "remove", REMOVE_COLOURS),
    ] {
        add_button(
            &document,
            &layer,
            Point::new(left, middle - BUTTON_HEIGHT / 2.0),
            label,
            action,
            colours,
        )?;
        left += BUTTON_WIDTH + CONTROL_GAP;
    }
    let size = scene.measure_markdown_node(STATUS_WIDTH, status).map_err(stringify)?;
    scene
        .add_markdown_node_with(
            Point::new(left, middle - size.height / 2.0),
            STATUS_WIDTH,
            status,
            status_options,
        )
        .map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again for the text it was drawn for, at walk position `position`, with the error as it is now, and
/// says whether that worked. The text box is replaced, and so is the button that was pressed. So the new button of the
/// same `action` is given the keyboard focus, as the text box is given it after a change of text. Without that, a
/// keyboard user would find that focus had gone back to the start of the page.
fn redraw_current(action: &str, position: usize) -> bool {
    let text = DEMO.with_borrow(|slot| slot.as_ref().map_or_else(|| INITIAL_TEXT.to_owned(), |d| d.text.clone()));
    let built = build(&text, position).map(|_| ());
    let worked = built.is_ok();
    report(built, "Could not draw the error");
    if !worked {
        return false;
    }
    let button = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| {
            d.query_selector(&format!("#{DIAGRAM_ID} g[data-action=\"{action}\"]"))
                .ok()
                .flatten()
        })
        .and_then(|button| button.dyn_into::<web_sys::SvgElement>().ok());
    if let Some(button) = button {
        let _ = button.focus();
    }
    true
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Says `message` to assistive technology, in a live region that stays on the page. The scene is drawn again for each
/// change, and a live region that is drawn along with it is new each time, which a screen reader may not announce. So
/// this one is in the window page, outside the scene.
pub(super) fn announce(message: &str) {
    if let Some(region) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(ANNOUNCEMENT_ID))
    {
        region.set_text_content(Some(message));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk position the scene is at.
fn current_position() -> usize {
    DEMO.with_borrow(|slot| slot.as_ref().map_or(0, |d| d.position))
}

/// Flips one bit, chosen at random from the bits of the data and of the CRC, in place of any that was flipped before.
/// Nothing is flipped in an empty text, which has no bytes.
///
/// The walk moves on to the bytes arriving, if it has not got that far, since the error cannot be seen before then.
fn introduce_error() {
    let Some(length) = text_length() else { return };
    FLIPS.with_borrow_mut(|flips| *flips = Flips::random(1, length, js_sys::Math::random));
    finish("introduce", current_position().max(Walk::new(length).arrival()));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts every flipped bit back. The walk stays where it is.
fn remove_error() {
    FLIPS.with_borrow_mut(|flips| *flips = Flips::default());
    finish("remove", current_position());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again at `position` with the bits as they are now, and says what they are. `action` is the button
/// that was pressed.
fn finish(action: &str, position: usize) {
    if redraw_current(action, position) {
        announce(&FLIPS.with_borrow(Flips::describe));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn text_length() -> Option<usize> {
    let length = DEMO.with_borrow(|slot| slot.as_ref().map_or(0, |d| utf8_prefix(&d.text, INPUT_LEN).len()));
    if length == 0 {
        announce("You must first enter some text before a bit can be flipped.");
        return None;
    }
    Some(length)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a button in the scene's own layer `layer`, at `top_left`, that `watch_buttons` will find by its `action`. It is
/// a rectangle with a label, in a group that has the role of a button and can take the keyboard focus.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
fn add_button(
    document: &web_sys::Document,
    layer: &web_sys::Element,
    top_left: Point,
    label: &str,
    action: &str,
    (fill, stroke): (&str, &str),
) -> Result<(), String> {
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
    let group = make(
        "g",
        &[
            ("role", "button".to_owned()),
            ("tabindex", "0".to_owned()),
            ("aria-label", label.to_owned()),
            ("data-action", action.to_owned()),
            ("style", "cursor: pointer;".to_owned()),
        ],
    )?;
    let rect = make(
        "rect",
        &[
            ("x", top_left.x.to_string()),
            ("y", top_left.y.to_string()),
            ("width", BUTTON_WIDTH.to_string()),
            ("height", BUTTON_HEIGHT.to_string()),
            ("rx", "6".to_owned()),
            ("fill", fill.to_owned()),
            ("stroke", stroke.to_owned()),
            ("stroke-width", "1.5".to_owned()),
        ],
    )?;
    let text = make(
        "text",
        &[
            ("x", (top_left.x + BUTTON_WIDTH / 2.0).to_string()),
            ("y", (top_left.y + BUTTON_HEIGHT / 2.0).to_string()),
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
/// Makes the buttons drawn by [`add_button`] work. One listener for clicks and one for the Enter and Space keys are on
/// the stage, which outlives every scene that is drawn in it, and find the button by its `data-action`. So a scene
/// drawn again needs no listeners of its own.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
pub(super) fn watch_buttons(document: &web_sys::Document) -> Result<(), String> {
    let stage = required_element(document, STAGE_ID)?;
    let press = |target: Option<web_sys::EventTarget>| {
        let action = target
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-action]").ok().flatten())
            .and_then(|button| button.get_attribute("data-action"));
        match action.as_deref() {
            Some("introduce") => introduce_error(),
            Some("remove") => remove_error(),
            _ => {},
        }
    };
    let clicked = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| press(event.target()));
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" || event.key() == " " {
            let on_button = event
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|e| e.closest("[data-action]").ok().flatten().is_some());
            if on_button {
                event.prevent_default();
                press(event.target());
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
