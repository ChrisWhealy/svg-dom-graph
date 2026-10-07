//! Shared DOM query helpers for every scenario module in this test group.

use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every `<text>` child of `group`, in document order.
pub(super) fn text_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "text")
}

/// Every `<rect>` child of `group`, in document order. For a data node, index 0 is always the outer box, and any
/// further entries are the per-value inner cells (see `draw_content_box`'s own doc comment).
pub(super) fn rect_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "rect")
}

pub(super) fn elements_matching(group: &web_sys::Element, selector: &str) -> Result<Vec<web_sys::Element>, String> {
    let nodes = group.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::with_capacity(nodes.length() as usize);
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a length longer than it could actually return")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| format!("{selector} is not an Element"))?;
        out.push(el);
    }
    Ok(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `<title>` child text, or `None` if `element` has no direct `<title>` child.
pub(super) fn title_of(element: &web_sys::Element) -> Result<Option<String>, String> {
    Ok(element
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .map(|title| title.text_content().unwrap_or_default()))
}
