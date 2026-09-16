//! `panel-elbow` / `#elbow-diagram`: two boxes, a straight/elbow toggle, and a corner-radius slider. See
//! [`build_elbow_demo`]'s own doc comment for what it demonstrates.

use crate::util::{document, required_input, required_query, stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId,
    scene::{ConnectorType, DragOptions, Scene},
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{Element, HtmlInputElement};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("elbow.rs");

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
/// 3. The `#corner-radius` slider adjusts an elbow's corner radius live.
/// 4. `P` and `Q` start close enough that a radius past about `22` already exceeds the available room. That room is
///    half the shorter of the two segments meeting at the connector's one bend — the same limit that
///    `elbow_path_into`'s own doc comment describes.
/// 5. This demo goes further than the library itself. The library just renders whatever fits, silently shrinking an
///    over-large request. This demo also keeps the slider's own `max`, and its value if that value no longer fits, in
///    step with the true limit — see [`refresh_radius_limit`] for how.
/// 6. Dragging `P` or `Q` further apart dynamically raises the slider's own ceiling. The slider's *value* does not
///    follow it back up on its own: it represents the currently applicable radius, not some previously remembered value
///    — see [`refresh_radius_limit`]'s own doc comment for why that is the chosen behaviour, not an oversight.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if [`wire_connector_controls`] cannot wire up its controls (see that
/// function's own `# Errors` section).
pub(crate) fn build_elbow_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("elbow-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let box_size = Size::new(90.0, 50.0);
    let p = scene.add_node(Point::new(20.0, 20.0), box_size, "P").map_err(stringify)?;
    let q = scene.add_node(Point::new(230.0, 160.0), box_size, "Q").map_err(stringify)?;

    // Bounded to the diagram's own viewBox — see build_demo_tree's own comment for why.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(p, drag_options).map_err(stringify)?;
    scene.make_draggable_with(q, drag_options).map_err(stringify)?;

    let edge = scene.add_edge(p, q).map_err(stringify)?;

    wire_connector_controls(scene.clone(), edge)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A corner radius large enough to exceed whatever room `P`/`Q` could ever realistically offer.
///
/// [`refresh_radius_limit`] explains why this must be implausibly large, not just larger than the slider's own
/// nominal range.
const RADIUS_PROBE: f64 = 1_000_000.0;

/// The corner-radius limit assumed when the current route has no corner to round at all — a straight, single-segment
/// route. [`max_renderable_radius`] cannot read a limit there, since `elbow_path_into` never writes an `A` command
/// for one.
///
/// Matches `index.html`'s own `#corner-radius` `max` attribute: the slider's pre-wasm-load fallback.
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
/// token, which then identifies the elbow point. A future change such as different whitespace, a relative `a` command
/// etc, would not break compilation, but it would make this function quietly stop finding the `A` token and incorrectly
/// return `None`. It would then fall back to [`FALLBACK_MAX_RADIUS`] which represents a silent wrong answer rather than
/// a loud failure.
///
/// This is acceptable for one demo control, and is preferable to reimplementing the elbow-routing geometry simply to
/// avoid it. However, it is anticpated that in future, a real application will probably need the public API shape to
/// include the query "how much room is actually left". At that point, it is appropriate to implement a proper library
/// method reporting it directly and removing this string coupling and the resulting doubled render that
/// [`refresh_radius_limit`]'s probe technique costs on every call.
fn max_renderable_radius(connector_path: &Element) -> Option<f64> {
    let d = connector_path.get_attribute("d")?;
    let mut tokens = d.split_whitespace();
    let mut min_radius: Option<f64> = None;

    while let Some(token) = tokens.next() {
        // Not collapsed into a `&&`-chained `if let` (clippy's own preference on a modern toolchain): let-chains
        // are not yet stable on this crate's declared MSRV (1.85) — see the `msrv` CI job, which builds demo-app
        // too, not just the library.
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
/// Keeps `radius_slider`'s own `max`, and its value if that value no longer fits, in step with the room `P`/`Q`'s
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
/// Reading `radius_slider`'s own current value as `desired` (below), rather than some separately tracked "last value
/// requested by the user" is deliberate. If room shrinks and this function pulls the slider down from, say, `60` to
/// `25`, the slider *becomes* `25`: the previous value of `60` is not hidden away in some cache. If room later returns,
/// only the slider's own `max` rises back up; its value stays at `25` until the user moves it again.
///
/// The alternative of silently restoring the previous value of `60` once room returns would move the control without
/// the user having touched it, which is considered to be the more surprising of the two slider behaviours a user
/// can experience.
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
fn refresh_radius_limit(
    scene: &Scene,
    edge: EdgeId,
    connector_path: &Element,
    radius_slider: &HtmlInputElement,
    radius_output: &Element,
) {
    let desired: f64 = radius_slider.value().parse().unwrap_or(0.0);

    // Both calls below only ever request a value this crate already accepts (a large-but-finite radius, or a
    // radius this same function just clamped itself), so neither fails in practice. Errors are still ignored, not
    // unwrapped, for the same reason `wire_connector_controls`'s own closure already ignores
    // `set_connector_type`'s result: a failed update should not crash a page the user is actively dragging.
    let _ = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius: RADIUS_PROBE });

    // Floored once, then reused for both the slider's own `max` attribute and the clamp below — not two separate
    // computations of "the limit". `#corner-radius` declares `step="1"`, an integer slider; a fractional `max` (say
    // `22.7`) would let `applied` land on a value (`22.7`) the slider itself could never actually represent, so the
    // displayed value, the slider's declared maximum, and the connector's own request would each tell a different
    // story about the same drag.
    let available = max_renderable_radius(connector_path).unwrap_or(FALLBACK_MAX_RADIUS).floor();

    let max_str = available.to_string();
    if radius_slider.get_attribute("max").as_deref() != Some(max_str.as_str()) {
        let _ = radius_slider.set_attribute("max", &max_str);
    }

    let applied = desired.min(available);
    if (applied - desired).abs() > f64::EPSILON {
        let value = applied.to_string();
        radius_slider.set_value(&value);
        radius_output.set_text_content(Some(&value));
    }

    let _ = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius: applied });
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#connector-type-straight`, `#connector-type-elbow`, and `#corner-radius` together. Any change to any of them
/// recomputes `edge`'s [`ConnectorType`] from all three controls' current state, and applies it.
///
/// Also mirrors the slider's value into `#corner-radius-value`. Disables the slider while "Straight" is selected, since
/// it has nothing to affect there.
///
/// While "Elbow" is selected, keeps the slider's own `max` and value honest via [`refresh_radius_limit`]. Calls it both
/// here and from a `pointermove` listener, so a drag in progress stays honest too, not just a finished one.
///
/// The installed closures capture `scene` and are never dropped — `Closure::forget` leaks them deliberately, for the
/// page's whole lifetime, the same span `SCENE` itself covers. The same shared closure is registered on all three
/// connector-type/radius controls. Each one only ever reads the other two elements' live values, instead of relying on
/// which control fired the event.
///
/// One shared handler is enough for those three. The `pointermove` listener is a second, separate closure: it fires for
/// pointer movement anywhere on the page, not just a change to one of these three controls.
///
/// # Errors
///
/// Returns `Err` if:
///
/// - `index.html` is missing `#connector-type-straight`, `#connector-type-elbow`, `#corner-radius`,
///   or `#corner-radius-value`.
/// - Any of the first three is not an `<input>` element.
/// - `edge`'s connector was not rendered as a `<path>` under `#elbow-diagram`.
/// - A listener could not be attached to any control.
fn wire_connector_controls(scene: Scene, edge: EdgeId) -> Result<(), String> {
    let document = document()?;
    let straight_radio = required_input(&document, "connector-type-straight")?;
    let elbow_radio = required_input(&document, "connector-type-elbow")?;
    let radius_slider = required_input(&document, "corner-radius")?;
    let radius_output = crate::util::required_element(&document, "corner-radius-value")?;
    let connector_path = required_query(&document, "#elbow-diagram > path")?;

    // Establishes the slider's real initial ceiling from P/Q's actual starting positions, rather than leaving it at
    // index.html's own hard-coded fallback until the first control change or drag.
    refresh_radius_limit(&scene, edge, &connector_path, &radius_slider, &radius_output);

    let listeners = [straight_radio.clone(), elbow_radio.clone(), radius_slider.clone()];

    // Cloned before `closure` below moves its own copies of `straight_radio`/`radius_slider`/`radius_output`, so
    // the `pointermove` listener installed further down still has its own handles to the same live elements.
    let pointer_scene = scene.clone();
    let pointer_straight_radio = straight_radio.clone();
    let pointer_slider = radius_slider.clone();
    let pointer_output = radius_output.clone();
    let pointer_path = connector_path.clone();

    let closure_scene = scene;
    let closure_path = connector_path;
    let closure = Closure::<dyn FnMut()>::new(move || {
        radius_output.set_text_content(Some(&radius_slider.value()));
        radius_slider.set_disabled(straight_radio.checked());

        if straight_radio.checked() {
            // Both radio buttons only ever request a value this crate already accepts, so in practice, this can never
            // fail — see refresh_radius_limit's own doc comment for the identical reasoning behind its own ignored
            // results.
            let _ = closure_scene.set_connector_type(edge, ConnectorType::Straight);
        } else {
            refresh_radius_limit(&closure_scene, edge, &closure_path, &radius_slider, &radius_output);
        }
    });

    for target in &listeners {
        let event = if target.type_() == "range" { "input" } else { "change" };
        target
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach a connector-control listener: {e:?}"))?;
    }
    closure.forget();

    // A dedicated listener, rather than folding this into `closure` above: dragging P or Q fires no event on any
    // of the three controls above at all — make_draggable's own pointer listeners live entirely inside the
    // library, invisible to this demo. Listening for `pointermove` on the whole document, rather than trying to
    // attach to P/Q's own rendered elements specifically, sidesteps needing to know which element a drag is
    // currently attached to, or even whether one is in progress: refresh_radius_limit is cheap enough (one parsed
    // attribute string, two attribute writes) to simply call unconditionally on every pointer movement anywhere on
    // the page rather than trying to filter down to just the ones that actually moved P or Q.
    let pointer_closure = Closure::<dyn FnMut()>::new(move || {
        if !pointer_straight_radio.checked() {
            refresh_radius_limit(&pointer_scene, edge, &pointer_path, &pointer_slider, &pointer_output);
        }
    });
    document
        .add_event_listener_with_callback("pointermove", pointer_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the corner-radius pointermove listener: {e:?}"))?;
    pointer_closure.forget();

    Ok(())
}
