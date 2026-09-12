//! Wasm entry point for `svg-dom-graph`'s demos.
//!
//! Attaches to three `<svg>` elements already present in `index.html`, and builds one small demo scene in each.
//! This crate — not the library — owns every demo-specific decision: which elements to attach to, and what each
//! scene contains.
//!
//! - `#diagram` — [`build_demo_tree`]: a minimal directed tree with straight connectors, showing ordinary dragging
//!   and connector reroute.
//! - `#elbow-diagram` — [`build_elbow_demo`]: two boxes, a straight/elbow toggle, and a corner-radius slider —
//!   see that function's own doc comment for exactly what it demonstrates.
//! - `#edge-anchors-diagram` — [`build_edge_anchors_demo`]: a parent with a growing and shrinking set of children, a
//!   fixing-point slider, and a straight/elbow toggle — see that function's own doc comment for exactly what it
//!   demonstrates.
//!
//! Each feature this crate gains should keep this pattern: land alongside a small demo scene of its own, not just a
//! line in the changelog.

use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId, Error,
    scene::{ConnectorOptions, ConnectorType, EdgeAnchors, NodeOptions, Scene},
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::HtmlInputElement;

thread_local! {
    // `Scene` is a cheap handle around an `Rc`-shared state, and its own listener closures deliberately hold only
    // `Weak` references back to it. A strong self-reference there would leak the whole scene forever. That means
    // nothing keeps a `Scene` alive once the function that built it returns: a `Scene` created, used, and simply let go
    // out of scope (the natural shape of a `#[wasm_bindgen(start)]` function), drops there and then — long before the
    // user ever gets a chance to click anything.  Thus it silently kills every listener with no panic and no console
    // output.
    //
    // `SCENE` keeps `build_demo_tree`'s only `Scene` handle alive for the page's whole lifetime.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    // Same reasoning, for `build_elbow_demo`'s own, separate `Scene`.
    static ELBOW_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    // Same reasoning, for `build_edge_anchors_demo`'s own, separate `Scene`.
    static EDGE_ANCHORS_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build() -> Result<(), Error> {
    let diagram = SvgRoot::attach("diagram")?;
    build_demo_tree(diagram)?;

    let elbow_diagram = SvgRoot::attach("elbow-diagram")?;
    build_elbow_demo(elbow_diagram)?;
    build_edge_anchors_demo()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, straight, arrow-tipped edges. This
/// is a minimal directed tree — the simplest case of the general graph `svg-dom-graph` targets.
///
/// Uses [`ConnectorType::Straight`] deliberately, so this first, simplest demo also shows the crate's original
/// connector style — [`build_elbow_demo`] is where the elbow style, added later, gets its own demonstration.
///
/// The two child boxes are draggable. Their connectors stay attached to the root and redraw as each child moves.
fn build_demo_tree(svg: SvgRoot) -> Result<(), Error> {
    let scene = Scene::new(svg)?;

    let box_size = Size::new(90.0, 50.0);
    let root = scene.add_node(Point::new(155.0, 20.0), box_size, "Root")?;
    let left = scene.add_node(Point::new(25.0, 180.0), box_size, "Left child")?;
    let right = scene.add_node(Point::new(285.0, 180.0), box_size, "Right child")?;

    let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    scene.add_edge_with(root, left, straight)?;
    scene.add_edge_with(root, right, straight)?;

    scene.make_draggable(left)?;
    scene.make_draggable(right)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment above.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the connector-routing demo: two draggable boxes, `P` and `Q`, joined by one connector.
///
/// Demonstrates five things about [`ConnectorType`]:
///
/// 1. Dragging either box reroutes the connector automatically, whichever type is selected.
/// 2. The `#connector-type-straight`/`#connector-type-elbow` radio buttons switch live between
///    [`ConnectorType::Straight`] and [`ConnectorType::Elbow`].
/// 3. The `#corner-radius` slider adjusts an elbow's corner radius live, from `0` up to `80`.
/// 4. `P` and `Q` start close enough together that a radius past about `22` already exceeds the available room. The
///    connector renders clamped to whatever fits, with no error.
/// 5. Dragging `P` or `Q` further apart gives the same requested radius more room, and the full, unclamped radius
///    returns on its own — proving the clamp is recomputed on every redraw, not a one-off correction.
fn build_elbow_demo(svg: SvgRoot) -> Result<(), Error> {
    let scene = Scene::new(svg)?;

    let box_size = Size::new(90.0, 50.0);
    let p = scene.add_node(Point::new(20.0, 20.0), box_size, "P")?;
    let q = scene.add_node(Point::new(230.0, 160.0), box_size, "Q")?;

    scene.make_draggable(p)?;
    scene.make_draggable(q)?;

    let edge = scene.add_edge(p, q)?;

    wire_connector_controls(scene.clone(), edge);

    // Keeps this Scene's only strong handle alive for the page's lifetime — see ELBOW_SCENE's own doc comment.
    ELBOW_SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#connector-type-straight`, `#connector-type-elbow`, and `#corner-radius` together, so any change to any
/// of them recomputes `edge`'s [`ConnectorType`] from all three controls' current state, and applies it.
///
/// Also mirrors the slider's value into `#corner-radius-value`, and disables the slider while "Straight" is
/// selected, since it has nothing to affect there.
///
/// The installed closure captures `scene` and is never dropped — `Closure::forget` leaks it deliberately, for the
/// page's whole lifetime, the same span `ELBOW_SCENE` itself covers. The same closure is registered on all three
/// controls. Each one only ever reads the other two elements' live values, instead of relying on which control
/// fired the event. One shared handler is enough.
///
/// # Panics
///
/// Panics if `index.html` does not define all four of `#connector-type-straight`, `#connector-type-elbow`,
/// `#corner-radius`, and `#corner-radius-value`, with the first three as `<input>` elements. This is demo markup
/// this crate controls, not user input, so a missing element is a bug in this crate, not a runtime condition to
/// recover from.
fn wire_connector_controls(scene: Scene, edge: EdgeId) {
    let document = web_sys::window()
        .expect("no global window")
        .document()
        .expect("no document on window");

    let input = |id: &str| -> HtmlInputElement {
        document
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("index.html must define #{id}"))
            .dyn_into::<HtmlInputElement>()
            .unwrap_or_else(|_| panic!("#{id} must be an <input>"))
    };

    let straight_radio = input("connector-type-straight");
    let elbow_radio = input("connector-type-elbow");
    let radius_slider = input("corner-radius");
    let radius_output = document
        .get_element_by_id("corner-radius-value")
        .expect("index.html must define #corner-radius-value");

    let listeners = [straight_radio.clone(), elbow_radio.clone(), radius_slider.clone()];

    let closure = Closure::<dyn FnMut()>::new(move || {
        let value = radius_slider.value();
        radius_output.set_text_content(Some(&value));
        let corner_radius: f64 = value.parse().unwrap_or(0.0);

        let connector_type = if straight_radio.checked() {
            ConnectorType::Straight
        } else {
            ConnectorType::Elbow { corner_radius }
        };
        radius_slider.set_disabled(straight_radio.checked());

        // Both radio buttons and the slider only ever produce values this crate already accepts, so this never
        // fails in practice. Errors are still ignored, not unwrapped, since a failed update should not crash the
        // page a user is actively interacting with.
        let _ = scene.set_connector_type(edge, connector_type);
    });

    for target in &listeners {
        let event = if target.type_() == "range" { "input" } else { "change" };
        target
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .expect("could not attach a connector-control listener");
    }
    closure.forget();
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most children this demo can show at once. Matches `index.html`'s `#edge-anchors-fixing-points` slider's own
/// `max` attribute.
const MAX_FIXING_POINTS: u8 = 5;

/// `Parent`'s own fixed position and size, and every child's fixed size and row.
const PARENT_X: f64 = 155.0;
const PARENT_Y: f64 = 20.0;
const PARENT_WIDTH: f64 = 90.0;
const PARENT_HEIGHT: f64 = 50.0;
const CHILD_WIDTH: f64 = 60.0;
const CHILD_HEIGHT: f64 = 34.0;
const CHILD_Y: f64 = 180.0;

/// The live scene [`wire_edge_anchors_controls`]' two listeners share, replaced whole every time the fixing-points
/// slider moves — see [`rebuild_edge_anchors_scene`].
struct EdgeAnchorsDemo {
    scene: Scene,
    edges: Vec<EdgeId>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the fixing-points demo: `Parent`, starting with one draggable child, `Child 1`.
///
/// Demonstrates [`EdgeAnchors`]:
///
/// 1. The `#edge-anchors-fixing-points` slider, `0` to [`MAX_FIXING_POINTS`], controls two things at once: how many
///    children are visible, and every visible node's own [`EdgeAnchors`].
/// 2. The visible child count is always `max(1, slider value)`, so at least `Child 1` stays on screen.
/// 3. `0` maps to `None`. `Parent` and `Child 1` then aim their straight connector at each other's own centre,
///    stopping at whichever boundary point that ray crosses first.
/// 4. `1..=`[`MAX_FIXING_POINTS`] map to `Some(EdgeAnchors(n))`. Moving the slider to `1` snaps the connector onto
///    the exact midpoint of the side it crosses. Moving it higher reveals more children, spread evenly across the
///    diagram, and `Parent`'s south side then offers that many evenly spaced fixing points, one per child.
/// 5. `#edge-anchors-type-straight`/`#edge-anchors-type-elbow` switch every edge live between
///    [`ConnectorType::Straight`] and [`ConnectorType::Elbow`].
///
/// `Scene` has no node-move or node-removal API, so a different child count needs each child spread across a new
/// set of positions, not just some hidden.
/// See [`rebuild_edge_anchors_scene`] for why this rebuilds the whole scene from scratch on every slider move,
/// instead of adjusting the one already built.
fn build_edge_anchors_demo() -> Result<(), Error> {
    let document = web_sys::window()
        .expect("no global window")
        .document()
        .expect("no document on window");

    let demo = rebuild_edge_anchors_scene(&document, 0, ConnectorType::Straight)?;
    EDGE_ANCHORS_SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));

    wire_edge_anchors_controls(document, RefCell::new(demo).into());
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `visible_count` evenly spaced x-coordinates for a row of [`CHILD_WIDTH`]-wide boxes, spanning the same width the
/// diagram's own `viewBox` offers.
///
/// A single child centres under `Parent`. Two or more spread edge-to-edge, with equal gaps between them and equal
/// margins on both sides.
fn child_x_positions(visible_count: u8) -> Vec<f64> {
    const MARGIN: f64 = 20.0;
    const VIEWBOX_WIDTH: f64 = 400.0;
    let usable_width = VIEWBOX_WIDTH - 2.0 * MARGIN;

    if visible_count <= 1 {
        return vec![MARGIN + (usable_width - CHILD_WIDTH) / 2.0];
    }

    let count = f64::from(visible_count);
    let gap = (usable_width - CHILD_WIDTH * count) / (count - 1.0);
    (0..visible_count)
        .map(|i| MARGIN + f64::from(i) * (CHILD_WIDTH + gap))
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Clears `#edge-anchors-diagram` and rebuilds it from scratch: `Parent`, `max(1, fixing_points)` draggable
/// children spread by [`child_x_positions`], and `connector_type` connectors between `Parent` and each child.
///
/// Every node's own [`EdgeAnchors`] is `None` if `fixing_points` is `0`, or `Some(EdgeAnchors(fixing_points))`
/// otherwise.
///
/// `Scene` has no node-move API, so a fixing-point count with a different child spread needs a fresh `Scene` built
/// over fresh positions, not an adjustment to the one already rendered. Clearing `#edge-anchors-diagram` first
/// discards the previous scene's own rendered elements; the previous `Scene` handle itself drops once its caller
/// replaces its own reference, taking its listeners with it.
fn rebuild_edge_anchors_scene(
    document: &web_sys::Document,
    fixing_points: u8,
    connector_type: ConnectorType,
) -> Result<EdgeAnchorsDemo, Error> {
    let container = document
        .get_element_by_id("edge-anchors-diagram")
        .expect("index.html must define #edge-anchors-diagram");
    container.set_inner_html("");

    let svg = SvgRoot::attach("edge-anchors-diagram")?;
    let scene = Scene::new(svg)?;

    let edge_anchors = if fixing_points == 0 { None } else { Some(EdgeAnchors(fixing_points)) };
    let node_options = NodeOptions::default().with_edge_anchors(edge_anchors);
    let connector_options = ConnectorOptions::default().with_connector_type(connector_type);

    let parent_origin = Point::new(PARENT_X, PARENT_Y);
    let parent_size = Size::new(PARENT_WIDTH, PARENT_HEIGHT);
    let parent = scene.add_node_with(parent_origin, parent_size, "Parent", node_options)?;

    let child_size = Size::new(CHILD_WIDTH, CHILD_HEIGHT);
    let visible_count = fixing_points.max(1);
    let mut edges = Vec::with_capacity(visible_count as usize);
    for (i, x) in child_x_positions(visible_count).into_iter().enumerate() {
        let label = format!("Child {}", i + 1);
        let child = scene.add_node_with(Point::new(x, CHILD_Y), child_size, label, node_options)?;
        scene.make_draggable(child)?;
        edges.push(scene.add_edge_with(parent, child, connector_options)?);
    }

    Ok(EdgeAnchorsDemo { scene, edges })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#edge-anchors-fixing-points`, `#edge-anchors-type-straight`, and `#edge-anchors-type-elbow` to `state`.
///
/// The slider's own handler rebuilds the whole scene via [`rebuild_edge_anchors_scene`] on every move, replacing
/// `state`'s contents. The radio buttons' shared handler applies the current connector type to every edge `state`
/// currently knows about, without rebuilding. Both installed closures capture `state` and are never dropped —
/// `Closure::forget` leaks them deliberately, for the page's whole lifetime, the same span `EDGE_ANCHORS_SCENE`
/// itself covers.
///
/// # Panics
///
/// Panics if `index.html` does not define all four of `#edge-anchors-fixing-points`,
/// `#edge-anchors-fixing-points-value`, `#edge-anchors-type-straight`, and `#edge-anchors-type-elbow`, with the
/// first, third, and fourth as `<input>` elements. This is demo markup this crate controls, not user input, so a
/// missing element is a bug in this crate, not a runtime condition to recover from.
fn wire_edge_anchors_controls(document: web_sys::Document, state: Rc<RefCell<EdgeAnchorsDemo>>) {
    let input = |id: &str| -> HtmlInputElement {
        document
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("index.html must define #{id}"))
            .dyn_into::<HtmlInputElement>()
            .unwrap_or_else(|_| panic!("#{id} must be an <input>"))
    };

    let fixing_points_slider = input("edge-anchors-fixing-points");
    let fixing_points_output = document
        .get_element_by_id("edge-anchors-fixing-points-value")
        .expect("index.html must define #edge-anchors-fixing-points-value");
    let straight_radio = input("edge-anchors-type-straight");
    let elbow_radio = input("edge-anchors-type-elbow");

    let slider_state = state.clone();
    let slider_document = document.clone();
    let slider = fixing_points_slider.clone();
    let slider_straight_radio = straight_radio.clone();
    let slider_closure = Closure::<dyn FnMut()>::new(move || {
        let value = slider.value();
        fixing_points_output.set_text_content(Some(&value));
        let fixing_points: u8 = value.parse().unwrap_or(0).min(MAX_FIXING_POINTS);

        let connector_type = if slider_straight_radio.checked() {
            ConnectorType::Straight
        } else {
            ConnectorType::Elbow { corner_radius: 0.0 }
        };

        // This demo's own geometry is always valid, so this never fails in practice. The rebuild is still chained
        // through `if let`, not unwrapped: on failure the previous scene stays rendered and live, rather than the
        // page crashing on a stray input event.
        if let Ok(demo) = rebuild_edge_anchors_scene(&slider_document, fixing_points, connector_type) {
            EDGE_ANCHORS_SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));
            *slider_state.borrow_mut() = demo;
        }
    });
    fixing_points_slider
        .add_event_listener_with_callback("input", slider_closure.as_ref().unchecked_ref())
        .expect("could not attach the fixing-points slider listener");
    slider_closure.forget();

    let type_state = state;
    let type_listeners = [straight_radio.clone(), elbow_radio.clone()];
    let type_closure = Closure::<dyn FnMut()>::new(move || {
        let demo = type_state.borrow();
        let connector_type = if straight_radio.checked() {
            ConnectorType::Straight
        } else {
            ConnectorType::Elbow { corner_radius: 0.0 }
        };

        // Same reasoning as the slider handler above: these calls cannot fail in practice. Errors are still
        // ignored rather than unwrapped, so a live page never panics from a stray input event.
        for &edge in &demo.edges {
            let _ = demo.scene.set_connector_type(edge, connector_type);
        }
    });
    for target in &type_listeners {
        target
            .add_event_listener_with_callback("change", type_closure.as_ref().unchecked_ref())
            .expect("could not attach an edge-anchors connector-type listener");
    }
    type_closure.forget();
}
