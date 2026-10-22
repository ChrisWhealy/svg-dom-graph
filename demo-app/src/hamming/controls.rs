//! The buttons in the scene that choose the data bits and introduce an error, and what they do.

use super::{
    ANNOUNCEMENT_ID, DIAGRAM_ID, STAGE_ID, STATE,
    code::{CODEWORD_LEN, Transmission},
    redraw,
    walk::ARRIVAL,
};
use demo_common::{
    svg_buttons::{self, Button},
    util::{document, stringify},
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{NodeOptions, Scene},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the buttons, and the gap between one and the next.
const BIT_WIDTH: f64 = 96.0;
const ACTION_WIDTH: f64 = 180.0;
const BUTTON_HEIGHT: f64 = 36.0;
const STATUS_WIDTH: f64 = 300.0;
const CONTROL_GAP: f64 = 16.0;

/// The fill of the status line: white, so that it is told apart from the light blue notes and from the buttons.
const STATUS_FILL: &str = "#ffffff";

/// The fill and the outline of the buttons. A data bit that is 1 is a stronger blue than one that is 0, and flipping a
/// bit is a warning amber, and putting it back is a calm green.
const BIT_ON: (&str, &str) = ("#bcd9f7", "#2b5d9b");
const BIT_OFF: (&str, &str) = ("#f1f1f1", "#767676");
const FLIP_COLOURS: (&str, &str) = ("#fde3b0", "#a8650a");
const REMOVE_COLOURS: (&str, &str) = ("#d4edd6", "#2f7a3a");

/// What [`add_controls`] drew.
pub(super) struct Controls {
    /// The x of the right edge of the last of them, and the y of the bottom of the tallest.
    pub(super) right: f64,
    pub(super) bottom: f64,
    pub(super) status: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds the controls in a row starting at `left`, with their tops at `top`: a button for each data bit, the button that
/// flips a bit at random, the button that puts it back, and the status line that says what has been done. The buttons
/// are drawn as SVG in the scene's own layer, so they zoom and pan with it.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer, or if an element or a node could not be made.
pub(super) fn add_controls(
    scene: &Scene,
    left: f64,
    top: f64,
    status: &str,
    t: &Transmission,
) -> Result<Controls, String> {
    let document = document()?;
    let layer = svg_buttons::layer(&document, DIAGRAM_ID)?;
    let mut x = left;
    let mut place = |width: f64, label: String, action: String, colours: (&str, &str), pressed: Option<bool>| {
        let result = svg_buttons::add_button(
            &document,
            &layer,
            Rect {
                origin: Point::new(x, top),
                size: Size::new(width, BUTTON_HEIGHT),
            },
            &Button {
                label: &label,
                action: &action,
                colours,
                pressed,
            },
        );
        x += width + CONTROL_GAP;
        result
    };
    for (index, bit) in t.data.iter().enumerate() {
        let on = *bit == 1;
        place(
            BIT_WIDTH,
            format!("d{} = {bit}", index + 1),
            format!("toggle-{index}"),
            if on { BIT_ON } else { BIT_OFF },
            Some(on),
        )?;
    }
    place(
        ACTION_WIDTH,
        "Flip a random bit".to_owned(),
        "introduce".to_owned(),
        FLIP_COLOURS,
        None,
    )?;
    place(
        ACTION_WIDTH,
        "Remove the error".to_owned(),
        "remove".to_owned(),
        REMOVE_COLOURS,
        None,
    )?;

    let size = scene.measure_markdown_node(STATUS_WIDTH, status).map_err(stringify)?;
    let middle = top + BUTTON_HEIGHT / 2.0;
    let status_node = scene
        .add_markdown_node_with(
            Point::new(x, middle - size.height / 2.0),
            STATUS_WIDTH,
            status,
            NodeOptions::default().with_fill(Some(STATUS_FILL)),
        )
        .map_err(stringify)?;
    Ok(Controls {
        right: x + STATUS_WIDTH,
        bottom: (top + BUTTON_HEIGHT).max(middle + size.height / 2.0),
        status: status_node,
    })
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the status line says about the error.
pub(super) fn describe(error: Option<usize>) -> String {
    match error {
        None => "No error has been introduced.".to_owned(),
        Some(position) => format!("The bit at position {position} is flipped in transmission."),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the live region says after the data bits change.
fn describe_data(data: &[u8; 4]) -> String {
    format!("The data bits are now {}.", data.map(|b| b.to_string()).join(" "))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk position the scene is at.
fn current_position() -> usize {
    super::DEMO.with_borrow(|slot| slot.as_ref().map_or(0, |d| d.position))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flips data bit `index`, 0 for d1. The walk stays where it is.
fn toggle_bit(index: usize) {
    let data = STATE.with_borrow_mut(|state| {
        state.data[index] ^= 1;
        state.data
    });
    if redraw(&format!("toggle-{index}"), current_position()) {
        announce(&describe_data(&data));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flips one bit of the codeword, chosen at random, in place of the one that was flipped before. The walk moves on to
/// the received codeword, if it has not got that far, since the error cannot be seen before then.
fn introduce_error() {
    // A number from 0 up to but not including 1, scaled to the 7 positions. The result is at most 6, even for the
    // largest number the source could give.
    let position = 1 + ((js_sys::Math::random() * CODEWORD_LEN as f64) as usize).min(CODEWORD_LEN - 1);
    STATE.with_borrow_mut(|state| state.error = Some(position));
    if redraw("introduce", current_position().max(ARRIVAL)) {
        announce(&describe(Some(position)));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the flipped bit back. The walk stays where it is.
fn remove_error() {
    STATE.with_borrow_mut(|state| state.error = None);
    if redraw("remove", current_position()) {
        announce(&describe(None));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons work: see [`svg_buttons::watch`].
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
pub(super) fn watch_buttons(document: &web_sys::Document) -> Result<(), String> {
    svg_buttons::watch(document, STAGE_ID, |action| match action {
        "introduce" => introduce_error(),
        "remove" => remove_error(),
        other => {
            if let Some(index) = other.strip_prefix("toggle-").and_then(|n| n.parse::<usize>().ok()) {
                if index < 4 {
                    toggle_bit(index);
                }
            }
        },
    })
}
