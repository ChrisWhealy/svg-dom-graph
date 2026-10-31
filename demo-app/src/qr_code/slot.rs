//! One attribute or text of one element, remembered so that writing the value it already holds does nothing.

use std::cell::RefCell;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a [`Slot`] writes.
#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    Attribute(&'static str),
    Text,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An attribute or the text of an element, and the last value written to it.
pub(super) struct Slot {
    element: web_sys::Element,
    target: Target,
    last: RefCell<Option<String>>,
}

impl Slot {
    pub(super) fn new(element: &web_sys::Element, target: Target) -> Self {
        Self {
            element: element.clone(),
            target,
            last: RefCell::new(None),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes `value` unless it is what the slot already holds, and says whether it wrote.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the DOM refuses the write. The slot keeps its old value, so that the next call tries again.
    pub(super) fn set(&self, value: &str) -> Result<bool, String> {
        if self.last.borrow().as_deref() == Some(value) {
            return Ok(false);
        }
        match self.target {
            Target::Attribute(name) => self
                .element
                .set_attribute(name, value)
                .map_err(|e| format!("could not set {name}: {e:?}"))?,
            Target::Text => self.element.set_text_content(Some(value)),
        }
        *self.last.borrow_mut() = Some(value.to_owned());
        Ok(true)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An SVG element called `tag` with `attributes`.
pub(super) fn make(
    document: &web_sys::Document,
    tag: &str,
    attributes: &[(&str, &str)],
) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        element
            .set_attribute(name, value)
            .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
    }
    Ok(element)
}
