//! `panel-edge-anchors` / `#edge-anchors-diagram`: a parent with a growing and shrinking set of children, a
//! fixing-point slider, and a straight/elbow toggle. See [`build_edge_anchors_demo`]'s own doc comment for what it
//! demonstrates.

use crate::util::{document, required_element, required_input, stringify, view_box_rect};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId,
    scene::{ConnectorOptions, ConnectorType, DragOptions, EdgeAnchors, NodeOptions, Scene},
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("edge_anchors.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

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
/// 3. `0` maps to `None`. `Parent` and `Child 1` then aim their straight connector at each other's own centre, stopping
///    at whichever boundary point that ray crosses first.
/// 4. `1..=`[`MAX_FIXING_POINTS`] map to `Some(EdgeAnchors(n))`. Moving the slider to `1` snaps the connector onto the
///    exact midpoint of the side it crosses. Moving it higher reveals more children, spread evenly across the diagram,
///    and `Parent`'s south side then offers that many evenly spaced fixing points, one per child.
/// 5. `#edge-anchors-type-straight`/`#edge-anchors-type-elbow` switch every edge live between
///    [`ConnectorType::Straight`] and [`ConnectorType::Elbow`].
///
/// `Scene` has no node-move or node-removal API, so a different child count needs each child spread across a new set of
/// positions, not just some hidden. See [`rebuild_edge_anchors_scene`] for why this rebuilds the whole scene from
/// scratch on every slider move, instead of adjusting the one already built.
///
/// # Errors
///
/// Returns `Err` if any library call fails, if `index.html` is missing `#edge-anchors-diagram`, or if
/// [`wire_edge_anchors_controls`] cannot wire up its controls (see that function's own `# Errors` section).
pub(crate) fn build_edge_anchors_demo() -> Result<(), String> {
    let document = document()?;

    let demo = rebuild_edge_anchors_scene(&document, 0, ConnectorType::Straight)?;
    SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));

    wire_edge_anchors_controls(document, RefCell::new(demo).into())
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
/// Clears `#edge-anchors-diagram` and rebuilds it from scratch: `Parent`, `max(1, fixing_points)` draggable children
/// spread by [`child_x_positions`], and `connector_type` connectors between `Parent` and each child.
///
/// Every node's own [`EdgeAnchors`] is `None` if `fixing_points` is `0`, or
/// `Some(EdgeAnchors(fixing_points))` otherwise.
///
/// `Scene` has no node-move API, so a fixing-point count with a different child spread needs a fresh `Scene` built over
/// fresh positions, not an adjustment to the one already rendered. Clearing `#edge-anchors-diagram` first discards the
/// previous scene's own rendered elements; the previous `Scene` handle itself drops once its caller replaces its own
/// reference, taking its listeners with it.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#edge-anchors-diagram`, or if any library call fails.
fn rebuild_edge_anchors_scene(
    document: &web_sys::Document,
    fixing_points: u8,
    connector_type: ConnectorType,
) -> Result<EdgeAnchorsDemo, String> {
    let container = required_element(document, "edge-anchors-diagram")?;
    container.set_inner_html("");

    let svg = SvgRoot::attach("edge-anchors-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let edge_anchors = if fixing_points == 0 { None } else { Some(EdgeAnchors(fixing_points)) };
    let node_options = NodeOptions::default().with_edge_anchors(edge_anchors);
    let connector_options = ConnectorOptions::default().with_connector_type(connector_type);
    // Bounded to the diagram's own viewBox — see build_demo_tree's own comment for why.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    let parent_origin = Point::new(PARENT_X, PARENT_Y);
    let parent_size = Size::new(PARENT_WIDTH, PARENT_HEIGHT);
    let parent = scene
        .add_node_with(parent_origin, parent_size, "Parent", node_options)
        .map_err(stringify)?;

    let child_size = Size::new(CHILD_WIDTH, CHILD_HEIGHT);
    let visible_count = fixing_points.max(1);
    let mut edges = Vec::with_capacity(visible_count as usize);
    for (i, x) in child_x_positions(visible_count).into_iter().enumerate() {
        let label = format!("Child {}", i + 1);
        let child = scene
            .add_node_with(Point::new(x, CHILD_Y), child_size, label, node_options)
            .map_err(stringify)?;
        scene.make_draggable_with(child, drag_options).map_err(stringify)?;
        edges.push(scene.add_edge_with(parent, child, connector_options).map_err(stringify)?);
    }

    Ok(EdgeAnchorsDemo { scene, edges })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#edge-anchors-fixing-points`, `#edge-anchors-type-straight`, and `#edge-anchors-type-elbow` to `state`.
///
/// The slider's own handler rebuilds the whole scene via [`rebuild_edge_anchors_scene`] on every move, replacing
/// `state`'s contents. The radio buttons' shared handler applies the current connector type to every edge `state`
/// currently knows about, without rebuilding. Both installed closures capture `state` and are never dropped —
/// `Closure::forget` leaks them deliberately, for the page's whole lifetime, the same span `SCENE`
/// itself covers.
///
/// # Errors
///
/// Returns `Err` if:
///
/// - `index.html` is missing `#edge-anchors-fixing-points`, `#edge-anchors-fixing-points-value`,
///   `#edge-anchors-type-straight`, or `#edge-anchors-type-elbow`.
/// - Any of the first, third, or fourth is not an `<input>` element.
/// - A listener could not be attached to any control.
fn wire_edge_anchors_controls(document: web_sys::Document, state: Rc<RefCell<EdgeAnchorsDemo>>) -> Result<(), String> {
    let fixing_points_slider = required_input(&document, "edge-anchors-fixing-points")?;
    let fixing_points_output = required_element(&document, "edge-anchors-fixing-points-value")?;
    let straight_radio = required_input(&document, "edge-anchors-type-straight")?;
    let elbow_radio = required_input(&document, "edge-anchors-type-elbow")?;

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
        // through `if let`, not unwrapped: on failure `state` keeps its previous `Scene` handle rather than being left
        // in a broken half-updated state, and the page does not crash on a stray input event. Note that
        // `rebuild_edge_anchors_scene` clears `#edge-anchors-diagram`'s DOM before it can fail, so a failure here would
        // still leave the container empty even though the old `Scene` handle lives on.
        if let Ok(demo) = rebuild_edge_anchors_scene(&slider_document, fixing_points, connector_type) {
            SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));
            *slider_state.borrow_mut() = demo;
        }
    });
    fixing_points_slider
        .add_event_listener_with_callback("input", slider_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the fixing-points slider listener: {e:?}"))?;
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
            .map_err(|e| format!("could not attach an edge-anchors connector-type listener: {e:?}"))?;
    }
    type_closure.forget();

    Ok(())
}
