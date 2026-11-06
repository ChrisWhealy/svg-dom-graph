//! Adding an edge is a transaction. A DOM write that fails at any point of `Scene::add_edge_with` leaves the scene exactly as
//! it was, and the call can be repeated. These tests make every one of the call's DOM writes fail in turn, which is the
//! sweep that the performance harness's recovery runner makes (`perf/runner/page/recovery.js`).
//!
//! Before the call was a transaction, a failure after the connector's `<path>` was attached left that path in the scene, a
//! failure between the two relationship clauses left one end named and not the other, and repeating the call added a second
//! connector beside the stray one.

use crate::common::{NthWriteFails, check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::Scene;
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn two_nodes(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, svg_dom_graph::NodeId), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(10.0, 10.0), Size::new(80.0, 36.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(220.0, 120.0), Size::new(80.0, 36.0), "B")
        .map_err(|e| e.to_string())?;
    Ok((scene, a, b))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every element under `#id`, the number of connectors, and the accessible name and tooltip of each of the two nodes.
#[derive(Debug, PartialEq)]
struct Picture {
    elements: u32,
    connectors: u32,
    names: [(Option<String>, Option<String>); 2],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn picture(id: &str) -> Result<Picture, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let count = |selector: &str| -> Result<u32, String> {
        Ok(document.query_selector_all(selector).map_err(|e| format!("{e:?}"))?.length())
    };
    let name = |n: u32| -> Result<(Option<String>, Option<String>), String> {
        let group = nth_group(id, n)?;
        let title = group
            .query_selector("title")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|t| t.text_content());
        Ok((group.get_attribute("aria-label"), title))
    };
    Ok(Picture {
        elements: count(&format!("#{id} *"))?,
        connectors: count(&format!("#{id} path[marker-end]"))?,
        names: [name(0)?, name(1)?],
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A write that fails at any of the call's DOM writes leaves the scene exactly as it was: the same elements, no connector,
/// and neither node's name or tooltip changed. The call reports the error, and repeating it works and gives the scene that
/// an edge that never failed has.
#[wasm_bindgen_test]
fn a_write_that_fails_at_any_point_of_add_edge_leaves_the_scene_as_it_was() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;

    // A twin that adds the edge with nothing wrong, to learn how many writes there are and what a good result looks like.
    let (twin, a, b) = two_nodes("edge-fail-twin")?;
    NthWriteFails::arm(0)?;
    twin.add_edge(a, b).map_err(|e| e.to_string())?;
    let writes = NthWriteFails::disarm()?;
    let reference = picture("edge-fail-twin")?;
    check(
        writes >= 8,
        &format!("an edge was drawn with only {writes} counted writes, so the sweep is not reaching them"),
    )?;
    check(reference.connectors == 1, "the twin did not draw its connector")?;

    for fail in 1..=writes {
        let id = format!("edge-fail-{fail}");
        let (scene, a, b) = two_nodes(&id)?;
        let before = picture(&id)?;

        NthWriteFails::arm(fail)?;
        let result = scene.add_edge(a, b);
        NthWriteFails::disarm()?;
        check(
            result.is_err(),
            &format!("write {fail} of {writes} failed, and the call reported success"),
        )?;
        let after = picture(&id)?;
        check(
            after == before,
            &format!(
                "write {fail} of {writes} failed and left the scene different:\nbefore {before:?}\nafter  {after:?}"
            ),
        )?;

        // Repeating the call gives what an edge that never failed gives, and not a second connector.
        scene.add_edge(a, b).map_err(|e| format!("repeating after write {fail}: {e}"))?;
        let repeated = picture(&id)?;
        check(
            repeated.elements == reference.elements && repeated.connectors == 1 && repeated.names == reference.names,
            &format!(
                "repeating after write {fail} of {writes} did not reach the reference:\nreference {reference:?}\nrepeated  {repeated:?}"
            ),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The injector fires at the write it is told to, once, and counts nothing when it is not armed. Without that, the sweep above
/// could be passing by never failing anything.
#[wasm_bindgen_test]
fn the_injector_fails_only_the_write_it_is_armed_for() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let element = document.create_element("div").map_err(|e| format!("{e:?}"))?;

    NthWriteFails::arm(2)?;
    check(element.set_attribute("a", "1").is_ok(), "the first write failed")?;
    check(element.set_attribute("b", "2").is_err(), "the second write did not fail")?;
    check(
        element.set_attribute("c", "3").is_ok(),
        "a third write failed, so the fault fired twice",
    )?;
    check(NthWriteFails::disarm()? == 3, "the writes were not all counted")?;
    check(element.set_attribute("d", "4").is_ok(), "an unarmed write failed")
}
