//! The window's one `<svg>`: the controls above, the readout below, and between them the scene, which pans and zooms.
//!
//! The controls and the readout are the browser's own HTML, in a `<foreignObject>` each, so that every button and every list is in the
//! scene's `<svg>` and none is on the page around it. The scene itself is a nested `<svg>`, which the library's `Scene` is built on,
//! and which has its own toolbar and its own view, so that the controls and the readout stay where they are when it pans.

use demo_common::util::required_element;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XHTML: &str = "http://www.w3.org/1999/xhtml";

/// The height of the controls above the scene and of the readout below it.
const CONTROLS_HEIGHT: f64 = 140.0;
const READOUT_HEIGHT: f64 = 340.0;
/// The room, in pixels, that the scene keeps clear at each side of the graph.
const EDGE_ROOM: f64 = 16.0;
/// A small graph is not drawn larger than this, so that a handful of nodes is not made enormous.
const MOST_ENLARGED: f64 = 2.0;
/// A graph that is made to be read is not drawn smaller than this, however tall it is, so that its text can be read. A taller one is
/// begun at the top, and the person pans and zooms to the rest.
const LEAST_READABLE: f64 = 0.7;

/// The controls: the size of the problem, and what to do with it. Every id is one that the page's script and its test find them by.
const CONTROLS_HTML: &str = include_str!("controls.html");

/// The readout: the verdict, what the selected node computes, and what the build and the steps cost.
const READOUT_HTML: &str = include_str!("readout.html");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `<svg>` of the window, and the room that is left for the scene.
#[derive(Clone)]
pub(super) struct Shell {
    pub host: web_sys::Element,
    /// The scene's place in the host: its left, its top, its width and its height.
    pub area: (f64, f64, f64, f64),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn make(document: &web_sys::Document, tag: &str, attributes: &[(&str, &str)]) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some(SVG_NS), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        element
            .set_attribute(name, value)
            .map_err(|e| format!("could not set {name}: {e:?}"))?;
    }
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn add(parent: &web_sys::Element, child: &web_sys::Element) -> Result<(), String> {
    parent
        .append_child(child)
        .map(|_| ())
        .map_err(|e| format!("could not add an element: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `<foreignObject>` of the given place, holding a `<div>` of HTML with `html` in it.
fn foreign(
    document: &web_sys::Document,
    place: (f64, f64, f64, f64),
    class: &str,
    html: &str,
) -> Result<web_sys::Element, String> {
    let object = make(
        document,
        "foreignObject",
        &[
            ("x", &place.0.to_string()),
            ("y", &place.1.to_string()),
            ("width", &place.2.to_string()),
            ("height", &place.3.to_string()),
        ],
    )?;
    let div = document
        .create_element_ns(Some(XHTML), "div")
        .map_err(|e| format!("could not make a <div>: {e:?}"))?;
    div.set_class_name(&format!("spmv-fo {class}"));
    div.set_inner_html(html);
    add(&object, &div)?;
    Ok(object)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Fills `#root_id` with the window's `<svg>`: a backdrop for the scene, the controls and the readout. The size is the window's.
///
/// # Errors
///
/// Returns `Err` if the root is missing or an element could not be made.
pub(super) fn build_host(document: &web_sys::Document, root_id: &str) -> Result<Shell, String> {
    let root = required_element(document, root_id)?;
    root.set_text_content(None);
    let window = web_sys::window().ok_or("no window")?;
    let size = |value: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>, least: f64| {
        value.ok().and_then(|v| v.as_f64()).unwrap_or(least).max(least)
    };
    let (width, height) = (size(window.inner_width(), 900.0), size(window.inner_height(), 640.0));
    let host = make(
        document,
        "svg",
        &[
            ("id", "spmv-host"),
            ("class", "spmv-host"),
            ("width", &width.to_string()),
            ("height", &height.to_string()),
            ("viewBox", &format!("0 0 {width} {height}")),
        ],
    )?;
    let area = (0.0, CONTROLS_HEIGHT, width, height - CONTROLS_HEIGHT - READOUT_HEIGHT);
    add(
        &host,
        &foreign(
            document,
            (0.0, 0.0, width, CONTROLS_HEIGHT),
            "spmv-controls-pane",
            CONTROLS_HTML,
        )?,
    )?;
    add(
        &host,
        &foreign(
            document,
            (0.0, height - READOUT_HEIGHT, width, READOUT_HEIGHT),
            "spmv-readout-pane",
            READOUT_HTML,
        )?,
    )?;
    add(&root, &host)?;
    Ok(Shell { host, area })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's own `<svg>`, and what the window needs to know of it.
pub(super) struct SceneSvg {
    pub element: web_sys::Element,
    /// Where the graph is to begin in the scene's space, so that it is centred in the room.
    pub origin: (f64, f64),
    /// The size of the scene's space, which is what zooming about its centre needs.
    pub view_size: (f64, f64),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds the scene's own `<svg>` in the room that is left. Its view box has the shape of that room, and it gives where the graph is to
/// begin in it: `extent` centred in the room, and a tall one at the top of it.
///
/// # Errors
///
/// Returns `Err` if the element could not be made.
pub(super) fn add_scene_svg(
    document: &web_sys::Document,
    shell: &Shell,
    id: &str,
    extent: (f64, f64),
    readable: bool,
) -> Result<SceneSvg, String> {
    let (x, y, w, h) = shell.area;
    let fits = ((w - 2.0 * EDGE_ROOM) / extent.0).min((h - 2.0 * EDGE_ROOM) / extent.1);
    let least = if readable { LEAST_READABLE } else { f64::MIN_POSITIVE };
    let scale = fits.clamp(least, MOST_ENLARGED);
    let left = ((w - extent.0 * scale) / 2.0).max(EDGE_ROOM);
    let top = ((h - extent.1 * scale) / 2.0).max(EDGE_ROOM);
    // The view box begins at the origin, and the graph is moved by `origin` instead, so that a view of 1, 0, 0 is the whole picture.
    let view_box = format!("0 0 {} {}", w / scale, h / scale);
    let svg = make(
        document,
        "svg",
        &[
            ("id", id),
            ("class", "spmv-scene-svg"),
            ("x", &x.to_string()),
            ("y", &y.to_string()),
            ("width", &w.to_string()),
            ("height", &h.to_string()),
            ("viewBox", &view_box),
        ],
    )?;
    // The backdrop is the scene's own, so that a press on empty space reaches the scene, which pans when the background is dragged.
    add(
        &svg,
        &make(
            document,
            "rect",
            &[
                ("class", "spmv-backdrop"),
                ("x", "0"),
                ("y", "0"),
                ("width", "100%"),
                ("height", "100%"),
            ],
        )?,
    )?;
    add(&shell.host, &svg)?;
    Ok(SceneSvg {
        element: svg,
        origin: (left / scale, top / scale),
        view_size: (w / scale, h / scale),
    })
}
