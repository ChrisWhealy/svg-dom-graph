//! Shared fixture and query helpers for every scenario module in this test group.
//!
//! Every test uses a viewport and `viewBox` of `400 x 300`, 1:1, so client pixels and user-space units coincide, unless
//! a helper's own doc comment says otherwise.

use crate::common::make_svg;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{Scene, ToolbarOptions};

pub(super) fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(400.0, 300.0), Size::new(400.0, 300.0))).map_err(|e| e.to_string())
}

pub(super) fn query(selector: &str) -> Result<Option<web_sys::Element>, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector(selector)
        .map_err(|e| format!("{e:?}"))
}

pub(super) fn required(selector: &str) -> Result<web_sys::Element, String> {
    query(selector)?.ok_or_else(|| format!("nothing matches {selector}"))
}

/// The scene's keyboard focus target: a `<rect>` of its own with the `application` role, never the application's
/// `<svg>`.
pub(super) fn focus_target(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > rect[role=\"application\"]"))
}

pub(super) fn bar(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > [role=\"toolbar\"]"))
}

pub(super) fn content(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > g.svg-dom-graph-content"))
}

/// The `n`th toolbar button: 0 is "+", 1 is "−", 2 is "Reset".
pub(super) fn button(id: &str, n: usize) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > [role=\"toolbar\"] > [role=\"button\"]:nth-child({})", n + 1))
}

pub(super) fn attr(element: &web_sys::Element, name: &str) -> Result<String, String> {
    element.get_attribute(name).ok_or_else(|| format!("missing attribute {name}"))
}

pub(super) fn click(element: &web_sys::Element) -> Result<(), String> {
    let event = web_sys::MouseEvent::new("click").map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

pub(super) fn keydown(element: &web_sys::Element, key: &str) -> Result<(), String> {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// Resolves after the browser's next animation frame. Wheel zoom and panning write the DOM once per frame, so a test
/// that reads the rendered `transform` mid-gesture must wait for it.
pub(super) async fn next_frame() -> Result<(), String> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(&resolve);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Parses a `translate(tx, ty) scale(s)` attribute into `(tx, ty, s)`, so a test can compare numbers with a tolerance
/// rather than exact text — repeated multiplication does not always land on the exact decimal.
pub(super) fn parse_view(transform: &str) -> Result<(f64, f64, f64), String> {
    let numbers: Vec<f64> = transform
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E'))
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    match numbers[..] {
        [tx, ty, scale] => Ok((tx, ty, scale)),
        _ => Err(format!("cannot parse transform {transform:?}")),
    }
}

pub(super) fn pan_surface(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > rect[aria-hidden=\"true\"]"))
}

/// Dispatches a bubbling, cancelable `wheel` event at `element` and reports whether a listener cancelled it — which is
/// what stops the browser zooming the whole page for a ctrl+wheel.
pub(super) fn wheel(
    element: &web_sys::Element,
    x: i32,
    y: i32,
    delta_y: f64,
    ctrl: bool,
    meta: bool,
) -> Result<bool, String> {
    let init = web_sys::WheelEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_client_x(x);
    init.set_client_y(y);
    init.set_delta_y(delta_y);
    init.set_ctrl_key(ctrl);
    init.set_meta_key(meta);
    let event = web_sys::WheelEvent::new_with_event_init_dict("wheel", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(event.default_prevented())
}

/// A bubbling, cancelable `keydown`, optionally with Shift, Ctrl, or Meta held. Reports whether a listener cancelled
/// it.
pub(super) fn key(element: &web_sys::Element, key: &str, shift: bool, ctrl: bool, meta: bool) -> Result<bool, String> {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_shift_key(shift);
    init.set_ctrl_key(ctrl);
    init.set_meta_key(meta);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(event.default_prevented())
}

/// The content layer's `(tx, ty)`, or `(0, 0)` if it has never moved.
pub(super) fn content_translate(id: &str) -> Result<(f64, f64), String> {
    match content(id)?.get_attribute("transform") {
        Some(transform) => parse_view(&transform).map(|(tx, ty, _)| (tx, ty)),
        None => Ok((0.0, 0.0)),
    }
}

/// The content layer's scale as it is drawn, or `1.0` if it has never been written.
pub(super) fn drawn_scale(id: &str) -> Result<f64, String> {
    match content(id)?.get_attribute("transform") {
        Some(transform) => parse_view(&transform).map(|(_, _, scale)| scale),
        None => Ok(1.0),
    }
}

/// Builds an `<svg>` with the given CSS size, `viewBox`, and optional `preserveAspectRatio`, and returns its scene.
pub(super) fn scene_in_svg(
    id: &str,
    css_width: u32,
    css_height: u32,
    view_box: &str,
    preserve_aspect_ratio: Option<&str>,
) -> Result<Scene, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .map_err(|e| format!("{e:?}"))?;
    svg.set_id(id);
    svg.set_attribute("viewBox", view_box).map_err(|e| format!("{e:?}"))?;
    if let Some(value) = preserve_aspect_ratio {
        svg.set_attribute("preserveAspectRatio", value).map_err(|e| format!("{e:?}"))?;
    }
    svg.set_attribute(
        "style",
        &format!("display: block; width: {css_width}px; height: {css_height}px;"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let body = document
        .query_selector("body")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no body")?;
    body.append_child(&svg).map_err(|e| format!("{e:?}"))?;
    Scene::new(svg_dom::SvgRoot::attach(id).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Every attribute of `element` as `name=value`, sorted, so two snapshots can be compared.
pub(super) fn attributes_of(element: &web_sys::Element) -> Vec<String> {
    let names = element.get_attribute_names();
    let mut out: Vec<String> = (0..names.length())
        .filter_map(|i| names.get(i).as_string())
        .map(|name| format!("{name}={}", element.get_attribute(&name).unwrap_or_default()))
        .collect();
    out.sort();
    out
}

/// A node at (100, 100), draggable, in a scene with the toolbar shown.
pub(super) fn draggable_node_scene(id: &str) -> Result<Scene, String> {
    let scene = new_scene(id)?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    Ok(scene)
}
