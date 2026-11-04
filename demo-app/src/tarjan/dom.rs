//! Small helpers that make and write SVG elements, which the board and the page share.

use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets the attribute `name` of `element` to `value`.
pub(super) fn set(element: &web_sys::Element, name: &str, value: &str) -> Result<(), String> {
    element
        .set_attribute(name, value)
        .map_err(|e| format!("could not set {name}: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes an SVG element called `tag` with `attributes`.
pub(super) fn make(
    document: &web_sys::Document,
    tag: &str,
    attributes: &[(&str, &str)],
) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        set(&element, name, value)?;
    }
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds `child` to `parent`.
pub(super) fn add(parent: &web_sys::Element, child: &web_sys::Element) -> Result<(), String> {
    parent
        .append_child(child)
        .map(|_| ())
        .map_err(|e| format!("could not add an element: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives the keyboard focus to `element`, if it is one that can have it.
pub(super) fn focus(element: &web_sys::Element) {
    if let Some(element) = element.dyn_ref::<web_sys::SvgElement>() {
        let _ = element.focus();
    }
}
