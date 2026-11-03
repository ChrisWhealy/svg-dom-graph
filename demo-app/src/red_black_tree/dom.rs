//! Small helpers that make and write SVG elements, and a journal that puts writes back.

use wasm_bindgen::JsCast;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

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
        .create_element_ns(Some(SVG_NS), tag)
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was there before a write, to put back.
enum Undo {
    Attr(web_sys::Element, String, Option<String>),
    Text(web_sys::Element, Option<String>),
    Input(web_sys::HtmlInputElement, String),
}

/// Writes that can be put back, in the reverse order, if a later one fails.
#[derive(Default)]
pub(super) struct Journal {
    undo: Vec<Undo>,
}

impl Journal {
    /// Sets an attribute, and remembers what it was. A write that fails changes nothing, and is not remembered.
    pub(super) fn attr(&mut self, element: &web_sys::Element, name: &str, value: &str) -> Result<(), String> {
        let old = element.get_attribute(name);
        set(element, name, value)?;
        self.undo.push(Undo::Attr(element.clone(), name.to_owned(), old));
        Ok(())
    }

    /// Sets the text of an element, and remembers what it was.
    pub(super) fn text(&mut self, element: &web_sys::Element, text: &str) {
        self.undo.push(Undo::Text(element.clone(), element.text_content()));
        element.set_text_content(Some(text));
    }

    /// Sets the value of a range, and remembers what it was.
    pub(super) fn input(&mut self, input: &web_sys::HtmlInputElement, value: &str) {
        self.undo.push(Undo::Input(input.clone(), input.value()));
        input.set_value(value);
    }

    /// Puts everything back as far as it can be, and says whether all of it went back. A put-back that fails is left,
    /// and the caller then treats what is on the page as unknown, which the next action repairs.
    pub(super) fn restore(&mut self) -> bool {
        let mut whole = true;
        while let Some(undo) = self.undo.pop() {
            match undo {
                Undo::Attr(element, name, Some(old)) => whole &= element.set_attribute(&name, &old).is_ok(),
                Undo::Attr(element, name, None) => whole &= element.remove_attribute(&name).is_ok(),
                Undo::Text(element, old) => element.set_text_content(old.as_deref()),
                Undo::Input(input, old) => input.set_value(&old),
            }
        }
        whole
    }
}
