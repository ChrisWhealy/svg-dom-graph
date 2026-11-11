//! `Scene::add_ellipse_node`: a real `<ellipse>` with a centred label, grown (never shrunk) to hold the label inside the
//! curved outline, its place in the model, and what connectors, focus, dragging and the other node methods do with it.
//!
//! The geometry itself, and the routing rules, are checked without a browser, in `src/geometry/ellipse` and
//! `src/scene/connector`. These tests are about what reaches the page.

use crate::common::{
    check, check_close, dispatch_pointer_event, first_point_of_path, group_translate, last_point_of_path, make_svg,
    nth_group, path_d, the_connector,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    EdgeEnd, EndpointProblem, Error, NodeId,
    scene::{
        ClockPosition, ConnectorOptions, ConnectorType, EdgeAnchors, EllipseOptions, EllipseRadii, NodeValues, Scene,
        Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(700.0, 500.0), Size::new(700.0, 500.0))).map_err(|e| e.to_string())
}

fn e(error: Error) -> String {
    error.to_string()
}

fn minute(m: u8) -> ClockPosition {
    ClockPosition::try_from(m).expect("a minute from 0 to 59")
}

fn on_outline(point: (f64, f64), centre: Point, rx: f64, ry: f64) -> bool {
    let value = ((point.0 - centre.x) / rx).powi(2) + ((point.1 - centre.y) / ry).powi(2);
    (value - 1.0).abs() < 1e-6
}

fn child(group: &web_sys::Element, tag: &str) -> Result<web_sys::Element, String> {
    group
        .query_selector(&format!(":scope > {tag}"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("no {tag} in the group"))
}

fn number(element: &web_sys::Element, name: &str) -> Result<f64, String> {
    element
        .get_attribute(name)
        .ok_or_else(|| format!("no {name}"))?
        .parse::<f64>()
        .map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn group_count(id: &str) -> Result<u32, String> {
    let list = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("a document")?
        .query_selector_all(&format!("#{id} > g.svg-dom-graph-content > g"))
        .map_err(|e| format!("{e:?}"))?;
    Ok(list.length())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn two_ellipses(id: &str) -> Result<(Scene, NodeId, NodeId), String> {
    let scene = new_scene(id)?;
    let a = scene
        .add_ellipse_node(Point::new(100.0, 100.0), EllipseRadii::new(60.0, 30.0), "A")
        .map_err(e)?;
    let b = scene
        .add_ellipse_node(Point::new(400.0, 300.0), EllipseRadii::circle(40.0), "B")
        .map_err(e)?;
    Ok((scene, a, b))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of a path of straight segments, `M x y L x y ...`, as the scene writes one with a corner radius of zero.
fn path_points(d: &str) -> Result<Vec<(f64, f64)>, String> {
    let numbers: Vec<f64> = d
        .split_whitespace()
        .filter(|token| *token != "M" && *token != "L")
        .map(|token| token.parse::<f64>().map_err(|e| format!("{token}: {e}")))
        .collect::<Result<_, _>>()?;
    Ok(numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The element the browser's own hit testing finds at `(x, y)` in `svg`'s user space, with the page scrolled so that the
/// scene is on screen. This is real SVG pointer targeting, as a mouse press would get it. It is not worked out from the
/// scene's own stored geometry.
fn element_at(svg: &web_sys::Element, x: f64, y: f64) -> Result<web_sys::Element, String> {
    svg.scroll_into_view();
    let area = svg.get_bounding_client_rect();
    let (client_x, client_y) = (area.left() + x, area.top() + y);
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or("a document")?
        .element_from_point(client_x as f32, client_y as f32)
        .ok_or_else(|| format!("nothing at client ({client_x}, {client_y}), which is user ({x}, {y})"))
}

/// The client position of a point in `svg`'s user space, with the scene scrolled on screen.
fn client_of(svg: &web_sys::Element, x: f64, y: f64) -> (i32, i32) {
    svg.scroll_into_view();
    let area = svg.get_bounding_client_rect();
    ((area.left() + x).round() as i32, (area.top() + y).round() as i32)
}

fn document() -> web_sys::Document {
    web_sys::window().and_then(|window| window.document()).expect("a document")
}

/// The `transform` of the scene's content layer, which is its zoom and pan.
fn content_transform(id: &str) -> Result<String, String> {
    document()
        .query_selector(&format!("#{id} > g.svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no content layer")?
        .get_attribute("transform")
        .ok_or_else(|| "the content layer has no transform yet".to_owned())
        .or_else(|_| Ok("translate(0, 0) scale(1)".to_owned()))
}

mod unit_tests;
