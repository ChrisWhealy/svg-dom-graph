//! Keeping keyboard focus across a redraw. A demo that draws its scene again removes every element in it, and focus
//! goes back to the start of the page. A keyboard user would then have to tab to the button they were using all over
//! again. So the demo notes which button had focus before the redraw, and gives focus to the new copy afterwards.

use crate::util::document;
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A selector for the button in the diagram `diagram_id` that has keyboard focus, if one does: by its action if it is
/// one of the demo's own buttons, and otherwise by its accessible name, as a toolbar button is.
#[must_use]
pub fn focused_selector(diagram_id: &str) -> Option<String> {
    let document = document().ok()?;
    let active = document.active_element()?;
    document.get_element_by_id(diagram_id)?.contains(Some(&active)).then_some(())?;
    if let Some(action) = active.get_attribute("data-action") {
        return Some(format!("#{diagram_id} g[data-action=\"{action}\"]"));
    }
    active
        .get_attribute("aria-label")
        .map(|label| format!("#{diagram_id} g[aria-label=\"{label}\"]"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives keyboard focus to the element `selector` finds, if there is one and it can take focus. A toolbar button that
/// is disabled now is left alone.
pub fn focus(selector: &str) {
    let Ok(document) = document() else { return };
    if let Ok(Some(element)) = document.query_selector(selector) {
        if let Ok(element) = element.dyn_into::<web_sys::SvgElement>() {
            let _ = element.focus();
        }
    }
}
