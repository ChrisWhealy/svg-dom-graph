//! The buttons in the scene that choose the array to sort, and what they do.

use super::{Algorithm, array, go, with_runtime};
use crate::svg_buttons::{self, Button};
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the buttons, and the gap between one and the next.
const BUTTON_WIDTH: f64 = 180.0;
const BUTTON_HEIGHT: f64 = 36.0;
const BUTTON_GAP: f64 = 16.0;

/// The buttons, as `(label, action, fill and outline)`. An array that is already sorted, or in reverse, is where many
/// algorithms are at their best or their worst.
const BUTTONS: [(&str, &str, (&str, &str)); 3] = [
    ("Random array", "random", ("#bcd9f7", "#2b5d9b")),
    ("Already sorted", "sorted", ("#fde3b0", "#a8650a")),
    ("Reverse sorted", "reversed", ("#fde3b0", "#a8650a")),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds the buttons in a row, centred on `axis` and below the stepping toolbar, which is fixed to the top edge. They
/// are drawn as SVG in the scene's own layer, so they zoom and pan with it. Returns the y of the bottom of the row.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer, or if an element could not be made.
pub(super) fn add_buttons(document: &web_sys::Document, diagram_id: &str, axis: f64) -> Result<f64, String> {
    let layer = svg_buttons::layer(document, diagram_id)?;
    let count = BUTTONS.len() as f64;
    let left = axis - (count * BUTTON_WIDTH + (count - 1.0) * BUTTON_GAP) / 2.0;
    for (index, (label, action, colours)) in BUTTONS.into_iter().enumerate() {
        svg_buttons::add_button(
            document,
            &layer,
            Rect {
                origin: Point::new(left.max(0.0) + index as f64 * (BUTTON_WIDTH + BUTTON_GAP), super::TOP),
                size: Size::new(BUTTON_WIDTH, BUTTON_HEIGHT),
            },
            &Button {
                label,
                action,
                colours,
                pressed: None,
            },
        )?;
    }
    Ok(super::TOP + BUTTON_HEIGHT)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Says `message` to assistive technology, in the live region that `announcement_id` names, which stays on the page.
/// The scene is drawn again for each step, and a live region that is drawn along with it is new each time, which a
/// screen reader may not announce. So this one is in the panel, outside the scene.
pub(super) fn announce(announcement_id: &str, message: &str) {
    if let Some(region) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(announcement_id))
    {
        region.set_text_content(Some(message));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The array as the live region says it, for after a new one is chosen.
pub(super) fn describe(values: &[u8]) -> String {
    format!(
        "A new array: {}.",
        values.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button of `algorithm`'s demo was pressed: sorts a new array, from the start.
fn choose(algorithm: &'static Algorithm, action: &str) {
    let mut values = array::sample(js_sys::Math::random);
    match action {
        "random" => {},
        "sorted" => values.sort_unstable(),
        "reversed" => values.sort_unstable_by(|a, b| b.cmp(a)),
        _ => return,
    }
    with_runtime(algorithm, |runtime| runtime.array = values);
    go(algorithm, 0, true);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons of `algorithm`'s demo work: see [`svg_buttons::watch`].
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
pub(super) fn watch_buttons(document: &web_sys::Document, algorithm: &'static Algorithm) -> Result<(), String> {
    svg_buttons::watch(document, &format!("{}-diagram-stage", algorithm.prefix), move |action| {
        choose(algorithm, action);
    })
}
