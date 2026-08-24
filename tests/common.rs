#![allow(dead_code)]
// Each test binary uses a different subset of these helpers.

// Shared fixture and assertion helpers for the browser integration tests.
//
// Tests are isolated by using a unique element id per test — there are no teardown hooks, but the elements are
// harmless since the browser page is discarded after the test run.
use svg_dom::{SvgRoot, root::utils::Size};
use wasm_bindgen::JsCast;

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn body() -> web_sys::Element {
    // query_selector avoids the HtmlElement feature requirement.
    document().query_selector("body").unwrap().unwrap()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates a fresh container `<div>`, then an `SvgRoot` inside it with its own element id set to `id`.
///
/// `viewport` is the `<svg>`'s own rendered width/height, in CSS pixels.
/// `view_box` is its `viewBox`.
/// The two need not match — a mismatch is exactly what lets a test prove pointer-coordinate conversion under
/// scaling.
pub fn make_svg(id: &str, viewport: Size, view_box: Size) -> SvgRoot {
    let container_id = format!("{id}-container");
    let el = document().create_element("div").unwrap();
    el.set_id(&container_id);
    body().append_child(&el).unwrap();

    let svg = SvgRoot::create_in(&container_id, viewport).unwrap();
    svg.root.set_id(id);
    svg.set_view_box(0.0, 0.0, view_box.width, view_box.height).unwrap();
    svg
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dispatches a synthetic pointer event with `client_x`/`client_y`/`pointer_id` set, directly to `element`.
///
/// Dispatched straight at `element` (not a descendant), so this does not rely on event bubbling.
pub fn dispatch_pointer_event(
    element: &web_sys::Element,
    event_type: &str,
    client_x: i32,
    client_y: i32,
    pointer_id: i32,
) -> Result<(), String> {
    let init = web_sys::PointerEventInit::new();
    init.set_client_x(client_x);
    init.set_client_y(client_y);
    init.set_pointer_id(pointer_id);

    let event = web_sys::PointerEvent::new_with_event_init_dict(event_type, &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns the `n`th `<g>` child of `container` (0-indexed), as an `Element`.
pub fn nth_group(container_id: &str, n: u32) -> Result<web_sys::Element, String> {
    let selector = format!("#{container_id} > g");
    let groups = document().query_selector_all(&selector).map_err(|e| format!("{e:?}"))?;
    let group = groups
        .get(n)
        .ok_or_else(|| format!("expected at least {} <g> children of #{container_id}, found fewer", n + 1))?;
    group
        .dyn_into::<web_sys::Element>()
        .map_err(|_| "group is not an Element".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns the sole `<line>` child of `container`, as an `Element`.
pub fn the_connector(container_id: &str) -> Result<web_sys::Element, String> {
    let selector = format!("#{container_id} > line");
    document()
        .query_selector(&selector)
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("no <line> connector found under #{container_id}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads `attr` off `element` and parses it as `f64`.
pub fn attr_f64(element: &web_sys::Element, attr: &str) -> Result<f64, String> {
    let value = element
        .get_attribute(attr)
        .ok_or_else(|| format!("missing attribute {attr:?}"))?;
    value
        .parse::<f64>()
        .map_err(|e| format!("attribute {attr:?} = {value:?} did not parse as f64: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns `Err` unless `got` is within `0.01` of `expected`.
///
/// A tolerance, not exact equality, since coordinate conversion through an inverted screen CTM involves real
/// floating-point division.
pub fn check_close(got: f64, expected: f64) -> Result<(), String> {
    let diff = (got - expected).abs();
    if diff > 0.01 {
        Err(format!("expected approximately {expected}, got {got} (diff {diff})"))
    } else {
        Ok(())
    }
}
