//! `panel-elbow` / `#elbow-diagram`: two boxes, a straight/elbow toggle, and a corner-radius slider. See
//! [`build_elbow_demo`]'s own doc comment for what it demonstrates.

use demo_common::util::{document, required_input, required_query, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::controls::sliders::{self, Slider};
use svg_dom_graph::{
    EdgeId,
    scene::{ConnectorType, DragOptions, Scene},
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::Element;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("elbow.rs");

/// The size of the part of the scene that the boxes are in, and of the whole scene, which has a strip under that for
/// the slider. The boxes cannot be dragged into the strip.
const DIAGRAM: Size = Size { width: 420.0, height: 300.0 };
const SCENE_SIZE: Size = Size { width: 420.0, height: 360.0 };

/// The slider in the strip: its id, where it is, how long it is, and the greatest value that it has before there is a
/// route to read the room from.
const SLIDER: &str = "corner-radius";
const SLIDER_LEFT: f64 = 20.0;
const SLIDER_MIDDLE: f64 = 332.0;
const SLIDER_LENGTH: f64 = 300.0;

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the connector-routing demo: two draggable boxes, `P` and `Q`, joined by one connector.
///
/// Demonstrates six things about [`ConnectorType`]:
///
/// 1. Dragging either box reroutes the connector. This works for whichever type is currently selected.
/// 2. The `#connector-type-straight`/`#connector-type-elbow` radio buttons switch live between
///    [`ConnectorType::Straight`] and [`ConnectorType::Elbow`].
/// 3. The corner-radius slider, drawn in the scene by [`sliders`], adjusts an elbow's corner radius live.
/// 4. `P` and `Q` start close enough that a radius past about `22` already exceeds the available room. That room is
///    half the shorter of the two segments meeting at the connector's one bend — the same limit that
///    `elbow_path_into`'s own doc comment describes.
/// 5. This demo goes further than the library itself. The library just renders whatever fits, silently shrinking an
///    over-large request. This demo also keeps the slider's own `max`, and its value if that value no longer fits, in
///    step with the true limit — see [`refresh_radius_limit`] for how.
/// 6. Dragging `P` or `Q` further apart dynamically raises the slider's own ceiling. The slider's *value* does not
///    follow it back up on its own. It represents the currently applicable radius, not some previously remembered
///    value. See [`refresh_radius_limit`]'s own doc comment for why that is the chosen behaviour, not an oversight.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if [`wire_connector_controls`] cannot wire up its controls (see that
/// function's own `# Errors` section).
pub(crate) fn build_elbow_demo() -> Result<(), String> {
    demo_common::util::ensure_svg_in(
        &demo_common::util::document()?,
        "elbow-diagram-stage",
        "elbow-diagram",
        SCENE_SIZE,
    )?;
    let svg = SvgRoot::attach("elbow-diagram").map_err(stringify)?;
    // Bounded to the part of the scene that the boxes are in, and not the strip of the slider.
    let bounds = Rect {
        origin: Point::new(0.0, 0.0),
        size: DIAGRAM,
    };
    let scene = Scene::new(svg).map_err(stringify)?;

    let box_size = Size::new(90.0, 50.0);
    let p = scene.add_node(Point::new(20.0, 20.0), box_size, "P").map_err(stringify)?;
    let q = scene.add_node(Point::new(230.0, 160.0), box_size, "Q").map_err(stringify)?;

    // Bounded to the diagram's own viewBox — see build_demo_tree's own comment for why.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(p, drag_options).map_err(stringify)?;
    scene.make_draggable_with(q, drag_options).map_err(stringify)?;

    let edge = scene.add_edge(p, q).map_err(stringify)?;

    let document = demo_common::util::document()?;
    let svg_element = sliders::scene(&document, "elbow-diagram")?;
    let slider = Slider {
        label: "Corner radius",
        id: SLIDER,
        left: SLIDER_LEFT,
        middle: SLIDER_MIDDLE,
        width: SLIDER_LENGTH,
        range: (0.0, FALLBACK_MAX_RADIUS),
        step: 1.0,
    };
    sliders::add_slider(&document, &svg_element, &slider, 0.0, "0")?;

    wire_connector_controls(scene.clone(), edge)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A corner radius large enough to exceed whatever room `P`/`Q` could ever realistically offer.
///
/// [`refresh_radius_limit`] explains why this must be implausibly large, not just larger than the slider's own nominal
/// range.
const RADIUS_PROBE: f64 = 1_000_000.0;

/// The corner-radius limit assumed when the current route has no corner to round at all — a straight, single-segment
/// route. [`max_renderable_radius`] cannot read a limit there, since `elbow_path_into` never writes an `A` command for
/// one.
///
/// Also the greatest value that the slider starts with, until the first read of the room.
const FALLBACK_MAX_RADIUS: f64 = 80.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The smallest per-corner radius `elbow_path_into` actually renders into `connector_path`'s own `d` attribute. Returns
/// `None` if the current route has no corner, or the connector is [`ConnectorType::Straight`].
///
/// Every interior corner that `elbow_path_into` rounds writes an SVG arc command of the shape `A {r} {r} 0 0 {sweep}
/// {x} {y}` into `d`. The token right after `A` is the exact radius used for that corner, already clamped to whatever
/// room its own two segments allow.
///
/// This needs no geometry of its own. It just reads back what the library already rendered. That is also why
/// [`refresh_radius_limit`] requests [`RADIUS_PROBE`] first: an ordinary radius that already fits would only prove that
/// it fits, not reveal the true ceiling.
///
/// A route with two corners reports the smaller of the two. One `corner_radius` value on [`ConnectorType::Elbow`]
/// applies to both alike, so the slider cannot ask for more than either allows.
///
/// # PoC coupling between `elbow_path_into` and `max_renderable_radius`, acceptable here
///
/// The demo has been coupled to `elbow_path_into`'s serialization. `d` must be scanned in order to locate the `A`
/// token, which then identifies the elbow point. A future change such as different whitespace, or a relative `a`
/// command, would not break compilation. It would make this function quietly stop finding the `A` token and incorrectly
/// return `None`. It would then fall back to [`FALLBACK_MAX_RADIUS`] which represents a silent wrong answer rather than
/// a loud failure.
///
/// This is acceptable for one demo control, and is preferable to reimplementing the elbow-routing geometry simply to
/// avoid it. However, it is anticpated that in future, a real application will probably need the public API shape to
/// include the query "how much room is actually left". At that point, it is appropriate to implement a proper library
/// method reporting it directly. That removes this string coupling. It also removes the doubled render that
/// [`refresh_radius_limit`]'s probe technique costs on every call.
fn max_renderable_radius(connector_path: &Element) -> Option<f64> {
    let d = connector_path.get_attribute("d")?;
    let mut tokens = d.split_whitespace();
    let mut min_radius: Option<f64> = None;

    while let Some(token) = tokens.next() {
        // Not collapsed into a `&&`-chained `if let`, which is clippy's own preference on a modern toolchain.
        // Let-chains are not yet stable on this crate's declared MSRV (1.85). See the `msrv` CI job, which builds
        // demo-app too, not just the library.
        #[allow(clippy::collapsible_if)]
        if token == "A" {
            if let Some(r) = tokens.next().and_then(|s| s.parse::<f64>().ok()) {
                min_radius = Some(min_radius.map_or(r, |m: f64| m.min(r)));
            }
        }
    }
    min_radius
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keeps the slider's own greatest value, and its value if that value no longer fits, in step with the room `P`/`Q`'s
/// current positions actually offer.
///
/// The library already clamps a too-large `corner_radius` at render time (see `elbow_path_into`). That clamp is
/// invisible to the slider. A user could drag it past the true limit and see the corner stop growing, with no
/// explanation why. A later drag that shrinks the room further would then leave the slider showing a value the
/// connector no longer honours.
///
/// This is what point 5 of [`build_elbow_demo`] means by "further than the library". This function finds today's true
/// ceiling, and makes the slider itself reflect it, not just the rendered path.
///
/// # The slider has no memory of a larger request
///
/// Reading the slider's own current value as `desired` (below), rather than some separately tracked "last value
/// requested by the user" is deliberate. If room shrinks and this function pulls the slider down from, say, `60` to
/// `25`, the slider *becomes* `25`. The previous value of `60` is not hidden away in some cache. If room later returns,
/// only the slider's own `max` rises back up; its value stays at `25` until the user moves it again.
///
/// The alternative is silently restoring the previous value of `60` once room returns. That would move the control
/// without the user having touched it. It is considered the more surprising of the two slider behaviours a user can
/// experience.
///
/// Only called while `Elbow` is selected. `Straight` has no corner to round, so this function leaves the limit
/// untouched until `Elbow` is reselected. It is then recomputed fresh, from whatever room the boxes occupy at that
/// later moment.
///
/// Called from three places:
///
/// - [`wire_connector_controls`]'s own shared control closure.
/// - A `pointermove` listener on the whole document, so a drag in progress is reflected live.
/// - Once at setup, so the initial state reflects `P`/`Q`'s starting positions, not `index.html`'s fallback.
///
/// # The probe technique
///
/// 1. Request [`RADIUS_PROBE`] — a radius no real position of `P`/`Q` could ever satisfy.
/// 2. Read back what [`max_renderable_radius`] says the library actually rendered. With an impossible request, that can
///    only be the current ceiling.
/// 3. Re-request whatever `radius_slider`'s own value should now be, clamped to that ceiling.
///
/// This approach removes the possibility of flickering. `set_connector_type` writes `d` synchronously, and this whole
/// function runs to completion before control returns to the browser's own event loop. `d` is already restored to its
/// real, intended value by the time anything gets painted.
fn refresh_radius_limit(scene: &Scene, edge: EdgeId, connector_path: &Element, layer: &Element) {
    let desired = sliders::value(layer, SLIDER).unwrap_or(0.0);

    // Both calls below only ever request a value this crate already accepts (a large-but-finite radius, or a radius
    // this same function just clamped itself), so neither fails in practice. Errors are still ignored, not unwrapped,
    // for the same reason `wire_connector_controls`'s own closure already ignores `set_connector_type`'s result: a
    // failed update should not crash a page the user is actively dragging.
    let _ = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius: RADIUS_PROBE });

    // Floored once, then reused for both the slider's own greatest value and the clamp below — not two separate
    // computations of "the limit". The slider has a step of 1, so a fractional greatest value (say `22.7`) would let
    // `applied` land on a value (`22.7`) the slider itself could never actually represent. The displayed value, the
    // slider's greatest value, and the connector's own request would then each tell a different story about the same
    // drag.
    let available = max_renderable_radius(connector_path).unwrap_or(FALLBACK_MAX_RADIUS).floor();
    let _ = sliders::set_range(layer, SLIDER, (0.0, available));

    let applied = desired.min(available);
    if (applied - desired).abs() > f64::EPSILON {
        let _ = sliders::set_value(layer, SLIDER, applied, &applied.to_string());
    }

    let _ = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius: applied });
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#connector-type-straight`, `#connector-type-elbow`, and the corner-radius slider in the scene together. Any
/// change to any of them recomputes `edge`'s [`ConnectorType`] from the state of the radio buttons and the slider, and
/// applies it.
///
/// Dims the slider while "Straight" is selected, since it has nothing to affect there.
///
/// While "Elbow" is selected, keeps the slider's own greatest value and its value honest via
/// [`refresh_radius_limit`]. Calls it both here and from a `pointermove` listener, so a drag in progress stays honest
/// too, not just a finished one.
///
/// The installed closures capture `scene` and are never dropped — `Closure::forget` leaks them deliberately, for the
/// page's whole lifetime, the same span `SCENE` itself covers. The same shared function is called by both radio buttons
/// and by the slider. Each one only ever reads the other controls' live state, instead of relying on which control
/// fired the event.
///
/// One shared function is enough for those three. The `pointermove` listener is a second, separate closure: it fires
/// for pointer movement anywhere on the page, not just a change to one of these controls.
///
/// # Errors
///
/// Returns `Err` if:
///
/// - `index.html` is missing `#connector-type-straight` or `#connector-type-elbow`, or either is not an `<input>`.
/// - `edge`'s connector was not rendered as a `<path>` under `#elbow-diagram`.
/// - The scene has no layer for the slider.
/// - A listener could not be attached to any control.
fn wire_connector_controls(scene: Scene, edge: EdgeId) -> Result<(), String> {
    let document = document()?;
    let straight_radio = required_input(&document, "connector-type-straight")?;
    let elbow_radio = required_input(&document, "connector-type-elbow")?;
    let connector_path = required_query(&document, "#elbow-diagram > g.svg-dom-graph-content > path")?;
    let layer = sliders::scene(&document, "elbow-diagram")?;

    // Establishes the slider's real initial ceiling from P/Q's actual starting positions, rather than leaving it at the
    // fallback until the first control change or drag.
    refresh_radius_limit(&scene, edge, &connector_path, &layer);

    // What to do when any control changes. Both radio buttons and the slider share it.
    let apply: Rc<dyn Fn()> = {
        let (scene, layer, path, straight) =
            (scene.clone(), layer.clone(), connector_path.clone(), straight_radio.clone());
        Rc::new(move || {
            let _ = sliders::set_disabled(&layer, SLIDER, straight.checked());

            if straight.checked() {
                // Both radio buttons only ever request a value this crate already accepts, so in practice this can
                // never fail. See refresh_radius_limit's own doc comment for the identical reasoning behind its own
                // ignored results.
                let _ = scene.set_connector_type(edge, ConnectorType::Straight);
            } else {
                refresh_radius_limit(&scene, edge, &path, &layer);
            }
        })
    };

    for target in [&straight_radio, &elbow_radio] {
        let apply = Rc::clone(&apply);
        let closure = Closure::<dyn FnMut()>::new(move || apply());
        target
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach a connector-control listener: {e:?}"))?;
        closure.forget();
    }

    let (slider_layer, slider_apply) = (layer.clone(), Rc::clone(&apply));
    sliders::watch(&document, "elbow-diagram-stage", move |_, value| {
        let _ = sliders::set_value(&slider_layer, SLIDER, value, &value.to_string());
        slider_apply();
    })?;

    // A dedicated listener, rather than folding this into `apply` above. Dragging P or Q fires no event on any of the
    // controls above at all. make_draggable's own pointer listeners live entirely inside the library, invisible to this
    // demo. Listening for `pointermove` on the whole document, rather than attaching to P and Q's own rendered
    // elements, avoids needing to know which element a drag is attached to. It also avoids knowing whether a drag is in
    // progress. This works because refresh_radius_limit is cheap: one parsed attribute string and two attribute writes.
    // So it is simply called on every pointer movement anywhere on the page, instead of filtering down to the movements
    // that actually moved P or Q.
    let pointer_closure = Closure::<dyn FnMut()>::new(move || {
        if !straight_radio.checked() {
            refresh_radius_limit(&scene, edge, &connector_path, &layer);
        }
    });
    document
        .add_event_listener_with_callback("pointermove", pointer_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the corner-radius pointermove listener: {e:?}"))?;
    pointer_closure.forget();

    Ok(())
}
