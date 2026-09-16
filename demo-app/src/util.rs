//! Small DOM/error helpers shared by more than one demo module.

use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts any displayable error into this crate's own `String` error type.
///
/// Lets a library `Error` and a demo-only DOM failure share one `Result` and one `?`, throughout this crate.
pub(crate) fn stringify<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The current page's `Document`, or `Err` if there is no global `window` or no `document` on it.
///
/// Neither should be possible in a real browser. Returned as a graceful `Err` anyway, not a panic, since `run` already
/// has a clean way to report a startup failure to the browser console.
pub(crate) fn document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .ok_or_else(|| "no global window".to_owned())?
        .document()
        .ok_or_else(|| "no document on window".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document`, or `Err` if `index.html` does not define it.
pub(crate) fn required_element(document: &web_sys::Document, id: &str) -> Result<web_sys::Element, String> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| format!("index.html must define #{id}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document` as an `<input>` element.
///
/// `Err` if `index.html` does not define `#id`, or defines it as something other than `<input>`.
pub(crate) fn required_input(document: &web_sys::Document, id: &str) -> Result<HtmlInputElement, String> {
    required_element(document, id)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("#{id} must be an <input>"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `selector` against `document`, or `Err` if nothing matches.
pub(crate) fn required_query(document: &web_sys::Document, selector: &str) -> Result<web_sys::Element, String> {
    document
        .query_selector(selector)
        .map_err(|e| format!("query_selector({selector:?}) failed: {e:?}"))?
        .ok_or_else(|| format!("nothing in the DOM matches {selector:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads `svg`'s own `viewBox` attribute back as a [`Rect`], to bound dragging within it.
///
/// `svg-dom`'s own `SvgRoot` deliberately does not cache `viewBox` — see that crate's own doc comment on `set_view_box`
/// for why. Every demo here sets `viewBox` once, directly in `index.html`, so reading the attribute straight from the
/// DOM is simpler than caching it a second time in this crate too.
///
/// # Errors
///
/// Returns `Err` if `svg` has no `viewBox` attribute, or its value is not exactly four numbers.
pub(crate) fn view_box_rect(svg: &SvgRoot) -> Result<Rect, String> {
    let value = svg
        .root
        .get_attribute("viewBox")
        .ok_or_else(|| "the <svg> has no viewBox attribute".to_owned())?;

    let mut numbers = value.split_whitespace().map(str::parse::<f64>);
    let (Some(Ok(x)), Some(Ok(y)), Some(Ok(width)), Some(Ok(height)), None) =
        (numbers.next(), numbers.next(), numbers.next(), numbers.next(), numbers.next())
    else {
        return Err(format!("viewBox {value:?} is not exactly four numbers"));
    };

    Ok(Rect {
        origin: Point::new(x, y),
        size: Size::new(width, height),
    })
}
