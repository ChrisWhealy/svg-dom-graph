//! Wasm entry point for `svg-dom-graph`'s demos.
//!
//! Attaches to two `<svg>` elements already present in `index.html`, and builds one small demo scene in each.
//! This crate — not the library — owns every demo-specific decision: which elements to attach to, and what each
//! scene contains.
//!
//! - `#diagram` — [`build_demo_tree`]: a minimal directed tree with straight connectors, showing ordinary dragging
//!   and connector reroute.
//! - `#elbow-diagram` — [`build_elbow_demo`]: two boxes, a straight/elbow toggle, and a corner-radius slider —
//!   see that function's own doc comment for exactly what it demonstrates.
//!
//! Each feature this crate gains should keep this pattern: land alongside a small demo scene of its own, not just a
//! line in the changelog.

use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId, Error,
    scene::{ConnectorOptions, ConnectorType, Scene},
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
    build_elbow_demo(elbow_diagram)
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
