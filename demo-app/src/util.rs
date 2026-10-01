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
/// Neither should be possible in a real browser. Returned as a graceful `Err` anyway, not a panic. This crate already
/// has clean ways to report a failure — `init_panel`'s own returned `Result`, or `report_panel_error`'s visible banner
/// for one inside a demo's own build function.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sizes `#id`'s own `<svg>` to `size`, at the same 1-user-unit-per-pixel scale every canvas in this crate already
/// uses: `width`/`height` (the on-page pixel footprint) and `viewBox` (the internal coordinate system) are set
/// together, to the same numbers, rather than letting either one imply a different scale than the other.
///
/// Written directly to the DOM via `web_sys`, not through `SvgRoot`/`Scene`: by the time a panel knows a diagram's
/// own real content size — after drawing it, or after [`Scene::measure_named_data_node`](svg_dom_graph::scene::Scene::measure_named_data_node)
/// measures it — `Scene::new` has already taken ownership of the `SvgRoot` that could resize it, and neither
/// `Scene` nor `svg-dom-graph` hands that access back. This is the write-side counterpart to [`view_box_rect`]'s
/// own read-side reasoning: `viewBox` is already something this crate reaches for directly on the DOM, because
/// `SvgRoot` deliberately does not cache it either way.
///
/// Safe to call after a `Scene` is already showing content: `svg-dom-graph`'s own internal toolbar/zoom layout
/// (`visible_area`) reads `viewBox` fresh from the DOM on every layout pass, not from any cache of its own, so a
/// plain attribute write here is picked up immediately — nothing is left stale for a panel with no drag bounds to
/// desync (a panel that *does* bound dragging to its own viewBox, via `DragOptions::bounds`/[`view_box_rect`],
/// would need to recompute those bounds after calling this, since `view_box_rect` itself reads the same attribute
/// fresh each time rather than caching it).
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#id`, or a DOM write fails.
pub(crate) fn resize_svg(document: &web_sys::Document, id: &str, size: Size) -> Result<(), String> {
    let svg = required_element(document, id)?;
    let (width, height) = (size.width, size.height);
    svg.set_attribute("width", &width.to_string())
        .map_err(|e| format!("could not set #{id}'s own width: {e:?}"))?;
    svg.set_attribute("height", &height.to_string())
        .map_err(|e| format!("could not set #{id}'s own height: {e:?}"))?;
    svg.set_attribute("viewBox", &format!("0 0 {width} {height}"))
        .map_err(|e| format!("could not set #{id}'s own viewBox: {e:?}"))
}
