//! The group that holds everything a `Scene` draws, and that an application can draw next to it.
//!
//! `Scene::new` adds one `<g class="svg-dom-graph-content">` to its `<svg>`. Every node, connector and label is inside it, and
//! the pan and zoom transform is written on it. An application can append its own elements to that group, and they then pan and
//! zoom with the scene. The discrete-event simulator does this for its queue window (design: `dess/s0/docs/content-hook.md`),
//! as the Petri-net demo does, so these tests pin what that relies on.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, ViewTransform};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| "no document".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn content_groups(id: &str) -> Result<Vec<web_sys::Element>, String> {
    let found = document()?
        .query_selector_all(&format!("#{id} > g.svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?;
    (0..found.length())
        .map(|i| {
            found
                .get(i)
                .ok_or_else(|| "a group vanished".to_owned())?
                .dyn_into::<web_sys::Element>()
                .map_err(|_| "not an Element".to_owned())
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `<rect>` of `side` units, at `(x, y)` in the content group's own coordinates, appended to `parent`.
fn append_rect(parent: &web_sys::Element, x: f64, y: f64, side: f64, class: &str) -> Result<web_sys::Element, String> {
    let rect = document()?
        .create_element_ns(Some(SVG_NS), "rect")
        .map_err(|e| format!("{e:?}"))?;
    for (name, value) in [
        ("x", x.to_string()),
        ("y", y.to_string()),
        ("width", side.to_string()),
        ("height", side.to_string()),
        ("class", class.to_owned()),
        ("fill", "none".to_owned()),
    ] {
        rect.set_attribute(name, &value).map_err(|e| format!("{e:?}"))?;
    }
    parent.append_child(&rect).map_err(|e| format!("{e:?}"))?;
    Ok(rect)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn rect_of(element: &web_sys::Element) -> (f64, f64, f64, f64) {
    let r = element.get_bounding_client_rect();
    (r.x(), r.y(), r.width(), r.height())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0))).map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scene has exactly one content group, a direct child of the `<svg>`, and a node is inside it.
#[wasm_bindgen_test]
fn a_scene_has_one_content_group_and_its_nodes_are_inside_it() -> Result<(), String> {
    let s = scene("hook-one")?;
    s.add_node(Point::new(20.0, 20.0), Size::new(80.0, 36.0), "server")
        .map_err(|e| e.to_string())?;
    let groups = content_groups("hook-one")?;
    check(
        groups.len() == 1,
        &format!("{} content groups, and one was meant", groups.len()),
    )?;
    let node = nth_group("hook-one", 0)?;
    check(
        groups[0].contains(Some(&node)),
        "the node is not inside the content group, so an overlay in the group is not in the node's coordinates",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An element appended to the content group is moved and scaled by the scene's view exactly as a node is.
#[wasm_bindgen_test]
fn an_element_in_the_content_group_follows_the_view_as_a_node_does() -> Result<(), String> {
    let s = scene("hook-view")?;
    let node = s
        .add_node(Point::new(20.0, 20.0), Size::new(80.0, 36.0), "server")
        .map_err(|e| e.to_string())?;
    let group = content_groups("hook-view")?.remove(0);
    let overlay = append_rect(&group, 120.0, 40.0, 30.0, "overlay")?;
    let frame = nth_group("hook-view", 0)?;
    let _ = node;

    let (node_before, overlay_before) = (rect_of(&frame), rect_of(&overlay));
    s.set_view(ViewTransform {
        scale: 2.0,
        tx: 30.0,
        ty: -10.0,
    })
    .map_err(|e| e.to_string())?;
    let (node_after, overlay_after) = (rect_of(&frame), rect_of(&overlay));

    check(
        (overlay_after.2 / overlay_before.2 - 2.0).abs() < 1e-6,
        "the overlay was not scaled by the view",
    )?;
    check(
        (node_after.2 / node_before.2 - 2.0).abs() < 1e-6,
        "the node was not scaled by the view",
    )?;
    // The distance between the two, in screen units, doubles with the scale: both are in the one transformed group.
    let (dx_before, dy_before) = (overlay_before.0 - node_before.0, overlay_before.1 - node_before.1);
    let (dx_after, dy_after) = (overlay_after.0 - node_after.0, overlay_after.1 - node_after.1);
    check(
        (dx_after - 2.0 * dx_before).abs() < 1e-6 && (dy_after - 2.0 * dy_before).abs() < 1e-6,
        &format!("the overlay and the node moved apart: ({dx_before}, {dy_before}) became ({dx_after}, {dy_after})"),
    )?;
    check(group.contains(Some(&overlay)), "the view moved the overlay out of the group")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a scene does while it is used leaves an application's element in the group alone: drawing nodes and edges, moving a
/// node, giving a data node values, and changing the view.
#[wasm_bindgen_test]
fn using_the_scene_leaves_an_application_element_in_the_group_alone() -> Result<(), String> {
    let s = scene("hook-survive")?;
    let a = s
        .add_node(Point::new(20.0, 20.0), Size::new(80.0, 36.0), "S0")
        .map_err(|e| e.to_string())?;
    let b = s
        .add_node(Point::new(220.0, 20.0), Size::new(80.0, 36.0), "S1")
        .map_err(|e| e.to_string())?;
    let data = s
        .add_data_node(
            Point::new(20.0, 120.0),
            DataNodeContent::new(NodeValues::U32(vec![0; 4]), DataFormat::Decimal).with_layout(GridLayout::Columns(4)),
        )
        .map_err(|e| e.to_string())?;
    let group = content_groups("hook-survive")?.remove(0);
    let overlay = append_rect(&group, 120.0, 90.0, 30.0, "overlay")?;

    s.add_edge(a, b).map_err(|e| e.to_string())?;
    s.move_node(a, Point::new(30.0, 30.0)).map_err(|e| e.to_string())?;
    s.set_data_values(data, NodeValues::U32(vec![1, 2, 3, 4]))
        .map_err(|e| e.to_string())?;
    s.set_view(ViewTransform { scale: 1.5, tx: 5.0, ty: 5.0 })
        .map_err(|e| e.to_string())?;

    check(group.contains(Some(&overlay)), "the overlay is no longer in the content group")?;
    check(
        overlay.get_attribute("class").as_deref() == Some("overlay"),
        "the overlay was changed",
    )?;
    check(
        content_groups("hook-survive")?.len() == 1,
        "the scene made another content group",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node drawn after the overlay is painted over it, since the group paints in the order its children were appended. So the
/// overlay is added once the scene's nodes are all drawn, as the simulator's fixed topology allows.
#[wasm_bindgen_test]
fn a_node_drawn_after_the_overlay_is_painted_over_it() -> Result<(), String> {
    let s = scene("hook-order")?;
    s.add_node(Point::new(20.0, 20.0), Size::new(80.0, 36.0), "first")
        .map_err(|e| e.to_string())?;
    let group = content_groups("hook-order")?.remove(0);
    let overlay = append_rect(&group, 120.0, 90.0, 30.0, "overlay")?;
    s.add_node(Point::new(220.0, 20.0), Size::new(80.0, 36.0), "later")
        .map_err(|e| e.to_string())?;

    let children = group.children();
    let position =
        |element: &web_sys::Element| (0..children.length()).find(|&i| children.item(i).as_ref() == Some(element));
    let later = nth_group("hook-order", 1)?;
    match (position(&overlay), position(&later)) {
        (Some(o), Some(l)) => check(l > o, "a node drawn later was not appended after the overlay"),
        other => Err(format!("a child was not found: {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping a scene leaves its drawing in the document, and a second scene on the same `<svg>` adds a second content group.
/// So a scene is not rebuilt in place: the host is cleared or replaced first, and the hook is one group for each scene.
#[wasm_bindgen_test]
fn a_second_scene_on_the_same_svg_adds_a_second_content_group() -> Result<(), String> {
    let svg = make_svg("hook-two", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let first = Scene::new(svg).map_err(|e| e.to_string())?;
    drop(first);
    check(
        content_groups("hook-two")?.len() == 1,
        "dropping the scene removed its content group",
    )?;
    let again = svg_dom::SvgRoot::attach("hook-two").map_err(|e| e.to_string())?;
    let _second = Scene::new(again).map_err(|e| e.to_string())?;
    check(
        content_groups("hook-two")?.len() == 2,
        "a second scene did not add its own content group",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A thousand consecutive commits through one data node, with an overlay write beside each, change no element count: the
/// scene is not rebuilt, and nothing accumulates. Each commit puts the slice number in the node, so every write changes
/// its content, even for a slice that changed nothing else.
#[wasm_bindgen_test]
fn a_thousand_commits_through_one_data_node_add_no_elements() -> Result<(), String> {
    let s = scene("hook-sustained")?;
    let data = s
        .add_data_node(
            Point::new(20.0, 20.0),
            DataNodeContent::new(NodeValues::U32(vec![0; 8]), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(4))
                .with_cell_width_hint("0000000"),
        )
        .map_err(|e| e.to_string())?;
    let group = content_groups("hook-sustained")?.remove(0);
    let slots: Vec<web_sys::Element> = (0..12)
        .map(|i| append_rect(&group, 150.0 + f64::from(i) * 10.0, 20.0, 8.0, "slot"))
        .collect::<Result<_, _>>()?;
    let count = || -> Result<u32, String> {
        Ok(document()?
            .query_selector_all("#hook-sustained *")
            .map_err(|e| format!("{e:?}"))?
            .length())
    };
    let elements = count()?;

    for slice in 1..=1000u32 {
        s.set_data_values(data, NodeValues::U32((0..8).map(|i| slice * 10 + i).collect()))
            .map_err(|e| format!("commit {slice}: {e}"))?;
        for (i, slot) in slots.iter().enumerate() {
            let shown = if (slice as usize + i) % 3 == 0 { "slot active" } else { "slot" };
            slot.set_attribute("class", shown).map_err(|e| format!("{e:?}"))?;
        }
    }
    check(count()? == elements, &format!("{} elements became {}", elements, count()?))?;
    let group_node = nth_group("hook-sustained", 0)?;
    let first_text = group_node
        .query_selector("text")
        .map_err(|e| format!("{e:?}"))?
        .and_then(|t| t.text_content())
        .unwrap_or_default();
    check(
        first_text == "10000",
        &format!("the first cell shows {first_text:?}, and the last commit was 10000"),
    )
}
