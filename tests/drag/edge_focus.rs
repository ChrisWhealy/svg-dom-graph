//! `Scene::set_edge_focus`: a connector emphasised on its own, any number at once, with nothing else about the edge or
//! the scene changed, and independent of the focus ring a node can have.

use crate::common::{check, make_svg, mutations_during, nth_connector, nth_group, path_d};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    EdgeId, Error, NodeId,
    scene::{ConnectorType, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

const PLAIN_WIDTH: &str = "1.5";
const FOCUSED_WIDTH: &str = "3.5";

fn e(error: Error) -> String {
    error.to_string()
}

/// Three nodes in a row with an edge from the first to each of the others, and a label on the first.
fn graph(id: &str) -> Result<(Scene, [NodeId; 3], [EdgeId; 2]), String> {
    let scene = Scene::new(make_svg(id, Size::new(700.0, 400.0), Size::new(700.0, 400.0))).map_err(e)?;
    let node = |name: &str, x: f64, y: f64| scene.add_node(Point::new(x, y), Size::new(60.0, 40.0), name).map_err(e);
    let (a, b, c) = (node("A", 20.0, 160.0)?, node("B", 320.0, 40.0)?, node("C", 320.0, 280.0)?);
    let edges = [scene.add_edge(a, b).map_err(e)?, scene.add_edge(a, c).map_err(e)?];
    Ok((scene, [a, b, c], edges))
}

/// The stroke width of the first node's outer box, which is `3` while the node has its focus ring.
fn node_ring_width(id: &str) -> Result<String, String> {
    nth_group(id, 0)?
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .and_then(|rect| rect.get_attribute("stroke-width"))
        .ok_or_else(|| "the first node has no outer box with a stroke width".to_owned())
}

fn stroke(id: &str, edge: u32) -> Result<(Option<String>, Option<String>), String> {
    let path = nth_connector(id, edge)?;
    Ok((path.get_attribute("stroke"), path.get_attribute("stroke-width")))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A focused edge is thicker, a different colour, and says `aria-current`. Unfocusing puts it exactly back.
#[wasm_bindgen_test]
fn a_focused_edge_is_emphasised_and_unfocusing_restores_it() -> Result<(), String> {
    let (scene, _, [first, _]) = graph("edge-focus-look")?;
    let plain = stroke("edge-focus-look", 0)?;
    check(plain.1.as_deref() == Some(PLAIN_WIDTH), &format!("{plain:?}"))?;

    scene.set_edge_focus(first, true).map_err(e)?;
    let focused = stroke("edge-focus-look", 0)?;
    check(focused.1.as_deref() == Some(FOCUSED_WIDTH), &format!("{focused:?}"))?;
    check(focused.0 != plain.0, "the colour changed")?;
    check(
        nth_connector("edge-focus-look", 0)?.get_attribute("aria-current").as_deref() == Some("true"),
        "aria-current",
    )?;

    scene.set_edge_focus(first, false).map_err(e)?;
    check(stroke("edge-focus-look", 0)? == plain, "unfocusing restored the stroke")?;
    check(
        nth_connector("edge-focus-look", 0)?.get_attribute("aria-current").is_none(),
        "aria-current is gone",
    )
}

/// Edges are focused one at a time and independently, so a whole path can be shown at once.
#[wasm_bindgen_test]
fn any_number_of_edges_can_be_focused_at_once() -> Result<(), String> {
    let (scene, _, [first, second]) = graph("edge-focus-many")?;
    scene.set_edge_focus(first, true).map_err(e)?;
    scene.set_edge_focus(second, true).map_err(e)?;
    check(
        stroke("edge-focus-many", 0)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "the first is focused",
    )?;
    check(
        stroke("edge-focus-many", 1)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "and so is the second",
    )?;

    scene.set_edge_focus(first, false).map_err(e)?;
    check(
        stroke("edge-focus-many", 0)?.1.as_deref() == Some(PLAIN_WIDTH),
        "the first is plain again",
    )?;
    check(
        stroke("edge-focus-many", 1)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "the second is still focused",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Focus changes the look of a line, and nothing else: not the route, the endpoints, the label or the nodes.
#[wasm_bindgen_test]
fn focus_does_not_change_the_route_the_endpoints_or_the_label() -> Result<(), String> {
    let (scene, nodes, [first, _]) = graph("edge-focus-same")?;
    scene.set_edge_label(first, Some("4")).map_err(e)?;
    let (d_before, rects_before) = (
        path_d(&nth_connector("edge-focus-same", 0)?)?,
        nodes.map(|n| scene.node_rect(n).map_err(e)),
    );
    let label = || {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| {
                d.query_selector("#edge-focus-same [data-svg-dom-graph-edge-label]")
                    .ok()
                    .flatten()
            })
            .and_then(|g| g.get_attribute("transform"))
    };
    let label_before = label();

    scene.set_edge_focus(first, true).map_err(e)?;

    check(path_d(&nth_connector("edge-focus-same", 0)?)? == d_before, "the route changed")?;
    check(label() == label_before, "the label moved")?;
    for (node, before) in nodes.into_iter().zip(rects_before) {
        check(scene.node_rect(node).map_err(e)? == before?, "a node moved")?;
    }
    Ok(())
}

/// A focused edge stays focused when its connector is routed again, and a connector type change keeps it too.
#[wasm_bindgen_test]
fn a_focused_edge_stays_focused_when_it_is_rerouted() -> Result<(), String> {
    let (scene, [_, b, _], [first, _]) = graph("edge-focus-reroute")?;
    scene.set_edge_focus(first, true).map_err(e)?;
    let before = path_d(&nth_connector("edge-focus-reroute", 0)?)?;

    scene.move_node(b, Point::new(420.0, 100.0)).map_err(e)?;
    check(
        path_d(&nth_connector("edge-focus-reroute", 0)?)? != before,
        "the edge was rerouted",
    )?;
    check(
        stroke("edge-focus-reroute", 0)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "still thick after a move",
    )?;

    scene
        .set_connector_type(first, ConnectorType::Elbow { corner_radius: 6.0 })
        .map_err(e)?;
    check(
        stroke("edge-focus-reroute", 0)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "still thick after a change of type",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Node focus and edge focus draw on different elements, so neither overrides the other.
#[wasm_bindgen_test]
fn node_focus_and_edge_focus_do_not_override_each_other() -> Result<(), String> {
    let (scene, [a, ..], [first, _]) = graph("edge-focus-node")?;
    scene.set_focus(a, true).map_err(e)?;
    scene.set_edge_focus(first, true).map_err(e)?;
    check(
        stroke("edge-focus-node", 0)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "the edge is focused",
    )?;

    check(node_ring_width("edge-focus-node")? == "3", "the node is ringed")?;
    scene.set_focus(a, false).map_err(e)?;
    check(
        stroke("edge-focus-node", 0)?.1.as_deref() == Some(FOCUSED_WIDTH),
        "unfocusing the node left the edge focused",
    )?;
    scene.set_focus(a, true).map_err(e)?;
    scene.set_edge_focus(first, false).map_err(e)?;
    check(
        stroke("edge-focus-node", 0)?.1.as_deref() == Some(PLAIN_WIDTH),
        "the edge is plain",
    )?;
    check(
        node_ring_width("edge-focus-node")? == "3",
        "unfocusing the edge left the node's ring alone",
    )
}

/// Setting the state an edge is already in writes nothing, and an unknown edge is refused.
#[wasm_bindgen_test]
fn repeating_a_state_writes_nothing_and_an_unknown_edge_is_refused() -> Result<(), String> {
    let (scene, _, [first, _]) = graph("edge-focus-repeat")?;
    let layer = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector("#edge-focus-repeat > g.svg-dom-graph-content").ok().flatten())
        .ok_or("the content layer")?;
    scene.set_edge_focus(first, true).map_err(e)?;
    let writes = mutations_during(&layer, || scene.set_edge_focus(first, true).map_err(e))?;
    check(writes == 0, &format!("{writes} writes for a focus that did not change"))?;
    let writes = mutations_during(&layer, || {
        scene.set_edge_focus(first, false).map_err(e)?;
        scene.set_edge_focus(first, false).map_err(e)
    })?;
    check(writes > 0, "the first unfocus wrote something")?;

    let (other, _, [foreign, _]) = graph("edge-focus-foreign")?;
    let outcome = scene.set_edge_focus(foreign, true);
    check(matches!(outcome, Err(Error::UnknownEdge(_))), &format!("{outcome:?}"))?;
    drop(other);
    Ok(())
}
