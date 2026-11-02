//! Controls that a caller puts in a scene beside the library's own nodes, and the listeners that make them work.
//!
//! - [`svg_buttons`] — buttons drawn in SVG in the scene's own layer, so they zoom and pan with it.
//! - [`sliders`] — range sliders in a scene: the browser's own `<input type="range">` in a `<foreignObject>`, so that
//!   the browser does the dragging, the keys and the accessibility.
//!
//! Each finds the `<svg>` of a scene by its id in the page, as a [`crate::scene::Scene`] is made in one.

pub mod sliders;
pub mod svg_buttons;

/// The element with the id `id` in `document`, or an error that says which id the page lacks.
fn required_element(document: &web_sys::Document, id: &str) -> Result<web_sys::Element, String> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| format!("the page must define #{id}"))
}
