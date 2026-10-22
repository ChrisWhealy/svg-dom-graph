//! `Scene::set_edge_label`: a label drawn beside its connector, clear of the line, the arrowhead and the nodes, placed
//! again whenever the connector is, kept clear of the labels of other edges, and removed with `None`.
//!
//! The placement rules themselves are checked without a browser, in `src/geometry/label`. These tests are about what
//! reaches the page: the drawn label, where it ends up, and that it follows its connector.

use crate::common::{FailingWrites, check, group_translate, make_svg, mutations_during};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    EdgeId, Error, NodeId,
    scene::{ConnectorOptions, ConnectorType, Scene},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

const ARROW_REACH: f64 = 14.0;

/// A scene `id` with room for the graphs below.
fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(700.0, 500.0), Size::new(700.0, 500.0))).map_err(|e| e.to_string())
}

fn node(scene: &Scene, name: &str, x: f64, y: f64) -> Result<NodeId, String> {
    scene
        .add_node(Point::new(x, y), Size::new(60.0, 40.0), name)
        .map_err(|e| e.to_string())
}

fn document() -> web_sys::Document {
    web_sys::window().and_then(|w| w.document()).expect("a document")
}

fn content_layer(id: &str) -> Result<web_sys::Element, String> {
    document()
        .query_selector(&format!("#{id} > g.svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("no content layer under #{id}"))
}

/// The group of the label on the edge with index `edge`, found by the attribute that marks it.
fn label_group(id: &str, edge: usize) -> Result<web_sys::Element, String> {
    document()
        .query_selector(&format!(
            "#{id} > g.svg-dom-graph-content > g[data-svg-dom-graph-edge-label=\"{edge}\"]"
        ))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("no label on edge {edge} under #{id}"))
}

/// The box of the label on edge `edge`, in the scene's own units: where its group is, and the size of its background.
fn label_rect(id: &str, edge: usize) -> Result<Rect, String> {
    let group = label_group(id, edge)?;
    let (x, y) = group_translate(&group)?;
    let background = group
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("a label has a background")?;
    let side = |name: &str| -> Result<f64, String> {
        background
            .get_attribute(name)
            .ok_or_else(|| format!("the background has no {name}"))?
            .parse::<f64>()
            .map_err(|e| format!("{e}"))
    };
    Ok(Rect {
        origin: Point::new(x, y),
        size: Size::new(side("width")?, side("height")?),
    })
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.width
        && a.origin.x + a.size.width > b.origin.x
        && a.origin.y < b.origin.y + b.size.height
        && a.origin.y + a.size.height > b.origin.y
}

/// Whether any of `points`, which are sampled along a connector, is inside `rect`.
fn line_hits(points: &[(f64, f64)], rect: Rect) -> bool {
    points.iter().any(|&(x, y)| {
        x > rect.origin.x
            && x < rect.origin.x + rect.size.width
            && y > rect.origin.y
            && y < rect.origin.y + rect.size.height
    })
}

fn call(target: &wasm_bindgen::JsValue, method: &str, argument: Option<f64>) -> Result<wasm_bindgen::JsValue, String> {
    let function: js_sys::Function = js_sys::Reflect::get(target, &method.into())
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| format!("{method} is not a function"))?;
    match argument {
        Some(value) => function.call1(target, &value.into()),
        None => function.call0(target),
    }
    .map_err(|e| format!("{e:?}"))
}

/// Points along the actual connector of edge `edge`, from its first to its last, so that an elbow's rounded corners and
/// every one of its runs are covered and not only the chord between its ends.
fn connector_points(id: &str, edge: u32) -> Result<Vec<(f64, f64)>, String> {
    let path = document()
        .query_selector_all(&format!("#{id} > g.svg-dom-graph-content > path"))
        .map_err(|e| format!("{e:?}"))?
        .get(edge)
        .ok_or("no such connector")?;
    let total = call(&path, "getTotalLength", None)?.as_f64().ok_or("a length")?;
    (0..=200)
        .map(|step| {
            let point = call(&path, "getPointAtLength", Some(total * f64::from(step) / 200.0))?;
            let read = |name: &str| {
                js_sys::Reflect::get(&point, &name.into())
                    .map_err(|e| format!("{e:?}"))
                    .and_then(|v| v.as_f64().ok_or_else(|| format!("{name} is not a number")))
            };
            Ok((read("x")?, read("y")?))
        })
        .collect()
}

fn centre(rect: Rect) -> (f64, f64) {
    (rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

/// The `translate(...)` the label's group carries, as text, so that two readings can be compared for equality.
fn transform_of(id: &str, edge: usize) -> Result<Option<String>, String> {
    Ok(label_group(id, edge)?.get_attribute("transform"))
}

fn e(error: Error) -> String {
    error.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A label is a group in the content layer, with its text, a background behind it, a name for assistive technology,
/// and the attribute that marks it. It takes no pointer events.
#[wasm_bindgen_test]
fn a_label_is_drawn_in_the_content_layer_with_its_text_and_its_name() -> Result<(), String> {
    let scene = new_scene("label-drawn")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("42")).map_err(e)?;

    let group = label_group("label-drawn", 0)?;
    check(
        group
            .query_selector("text")
            .map_err(|e| format!("{e:?}"))?
            .is_some_and(|t| t.text_content().as_deref() == Some("42")),
        "the text",
    )?;
    check(
        group.query_selector("rect").map_err(|e| format!("{e:?}"))?.is_some(),
        "a background",
    )?;
    check(group.get_attribute("role").as_deref() == Some("group"), "role=group")?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("Edge from A to B: 42"),
        &format!("the name was {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        group.get_attribute("pointer-events").as_deref() == Some("none"),
        "a label takes no pointer events",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The label is beside its connector and clear of the line, of the arrowhead, and of both nodes.
#[wasm_bindgen_test]
fn a_label_sits_clear_of_its_line_its_arrowhead_and_both_nodes() -> Result<(), String> {
    let scene = new_scene("label-clear")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 160.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("8")).map_err(e)?;

    let label = label_rect("label-clear", 0)?;
    let line = connector_points("label-clear", 0)?;
    let (start, end) = (line[0], line[line.len() - 1]);
    check(
        !line_hits(&line, label),
        &format!("the line passes through the label {label:?}"),
    )?;
    let arrow = Rect {
        origin: Point::new(end.0 - ARROW_REACH, end.1 - ARROW_REACH),
        size: Size::new(2.0 * ARROW_REACH, 2.0 * ARROW_REACH),
    };
    check(!overlap(label, arrow), "the label is on the arrowhead")?;
    for node in [a, b] {
        check(!overlap(label, scene.node_rect(node).map_err(e)?), "the label is on a node")?;
    }
    // Beside it, and not stranded: the label's centre is within a label's width of the line's middle.
    let middle = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
    let (cx, cy) = centre(label);
    check(
        ((cx - middle.0).powi(2) + (cy - middle.1).powi(2)).sqrt() < 200.0,
        "the label is near its connector",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moving a node moves the connector and the label together, and the label is still clear of the new line.
#[wasm_bindgen_test]
fn a_label_follows_its_connector_when_a_node_is_moved() -> Result<(), String> {
    let scene = new_scene("label-follows")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("5")).map_err(e)?;

    for to in [Point::new(320.0, 200.0), Point::new(420.0, 300.0), Point::new(320.0, 60.0)] {
        scene.move_node(b, to).map_err(e)?;
        let label = label_rect("label-follows", 0)?;
        let line = connector_points("label-follows", 0)?;
        let (start, end) = (line[0], line[line.len() - 1]);
        check(
            !line_hits(&line, label),
            &format!("the line passes through the label at {to:?}"),
        )?;
        check(!overlap(label, scene.node_rect(a).map_err(e)?), "the label is on A")?;
        check(!overlap(label, scene.node_rect(b).map_err(e)?), "the label is on B")?;
        let middle = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
        let (cx, cy) = centre(label);
        check(
            ((cx - middle.0).powi(2) + (cy - middle.1).powi(2)).sqrt() < 200.0,
            "the label stayed near its connector",
        )?;
    }
    Ok(())
}

/// Changing the connector type puts the label beside the new route.
#[wasm_bindgen_test]
fn changing_the_connector_type_places_the_label_again() -> Result<(), String> {
    let scene = new_scene("label-type")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 220.0)?);
    let edge = scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.set_edge_label(edge, Some("3")).map_err(e)?;
    let straight = label_rect("label-type", 0)?;

    scene
        .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 8.0 })
        .map_err(e)?;
    let elbow = label_rect("label-type", 0)?;
    let line = connector_points("label-type", 0)?;
    check(elbow != straight, "the label moved with the route")?;
    // An elbow has up to three straight runs and rounded corners. The label is clear of all of the line, and not only
    // of the chord between its ends.
    check(
        !line_hits(&line, elbow),
        &format!("the elbow passes through the label {elbow:?}"),
    )?;
    check(!overlap(elbow, scene.node_rect(a).map_err(e)?), "the label is on A")?;
    check(!overlap(elbow, scene.node_rect(b).map_err(e)?), "the label is on B")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Setting the label an edge already has writes nothing: no attribute, no text, no element.
#[wasm_bindgen_test]
fn setting_the_same_label_again_writes_nothing() -> Result<(), String> {
    let scene = new_scene("label-same")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("7")).map_err(e)?;

    let writes = mutations_during(&content_layer("label-same")?, || {
        scene.set_edge_label(edge, Some("7")).map_err(e)
    })?;
    check(writes == 0, &format!("{writes} DOM writes for a label that did not change"))
}

/// A label's text can be changed, and the box and the name follow. `None` takes the label away, and the edge can be
/// labelled again.
#[wasm_bindgen_test]
fn a_label_can_be_changed_and_removed_and_put_back() -> Result<(), String> {
    let scene = new_scene("label-change")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("7")).map_err(e)?;
    let narrow = label_rect("label-change", 0)?;

    scene.set_edge_label(edge, Some("a much longer label")).map_err(e)?;
    let wide = label_rect("label-change", 0)?;
    check(wide.size.width > narrow.size.width, "the box grew with its text")?;
    check(
        label_group("label-change", 0)?.get_attribute("aria-label").as_deref()
            == Some("Edge from A to B: a much longer label"),
        "the name follows the text",
    )?;
    let line = connector_points("label-change", 0)?;
    check(!line_hits(&line, wide), "the wider label is still clear of the line")?;

    scene.set_edge_label(edge, None).map_err(e)?;
    check(label_group("label-change", 0).is_err(), "no label is left")?;
    scene.set_edge_label(edge, None).map_err(e)?;
    scene.set_edge_label(edge, Some("again")).map_err(e)?;
    check(label_group("label-change", 0).is_ok(), "a label can be put back")
}

/// An empty label, a label with a line break or a control character, and an edge from another scene are all refused,
/// and nothing is drawn.
#[wasm_bindgen_test]
fn an_invalid_label_or_edge_is_refused() -> Result<(), String> {
    let scene = new_scene("label-invalid")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    for bad in ["", "two\nlines", "tab\there", "bell\u{7}"] {
        let outcome = scene.set_edge_label(edge, Some(bad));
        check(
            matches!(outcome, Err(Error::InvalidEdgeLabel)),
            &format!("{bad:?} gave {outcome:?}"),
        )?;
    }
    check(label_group("label-invalid", 0).is_err(), "a refused label draws nothing")?;

    let other = new_scene("label-other")?;
    let (c, d) = (node(&other, "C", 20.0, 60.0)?, node(&other, "D", 320.0, 60.0)?);
    let foreign: EdgeId = other.add_edge(c, d).map_err(e)?;
    let outcome = scene.set_edge_label(foreign, Some("1"));
    check(
        matches!(outcome, Err(Error::UnknownEdge(_))),
        &format!("a foreign edge gave {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two connectors that leave one node together, as C to D and C to E do in the Dijkstra demo, keep their labels apart,
/// at rest and through a run of drags.
#[wasm_bindgen_test]
fn labels_of_two_connectors_that_leave_one_node_do_not_overlap() -> Result<(), String> {
    let scene = new_scene("label-pair")?;
    let c = node(&scene, "C", 40.0, 200.0)?;
    let (d, f) = (node(&scene, "D", 400.0, 90.0)?, node(&scene, "E", 400.0, 300.0)?);
    let (c_to_d, c_to_e) = (scene.add_edge(c, d).map_err(e)?, scene.add_edge(c, f).map_err(e)?);
    scene.set_edge_label(c_to_d, Some("8")).map_err(e)?;
    scene.set_edge_label(c_to_e, Some("10")).map_err(e)?;

    let apart = |when: &str| -> Result<(), String> {
        let (first, second) = (label_rect("label-pair", 0)?, label_rect("label-pair", 1)?);
        check(
            !overlap(first, second),
            &format!("the labels overlap {when}: {first:?} and {second:?}"),
        )
    };
    apart("at rest")?;
    for (to, name) in [
        (Point::new(400.0, 130.0), "with D lower"),
        (Point::new(400.0, 170.0), "with D lower still"),
        (Point::new(300.0, 150.0), "with D nearer"),
        (Point::new(400.0, 90.0), "with D back"),
    ] {
        scene.move_node(d, to).map_err(e)?;
        apart(name)?;
    }
    for (to, name) in [
        (Point::new(400.0, 260.0), "with E higher"),
        (Point::new(400.0, 300.0), "with E back"),
    ] {
        scene.move_node(f, to).map_err(e)?;
        apart(name)?;
    }
    Ok(())
}

/// A label that was placed earlier is not moved to make room for one placed later: the later one gives way.
///
/// Two connectors leave one node. The second is then moved until it runs almost alongside the first, which is where the
/// first one's label is. Nothing else stands in the way, so there is room for the second label elsewhere.
#[wasm_bindgen_test]
fn a_later_label_gives_way_to_an_earlier_one() -> Result<(), String> {
    let scene = new_scene("label-priority")?;
    let c = node(&scene, "C", 40.0, 200.0)?;
    let (d, f) = (node(&scene, "D", 400.0, 90.0)?, node(&scene, "E", 400.0, 400.0)?);
    let (first, second) = (scene.add_edge(c, d).map_err(e)?, scene.add_edge(c, f).map_err(e)?);
    scene.set_edge_label(first, Some("first")).map_err(e)?;
    scene.set_edge_label(second, Some("second")).map_err(e)?;
    let kept = transform_of("label-priority", 0)?;

    for (to, name) in [
        (Point::new(400.0, 300.0), "second edge at 300"),
        (Point::new(400.0, 200.0), "second edge at 200"),
        (Point::new(400.0, 150.0), "second edge at 150"),
    ] {
        scene.move_node(f, to).map_err(e)?;
        check(
            transform_of("label-priority", 0)? == kept,
            &format!("the earlier label was moved for the later one ({name})"),
        )?;
        let (one, two) = (label_rect("label-priority", 0)?, label_rect("label-priority", 1)?);
        check(!overlap(one, two), &format!("the labels overlap ({name}): {one:?} and {two:?}"))?;
    }
    Ok(())
}

/// A label of lower priority that is in the way of a higher one that has just moved is moved aside, so the two never
/// end up overlapping.
///
/// The first connector is swept down towards the second, and the second's label has to be put again to keep clear of
/// it. Both connectors leave one node and end at nodes that stand clear of each other and of the other connector, so
/// there is always room for both labels.
#[wasm_bindgen_test]
fn a_later_label_in_the_way_of_an_earlier_one_that_moves_is_moved_aside() -> Result<(), String> {
    let scene = new_scene("label-reconcile")?;
    let c = node(&scene, "C", 40.0, 200.0)?;
    let (d, f) = (node(&scene, "D", 400.0, 20.0)?, node(&scene, "E", 400.0, 200.0)?);
    let (first, second) = (scene.add_edge(c, d).map_err(e)?, scene.add_edge(c, f).map_err(e)?);
    scene.set_edge_label(first, Some("first")).map_err(e)?;
    scene.set_edge_label(second, Some("second")).map_err(e)?;

    for (to, name) in [
        (Point::new(400.0, 60.0), "first edge at 60"),
        (Point::new(400.0, 100.0), "first edge at 100"),
        (Point::new(400.0, 140.0), "first edge at 140"),
    ] {
        scene.move_node(d, to).map_err(e)?;
        let (one, two) = (label_rect("label-reconcile", 0)?, label_rect("label-reconcile", 1)?);
        check(!overlap(one, two), &format!("the labels overlap ({name}): {one:?} and {two:?}"))?;
    }
    // Whether the second label has to move depends on where the first one lands, so that is not required here. The
    // placer's own tests in `src/geometry/label` cover the pass that moves it. This test requires that the page never
    // shows two overlapping labels.
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node that is moved onto a label puts the label somewhere clear of it, though the label's own edge was not routed
/// again. The node is not joined to anything.
#[wasm_bindgen_test]
fn a_node_moved_onto_an_unrelated_label_moves_the_label_clear_of_it() -> Result<(), String> {
    let scene = new_scene("label-obstacle")?;
    let (a, b) = (node(&scene, "A", 20.0, 200.0)?, node(&scene, "B", 420.0, 200.0)?);
    let stranger = node(&scene, "C", 300.0, 400.0)?;
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("weight")).map_err(e)?;
    let (rect_before, transform_before) = (label_rect("label-obstacle", 0)?, transform_of("label-obstacle", 0)?);

    // Centre the stranger on the label.
    let size = Size::new(60.0, 40.0);
    let onto = Point::new(
        rect_before.origin.x + rect_before.size.width / 2.0 - size.width / 2.0,
        rect_before.origin.y + rect_before.size.height / 2.0 - size.height / 2.0,
    );
    scene.move_node(stranger, onto).map_err(e)?;

    check(
        transform_of("label-obstacle", 0)? != transform_before,
        "the label stayed where the node was moved onto it",
    )?;
    let moved = Rect { origin: onto, size };
    let label = label_rect("label-obstacle", 0)?;
    check(
        !overlap(label, moved),
        &format!("the label {label:?} still overlaps the node {moved:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A label whose move failed is placed again by a later move that leaves a node where it is, and so needs no change of
/// position to be retried.
///
/// Changing the connector type writes the path's `d` and then the label's `transform`. Only `transform` is made to fail,
/// so the connector changes and the label does not, and the call reports the error.
#[wasm_bindgen_test]
fn a_label_that_failed_to_move_is_placed_again_by_a_move_that_changes_nothing() -> Result<(), String> {
    let scene = new_scene("label-retry")?;
    let (a, b) = (node(&scene, "A", 20.0, 40.0)?, node(&scene, "B", 400.0, 300.0)?);
    let edge = scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(e)?;
    scene.set_edge_label(edge, Some("weight")).map_err(e)?;
    let before = transform_of("label-retry", 0)?;

    {
        let _failing = FailingWrites::start(&["transform"])?;
        check(
            scene
                .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 0.0 })
                .is_err(),
            "the injected label write failure should be reported",
        )?;
    }
    check(
        transform_of("label-retry", 0)? == before,
        "the label should be where it was while its write failed",
    )?;

    // The node is moved to where it already is. Nothing moves, but the label is tried again.
    scene.move_node(a, Point::new(20.0, 40.0)).map_err(e)?;
    check(
        transform_of("label-retry", 0)? != before,
        "the label was not placed again by a move that changed nothing",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming moves the content layer, and the label with it. The label's own place does not change, because it is
/// placed in the layer's own units.
#[wasm_bindgen_test]
fn zooming_moves_labels_with_the_content_and_does_not_place_them_again() -> Result<(), String> {
    let scene = new_scene("label-zoom")?;
    let (a, b) = (node(&scene, "A", 20.0, 60.0)?, node(&scene, "B", 320.0, 60.0)?);
    let edge = scene.add_edge(a, b).map_err(e)?;
    scene.set_edge_label(edge, Some("9")).map_err(e)?;
    let (rect_before, transform_before) = (label_rect("label-zoom", 0)?, transform_of("label-zoom", 0)?);
    let content_before = content_layer("label-zoom")?.get_attribute("transform");

    scene.zoom_in().map_err(e)?;

    check(
        content_layer("label-zoom")?.get_attribute("transform") != content_before,
        "the content layer zoomed",
    )?;
    check(
        transform_of("label-zoom", 0)? == transform_before,
        "the label's own transform did not change",
    )?;
    check(label_rect("label-zoom", 0)? == rect_before, "the label's box did not change")?;
    check(
        label_group("label-zoom", 0)?
            .closest("g.svg-dom-graph-content")
            .map_err(|e| format!("{e:?}"))?
            .is_some(),
        "the label is inside the content layer",
    )
}
