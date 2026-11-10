//! What a failed DOM write leaves behind when a data node is drawn (`Scene::add_data_node`) or given new values
//! (`Scene::set_data_values`). Every one of the call's DOM writes is made to fail in turn, which is the sweep that the
//! performance harness's recovery runner makes. The simulator dashboard writes a data node on every displayed slice, so
//! this is the contract that its repair rests on (design: `dess/s0/docs/setter-sweep.md`).
//!
//! The results at the time of writing:
//!
//! * `add_data_node` has 208 counted writes. A failure at any of them leaves the scene as it was, and repeating the
//!   call draws the node. It is `nothing_committed`.
//! * `set_data_values` has 8 counted writes for a grid of 8 cells. A failure at any of them is reported, and leaves
//!   some cells rewritten. It is `partly_committed`, and repeating the same call heals it.
//! * Asking for the old values after a failure used to leave the node torn, since the model still holds them and there
//!   was nothing to rewrite. A call that fails now marks the node's cells as not known to match, and the next call
//!   writes all of them, whatever values it asks for. The tests below that follow the sweep are the repair stage's.

use crate::common::{NthWriteFails, check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

const CELLS: u32 = 8;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn values(first: u32) -> NodeValues {
    NodeValues::U32((0..CELLS).map(|i| first + i).collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Seven digits are drawn, so that every later value fits the cell. A cell keeps the width it was drawn with.
fn content() -> DataNodeContent {
    DataNodeContent::new(values(1_000_000), DataFormat::Decimal)
        .with_layout(GridLayout::Columns(4))
        .with_cell_width_hint("0000000")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn empty_scene(id: &str) -> Result<Scene, String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    Scene::new(svg).map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn scene_with_node(id: &str) -> Result<(Scene, svg_dom_graph::NodeId), String> {
    let scene = empty_scene(id)?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content())
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a reader of the page sees of one data node, and how many elements the scene holds.
#[derive(Debug, PartialEq, Clone)]
struct Picture {
    elements: u32,
    texts: Vec<String>,
    names: Vec<Option<String>>,
    group: (Option<String>, Option<String>),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn picture(id: &str, node_index: u32) -> Result<Picture, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let elements = document
        .query_selector_all(&format!("#{id} *"))
        .map_err(|e| format!("{e:?}"))?
        .length();
    let (mut texts, mut names) = (Vec::new(), Vec::new());
    let mut group = (None, None);
    if elements > 0
        && document
            .query_selector(&format!("#{id} g[role]"))
            .map_err(|e| format!("{e:?}"))?
            .is_some()
    {
        let g = nth_group(id, node_index)?;
        let found = g.query_selector_all("text").map_err(|e| format!("{e:?}"))?;
        for i in 0..found.length() {
            let el = found
                .get(i)
                .ok_or("a text vanished")?
                .dyn_into::<web_sys::Element>()
                .map_err(|_| "text is not an Element")?;
            texts.push(el.text_content().unwrap_or_default());
            names.push(el.get_attribute("aria-label"));
        }
        let title = g
            .query_selector("title")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|t| t.text_content());
        group = (g.get_attribute("aria-label"), title);
    }
    Ok(Picture { elements, texts, names, group })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How one failed write left the scene, compared with what it was before the call.
#[derive(Debug, PartialEq)]
enum Left {
    Nothing,
    SameElementsOtherContent,
    OtherElements,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn left(before: &Picture, after: &Picture) -> Left {
    if before == after {
        Left::Nothing
    } else if before.elements == after.elements {
        Left::SameElementsOtherContent
    } else {
        Left::OtherElements
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A write that fails at any point of drawing a data node leaves the scene as it was, and drawing it again gives the node
/// that a call that never failed gives.
#[wasm_bindgen_test]
fn a_write_that_fails_while_a_data_node_is_drawn_leaves_the_scene_as_it_was() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let twin = empty_scene("da-twin")?;
    NthWriteFails::arm(0)?;
    twin.add_data_node(Point::new(10.0, 10.0), content())
        .map_err(|e| e.to_string())?;
    let writes = NthWriteFails::disarm()?;
    let reference = picture("da-twin", 0)?;
    check(
        writes >= 100,
        &format!("a data node was drawn with only {writes} counted writes"),
    )?;
    check(reference.texts.len() == CELLS as usize, "the twin did not draw its cells")?;

    for fail in 1..=writes {
        let id = format!("da-{fail}");
        let scene = empty_scene(&id)?;
        let before = picture(&id, 0)?;
        NthWriteFails::arm(fail)?;
        let result = scene.add_data_node(Point::new(10.0, 10.0), content());
        NthWriteFails::disarm()?;
        check(
            result.is_err(),
            &format!("write {fail} of {writes} failed, and the call reported success"),
        )?;
        let after = picture(&id, 0)?;
        check(
            left(&before, &after) == Left::Nothing,
            &format!("write {fail} of {writes} failed and left the scene different: {before:?} then {after:?}"),
        )?;
        scene
            .add_data_node(Point::new(10.0, 10.0), content())
            .map_err(|e| format!("repeating after write {fail}: {e}"))?;
        check(
            picture(&id, 0)? == reference,
            &format!("repeating after write {fail} of {writes} did not reach the reference"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A write that fails while a data node is given new values is reported, and leaves some cells rewritten. Repeating the same
/// call rewrites every cell, and gives what a call that never failed gives.
#[wasm_bindgen_test]
fn a_write_that_fails_in_set_data_values_is_reported_and_repeating_the_call_heals_it() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let (twin, node) = scene_with_node("dv-twin")?;
    NthWriteFails::arm(0)?;
    twin.set_data_values(node, values(2_000_000)).map_err(|e| e.to_string())?;
    let writes = NthWriteFails::disarm()?;
    let reference = picture("dv-twin", 0)?;
    check(writes == CELLS, &format!("{writes} writes were counted for {CELLS} cells"))?;

    for fail in 1..=writes {
        let id = format!("dv-{fail}");
        let (scene, node) = scene_with_node(&id)?;
        let before = picture(&id, 0)?;
        NthWriteFails::arm(fail)?;
        let result = scene.set_data_values(node, values(2_000_000));
        NthWriteFails::disarm()?;
        check(
            result.is_err(),
            &format!("write {fail} of {writes} failed, and the call reported success"),
        )?;
        let after = picture(&id, 0)?;
        check(
            left(&before, &after) == Left::SameElementsOtherContent,
            &format!(
                "write {fail} of {writes} left {:?}, and some cells were meant to be rewritten",
                left(&before, &after)
            ),
        )?;
        scene
            .set_data_values(node, values(2_000_000))
            .map_err(|e| format!("repeating after write {fail}: {e}"))?;
        let healed = picture(&id, 0)?;
        check(
            healed.texts == reference.texts && healed.names == reference.names && healed.elements == reference.elements,
            &format!("repeating after write {fail} of {writes} did not reach the reference"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a caller that keeps the last committed values has to do today to put a torn node back: repeat the call that failed,
/// which draws the values that were refused, and then ask for the committed ones.
#[wasm_bindgen_test]
fn a_torn_node_is_put_back_to_its_committed_values_by_repeating_the_call_and_then_asking_for_them() -> Result<(), String>
{
    let _injector = NthWriteFails::install()?;
    for fail in 1..=CELLS {
        let id = format!("dt-{fail}");
        let (scene, node) = scene_with_node(&id)?;
        scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
        let committed = picture(&id, 0)?;
        NthWriteFails::arm(fail)?;
        let result = scene.set_data_values(node, values(2_000_000));
        NthWriteFails::disarm()?;
        check(result.is_err(), &format!("write {fail} did not fail"))?;
        check(
            picture(&id, 0)? != committed,
            &format!("write {fail} left the node as it was, so there is nothing to put back"),
        )?;

        scene.set_data_values(node, values(2_000_000)).map_err(|e| e.to_string())?;
        scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
        check(
            picture(&id, 0)? == committed,
            &format!("after write {fail}, repeating the call and asking for the committed values did not restore them"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the simulator's repair wants, and what finding F1 of the S0 audit asked for: one call with the last committed
/// values puts a torn node back. A call that fails part way marks the node's cells as not known to match its values,
/// and the next call writes every cell, whatever it asks for, since the model still holds the old values and a call
/// that changes nothing would otherwise be skipped.
#[wasm_bindgen_test]
fn one_call_with_the_committed_values_should_put_a_torn_node_back() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    for fail in 1..=CELLS {
        let id = format!("do-{fail}");
        let (scene, node) = scene_with_node(&id)?;
        scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
        let committed = picture(&id, 0)?;
        NthWriteFails::arm(fail)?;
        let _ = scene.set_data_values(node, values(2_000_000));
        NthWriteFails::disarm()?;
        scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
        check(
            picture(&id, 0)? == committed,
            &format!("after write {fail}, one call with the committed values did not restore them"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Any call after a failed one repairs the node, whatever values it asks for: the values that were refused, the
/// committed ones, or some others. After it the node shows exactly the values that were asked for, and equals a node
/// that never failed.
#[wasm_bindgen_test]
fn any_call_after_a_failed_one_writes_every_cell_and_gives_the_values_it_was_asked_for() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    for (what, ask) in [
        ("the refused values", 2_000_000),
        ("the committed values", 1_500_000),
        ("other values", 3_000_000),
    ] {
        // The picture of a node that never failed, given the same values.
        let twin_id = format!("da-twin-{ask}");
        let (twin, twin_node) = scene_with_node(&twin_id)?;
        twin.set_data_values(twin_node, values(ask)).map_err(|e| e.to_string())?;
        let reference = picture(&twin_id, 0)?;
        for fail in 1..=CELLS {
            let id = format!("da-{ask}-{fail}");
            let (scene, node) = scene_with_node(&id)?;
            scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
            NthWriteFails::arm(fail)?;
            let _ = scene.set_data_values(node, values(2_000_000));
            NthWriteFails::disarm()?;
            scene.set_data_values(node, values(ask)).map_err(|e| e.to_string())?;
            let after = picture(&id, 0)?;
            check(
                after.texts == reference.texts
                    && after.names == reference.names
                    && after.elements == reference.elements,
                &format!("after write {fail}, one call with {what} did not give what a node that never failed gives"),
            )?;
        }
    }
    Ok(())
}

/// A repair that fails too leaves the node marked, and the call after it still writes every cell.
#[wasm_bindgen_test]
fn a_repair_that_fails_leaves_the_node_marked_and_the_next_call_writes_every_cell() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let (twin, twin_node) = scene_with_node("dr-twin")?;
    twin.set_data_values(twin_node, values(1_500_000)).map_err(|e| e.to_string())?;
    let reference = picture("dr-twin", 0)?;
    for (first, second) in [(1, 1), (3, 6), (CELLS, 2), (5, CELLS)] {
        let id = format!("dr-{first}-{second}");
        let (scene, node) = scene_with_node(&id)?;
        scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
        NthWriteFails::arm(first)?;
        let _ = scene.set_data_values(node, values(2_000_000));
        NthWriteFails::disarm()?;
        // The repair is asked for the committed values, and it fails at its own write. A marked node writes every cell,
        // so there are as many writes as cells, and the second failure lands on a write that exists.
        NthWriteFails::arm(second)?;
        let repair = scene.set_data_values(node, values(1_500_000));
        NthWriteFails::disarm()?;
        check(
            repair.is_err(),
            &format!("{first} then {second}: the repair that was made to fail reported success"),
        )?;
        scene
            .set_data_values(node, values(1_500_000))
            .map_err(|e| format!("{first} then {second}: {e}"))?;
        let after = picture(&id, 0)?;
        check(
            after.texts == reference.texts && after.names == reference.names,
            &format!("{first} then {second}: the third call did not restore the committed values"),
        )?;
    }
    Ok(())
}

/// A node that is not marked costs no more than before. A call that changes nothing writes nothing, a call that changes
/// some cells writes only those, and the first call after a failure writes every cell and then clears the mark.
#[wasm_bindgen_test]
fn only_a_marked_node_is_written_in_every_cell() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let (scene, node) = scene_with_node("dc-1")?;
    scene.set_data_values(node, values(1_500_000)).map_err(|e| e.to_string())?;
    let writes = |scene: &Scene, ask: u32| -> Result<u32, String> {
        NthWriteFails::arm(0)?;
        scene.set_data_values(node, values(ask)).map_err(|e| e.to_string())?;
        NthWriteFails::disarm()
    };
    check(writes(&scene, 1_500_000)? == 0, "a call that changes nothing wrote something")?;
    // Two cells differ: the first and the last.
    let mut some = (0..CELLS).map(|i| 1_500_000 + i).collect::<Vec<_>>();
    some[0] += 100;
    some[CELLS as usize - 1] += 100;
    NthWriteFails::arm(0)?;
    scene
        .set_data_values(node, NodeValues::U32(some.clone()))
        .map_err(|e| e.to_string())?;
    check(
        NthWriteFails::disarm()? == 2,
        "a call that changed two cells did not write exactly two",
    )?;
    // A failure marks the node, and the next call writes every cell, once, and then clears the mark. It asks for the
    // values that the model holds, so that nothing differs and only the mark makes it write.
    NthWriteFails::arm(3)?;
    let _ = scene.set_data_values(node, values(2_000_000));
    NthWriteFails::disarm()?;
    NthWriteFails::arm(0)?;
    scene
        .set_data_values(node, NodeValues::U32(some.clone()))
        .map_err(|e| e.to_string())?;
    check(
        NthWriteFails::disarm()? == CELLS,
        "the first call after a failure did not write every cell",
    )?;
    NthWriteFails::arm(0)?;
    scene.set_data_values(node, NodeValues::U32(some)).map_err(|e| e.to_string())?;
    check(
        NthWriteFails::disarm()? == 0,
        "the mark was not cleared by a call that wrote every cell",
    )?;
    Ok(())
}

/// A node that holds one value is torn in the same way, and a call with the committed value puts it back, name
/// included.
#[wasm_bindgen_test]
fn a_single_value_node_that_is_torn_is_put_back_by_one_call_with_its_committed_value() -> Result<(), String> {
    let _injector = NthWriteFails::install()?;
    let make = |id: &str| -> Result<(Scene, svg_dom_graph::NodeId), String> {
        let scene = empty_scene(id)?;
        let node = scene
            .add_data_node(
                Point::new(10.0, 10.0),
                DataNodeContent::new(NodeValues::U32(vec![1_500_000]), DataFormat::Decimal)
                    .with_cell_width_hint("0000000"),
            )
            .map_err(|e| e.to_string())?;
        Ok((scene, node))
    };
    let (twin, twin_node) = make("ds-twin")?;
    NthWriteFails::arm(0)?;
    twin.set_data_values(twin_node, NodeValues::U32(vec![2_000_000]))
        .map_err(|e| e.to_string())?;
    let writes = NthWriteFails::disarm()?;
    check(writes >= 1, "a single value is written at least once")?;
    for fail in 1..=writes {
        let id = format!("ds-{fail}");
        let (scene, node) = make(&id)?;
        let committed = picture(&id, 0)?;
        NthWriteFails::arm(fail)?;
        let result = scene.set_data_values(node, NodeValues::U32(vec![2_000_000]));
        NthWriteFails::disarm()?;
        check(result.is_err(), &format!("write {fail} of {writes} did not fail"))?;
        scene
            .set_data_values(node, NodeValues::U32(vec![1_500_000]))
            .map_err(|e| e.to_string())?;
        check(
            picture(&id, 0)? == committed,
            &format!("after write {fail} of {writes}, one call with the committed value did not put the node back"),
        )?;
    }
    Ok(())
}
