//! Performance. Zoom and pan should cost the same however big the graph is: one group's `transform`, written at most
//! once per animation frame, with nothing beneath it touched.

use super::support::*;
use crate::common::{check, dispatch_pointer_event, nth_connector, nth_group, path_d};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::ToolbarOptions;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

/// One recorded DOM mutation, reduced to what a test needs: whether it hit `content` itself, its type, and which
/// attribute it wrote.
type Mutation = (bool, String, Option<String>);

/// Records every DOM mutation anywhere at or below a target: attributes, children, and text.
///
/// The browser delivers records to the observer's callback at the next microtask checkpoint, which any `await` reaches.
/// So the callback is where they are collected. [`Recorder::pending`] reads only those not yet delivered, for a check
/// made before the test has yielded.
struct Recorder {
    observer: web_sys::MutationObserver,
    content: web_sys::Element,
    log: std::rc::Rc<std::cell::RefCell<Vec<Mutation>>>,
    // Kept alive for as long as the observer may call it.
    _callback: wasm_bindgen::closure::Closure<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>,
}

fn summarise(records: &js_sys::Array, content: &web_sys::Element) -> Vec<Mutation> {
    (0..records.length())
        .map(|i| records.get(i).unchecked_into::<web_sys::MutationRecord>())
        .map(|record| {
            let on_content = record.target().is_some_and(|t| content.is_same_node(Some(&t)));
            (on_content, record.type_(), record.attribute_name())
        })
        .collect()
}

impl Recorder {
    fn watch(target: &web_sys::Element) -> Result<Self, String> {
        let log: std::rc::Rc<std::cell::RefCell<Vec<Mutation>>> = Default::default();
        let (sink, content) = (log.clone(), target.clone());
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>::new(
            move |records: js_sys::Array, _observer| sink.borrow_mut().extend(summarise(&records, &content)),
        );

        let observer =
            web_sys::MutationObserver::new(callback.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
        let init = web_sys::MutationObserverInit::new();
        init.set_attributes(true);
        init.set_child_list(true);
        init.set_subtree(true);
        init.set_character_data(true);
        observer.observe_with_options(target, &init).map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            observer,
            content: target.clone(),
            log,
            _callback: callback,
        })
    }

    /// Mutations made but not yet delivered — that is, made since the test last yielded.
    fn pending(&self) -> Vec<Mutation> {
        summarise(&self.observer.take_records(), &self.content)
    }

    /// Every mutation delivered so far. Call after an `await`, which is what delivers them.
    fn delivered(&self) -> Vec<Mutation> {
        self.log.borrow().clone()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A graph of nodes and connectors, then every way of zooming and panning it: buttons, wheel, drag, and keyboard. The
/// only thing that may change anywhere at or below the content layer is that layer's own `transform`. No node moves, no
/// connector is rerouted, and no label or marker is rewritten — which is what makes the cost independent of graph size.
#[wasm_bindgen_test]
async fn zooming_and_panning_change_only_the_content_layers_own_transform() -> Result<(), String> {
    let scene = new_scene("perf-only-transform")?;
    let mut nodes = Vec::new();
    for i in 0..6 {
        let x = 20.0 + 60.0 * f64::from(i % 3);
        let y = 20.0 + 70.0 * f64::from(i / 3);
        nodes.push(
            scene
                .add_node(Point::new(x, y), Size::new(40.0, 24.0), format!("n{i}"))
                .map_err(|e| e.to_string())?,
        );
    }
    for pair in nodes.windows(2) {
        scene.add_edge(pair[0], pair[1]).map_err(|e| e.to_string())?;
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let content_element = content("perf-only-transform")?;
    let surface = pan_surface("perf-only-transform")?;
    let root = focus_target("perf-only-transform")?;
    let recorder = Recorder::watch(&content_element)?;

    // Every route into the view.
    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_out().map_err(|e| e.to_string())?;
    click(&button("perf-only-transform", 0)?)?;
    for _ in 0..12 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 150, 130, 1)?;
    key(&root, "ArrowRight", false, false, false)?;
    key(&root, "+", false, false, false)?;
    click(&button("perf-only-transform", 2)?)?;
    next_frame().await?;

    let mutations = recorder.delivered();
    check(
        !mutations.is_empty(),
        "test setup: nothing was recorded, so this proves nothing",
    )?;
    for (on_content, kind, attribute) in &mutations {
        check(*on_content, "a zoom or pan changed something beneath the content layer")?;
        check(kind == "attributes", &format!("a zoom or pan made a {kind:?} mutation"))?;
        check(
            attribute.as_deref() == Some("transform"),
            &format!("a zoom or pan wrote {attribute:?}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cost of a burst is one write, not one per event. Twenty wheel events and a run of pan moves, all inside one
/// frame, are a single mutation of the content layer's `transform`.
#[wasm_bindgen_test]
async fn a_burst_of_wheel_and_pan_events_is_one_transform_write() -> Result<(), String> {
    let scene = new_scene("perf-one-write")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let content_element = content("perf-one-write")?;
    let surface = pan_surface("perf-one-write")?;
    next_frame().await?; // Let anything from setup settle, so the frame below is a clean one.

    let recorder = Recorder::watch(&content_element)?;
    for _ in 0..20 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    for step in 1..=10 {
        dispatch_pointer_event(&surface, "pointermove", 100 + 5 * step, 100, 1)?;
    }
    // Still inside the same frame: none of those thirty events has written the DOM yet.
    check(recorder.pending().is_empty(), "an event wrote the DOM ahead of its frame")?;

    next_frame().await?;
    let writes = recorder.delivered();
    check(
        writes.len() == 1,
        &format!("expected one write for the whole burst, found {}", writes.len()),
    )?;
    dispatch_pointer_event(&surface, "pointerup", 150, 100, 1)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom and pan do not invalidate routing: every node's own position and every connector's own path are byte-for-byte
/// what they were, however far the view has been zoomed and panned.
#[wasm_bindgen_test]
fn zoom_and_pan_leave_every_node_position_and_connector_path_untouched() -> Result<(), String> {
    let scene = new_scene("perf-routing")?;
    let mut nodes = Vec::new();
    for i in 0..4 {
        nodes.push(
            scene
                .add_node(
                    Point::new(20.0 + 90.0 * f64::from(i), 40.0 + 30.0 * f64::from(i % 2)),
                    Size::new(50.0, 24.0),
                    format!("n{i}"),
                )
                .map_err(|e| e.to_string())?,
        );
    }
    for pair in nodes.windows(2) {
        scene.add_edge(pair[0], pair[1]).map_err(|e| e.to_string())?;
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let snapshot = || -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for n in 0..4 {
            out.push(attr(&nth_group("perf-routing", n)?, "transform")?);
        }
        for n in 0..3 {
            out.push(path_d(&nth_connector("perf-routing", n)?)?);
        }
        Ok(out)
    };
    let before = snapshot()?;

    for _ in 0..5 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let surface = pan_surface("perf-routing")?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 60, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 160, 60, 1)?;
    key(&focus_target("perf-routing")?, "ArrowLeft", true, false, false)?;

    check(
        snapshot()? == before,
        "zooming and panning changed a node's position or a connector's path",
    )
}
