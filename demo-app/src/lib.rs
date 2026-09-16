//! Wasm entry point for `svg-dom-graph`'s demos.
//!
//! Each panel builds its own small demo scene the first time it is selected in the gallery — see [`init_panel`],
//! called from JavaScript by `demo/index.template.html`'s own `selectDemo`, not eagerly at page load. This crate
//! owns every demo-specific decision, not the library — which elements to attach to, and what each scene
//! contains.
//!
//! - `panel-tree` / `#diagram` — [`build_demo_tree`]: a minimal directed tree with straight connectors. Shows
//!   ordinary dragging and connector reroute.
//! - `panel-elbow` / `#elbow-diagram` — [`build_elbow_demo`]: two boxes, a straight/elbow toggle, and a
//!   corner-radius slider. See that function's own doc comment for what it demonstrates.
//! - `panel-edge-anchors` / `#edge-anchors-diagram` — [`build_edge_anchors_demo`]: a parent with a growing and
//!   shrinking set of children, a fixing-point slider, and a straight/elbow toggle. See that function's own doc
//!   comment for what it demonstrates.
//! - `panel-data` / `#data-diagram` — [`build_data_demo`]: draggable nodes whose content is a [`DataNodeContent`]
//!   grid of values rather than a plain text label — one single-value and one multi-value node per integer
//!   width (`u8`/`u16`/`u32`/`u64`), the multi-value counts chosen to cover every combination the grid layout
//!   rule can produce. See that function's own doc comment for exactly which.
//!
//! Each feature this crate gains should keep this pattern: land it alongside a small demo scene of its own, add
//! a `demo/panels/{id}.html` fragment and a `demo_gallery!` entry, not just a line in the changelog.
//!
//! No function in this file panics. Every failure — a missing DOM element, a failed listener attach, or a
//! library `Error` — returns as a `Result`. [`init_panel`] reports a build failure directly in the gallery (see
//! [`report_panel_error`]) instead of trapping.

use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    EdgeId,
    scene::{
        ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, DragOptions, EdgeAnchors, NodeOptions,
        NodeValues, Scene,
    },
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{Element, HtmlInputElement};

mod highlight;

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
    // Same reasoning, for `build_data_demo`'s own, separate `Scene`.
    static DATA_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts any displayable error into this crate's own `String` error type.
///
/// Lets a library `Error` and a demo-only DOM failure share one `Result` and one `?`, throughout this file.
fn stringify<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The current page's `Document`, or `Err` if there is no global `window` or no `document` on it.
///
/// Neither should be possible in a real browser. Returned as a graceful `Err` anyway, not a panic, since
/// `run` already has a clean way to report a startup failure to the browser console.
fn document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .ok_or_else(|| "no global window".to_owned())?
        .document()
        .ok_or_else(|| "no document on window".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document`, or `Err` if `index.html` does not define it.
fn required_element(document: &web_sys::Document, id: &str) -> Result<Element, String> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| format!("index.html must define #{id}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document` as an `<input>` element.
///
/// `Err` if `index.html` does not define `#id`, or defines it as something other than `<input>`.
fn required_input(document: &web_sys::Document, id: &str) -> Result<HtmlInputElement, String> {
    required_element(document, id)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("#{id} must be an <input>"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `selector` against `document`, or `Err` if nothing matches.
fn required_query(document: &web_sys::Document, selector: &str) -> Result<Element, String> {
    document
        .query_selector(selector)
        .map_err(|e| format!("query_selector({selector:?}) failed: {e:?}"))?
        .ok_or_else(|| format!("nothing in the DOM matches {selector:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads `svg`'s own `viewBox` attribute back as a [`Rect`], to bound dragging within it.
///
/// `svg-dom`'s own `SvgRoot` deliberately does not cache `viewBox` — see that crate's own doc comment on
/// `set_view_box` for why. Every demo here sets `viewBox` once, directly in `index.html`, so reading the
/// attribute straight from the DOM is simpler than caching it a second time in this crate too.
///
/// # Errors
///
/// Returns `Err` if `svg` has no `viewBox` attribute, or its value is not exactly four numbers.
fn view_box_rect(svg: &SvgRoot) -> Result<Rect, String> {
    let value = svg
        .root
        .get_attribute("viewBox")
        .ok_or_else(|| "the <svg> has no viewBox attribute".to_owned())?;

    let mut numbers = value.split_whitespace().map(str::parse::<f64>);
    let (Some(Ok(x)), Some(Ok(y)), Some(Ok(width)), Some(Ok(height)), None) =
        (numbers.next(), numbers.next(), numbers.next(), numbers.next(), numbers.next())
    else {
        return Err(format!("viewBox {value:?} is not exactly four numbers"));
    };

    Ok(Rect {
        origin: Point::new(x, y),
        size: Size::new(width, height),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Source-code frames
//
// Each demo panel gets a collapsible frame below it showing the formatted Rust source of the function that built
// it, appended by `build` itself right after that function returns successfully. The source text is embedded at
// compile time (`LIB_SOURCE`, via `include_str!`), so it never drifts from what is actually running — there is no
// separate copy of it kept in sync by hand. Mirrors `svg-dom`'s own demo gallery, which does the same thing for
// each of its own many demo files; this crate has only the one file all three demos live in, so there is only one
// entry here, not one per demo, and no module-path lookup is needed to tell same-named functions in different
// files apart.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// This file's own full source, embedded at compile time. [`demo_fn_source`] slices a single top-level function's
/// body out of this text; nothing here is ever hand-copied elsewhere, so it cannot go stale.
const LIB_SOURCE: &str = include_str!("lib.rs");

/// `(panel id, build function, build function name)` for every demo, generated by [`demo_gallery`] below so a
/// panel's id, the function [`init_panel`] calls to build it, and the function name shown in its source frame can
/// never drift apart the way three independently hand-maintained lists could — mirrors `svg-dom`'s own
/// `demo_gallery!`, minus the module-path half `LIB_SOURCE`'s single shared file has no need for.
///
/// The panel id is also the id of the `<section>` (see `demo/panels/*.html`) that [`init_panel`] builds into and
/// [`append_demo_source`] appends the source frame to.
macro_rules! demo_gallery {
    ( $( $panel_id:literal => $name:ident ),+ $(,)? ) => {
        const DEMO_PANELS: &[(&str, fn() -> Result<(), String>, &str)] = &[
            $( ($panel_id, $name, stringify!($name)) ),+
        ];
    };
}

demo_gallery! {
    "panel-tree" => build_demo_tree,
    "panel-elbow" => build_elbow_demo,
    "panel-edge-anchors" => build_edge_anchors_demo,
    "panel-data" => build_data_demo,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns the source text of the top-level `fn {name}` item in [`LIB_SOURCE`], from its signature line through
/// its closing brace, or `None` if it cannot be located.
///
/// Relies on `rustfmt`'s guarantee that a top-level item's own closing brace always sits in column 0, while every
/// brace nested inside the body (including one inside a `format!` string) does not. Scanning for the first line
/// that is exactly `}` after the signature therefore finds the function's end without parsing anything. Mirrors
/// `svg-dom`'s own `demo_fn_source` exactly, bar the module-path lookup a single shared source file has no need
/// for.
fn demo_fn_source(name: &str) -> Option<&'static str> {
    let needle = format!("fn {name}(");
    let hit = LIB_SOURCE.find(&needle)?;
    let start = LIB_SOURCE[..hit].rfind('\n').map_or(0, |i| i + 1);
    let tail = &LIB_SOURCE[hit..];
    let mut from = 0;

    loop {
        let rel = tail[from..].find("\n}")?;
        let close = from + rel + 1; // index of the '}' within `tail`
        let after = close + 1;
        // A genuine top-level close: the '}' stands alone on its line (next byte is a newline or end of file).
        if tail.as_bytes().get(after).is_none_or(|&b| b == b'\n') {
            return Some(&LIB_SOURCE[start..hit + after]);
        }
        from = after;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds `<details class="source" open><summary>...</summary><pre><code>...</code></pre></details>` and appends
/// it to `panel_id`'s own `<section>`, showing the exact source of the function that just built that panel.
/// Mirrors `svg-dom`'s own `append_source_frame`.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#{panel_id}`, if `panel_id` is not registered in [`DEMO_PANELS`], if
/// its function's source cannot be located in [`LIB_SOURCE`] (see [`demo_fn_source`]), or if building any of the
/// DOM nodes below fails.
fn append_demo_source(document: &web_sys::Document, panel_id: &str) -> Result<(), String> {
    let section = required_element(document, panel_id)?;
    let &(_, _, fn_name) = DEMO_PANELS
        .iter()
        .find(|(id, ..)| *id == panel_id)
        .ok_or_else(|| format!("{panel_id} is not registered in DEMO_PANELS"))?;
    let source = demo_fn_source(fn_name).ok_or_else(|| format!("source not found for fn {fn_name}"))?;

    let create = |tag: &str| {
        document
            .create_element(tag)
            .map_err(|e| format!("create_element({tag:?}): {e:?}"))
    };

    let details = create("details")?;
    details.set_attribute("class", "source").map_err(|e| format!("{e:?}"))?;
    details.set_attribute("open", "").map_err(|e| format!("{e:?}"))?;

    let summary = create("summary")?;
    summary.set_text_content(Some(&format!("Rust source — fn {fn_name}")));
    details.append_child(&summary).map_err(|e| format!("{e:?}"))?;

    let pre = create("pre")?;
    let code = create("code")?;
    // `rust_to_html` returns `<span>`-wrapped, HTML-escaped tokens, so angle brackets and ampersands in the code
    // still render verbatim while keywords, strings, etc. are coloured by `demo/style.css`.
    code.set_inner_html(&highlight::rust_to_html(source));
    pre.append_child(&code).map_err(|e| format!("{e:?}"))?;
    details.append_child(&pre).map_err(|e| format!("{e:?}"))?;

    section.append_child(&details).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks a panel's `<section>` with its initialisation outcome, so [`init_panel`] can tell "never attempted"
/// (attribute absent) apart from "already attempted" (attribute present, `"ready"` or `"failed"`) and skip
/// rebuilding a panel that has already been built once — rebuilding it again would duplicate its contents rather
/// than replace them, since each `build_*` function always appends a fresh `<svg>`/child set into its own
/// container. Mirrors `svg-dom`'s own `PANEL_STATE_ATTR`.
const PANEL_STATE_ATTR: &str = "data-panel-state";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds one demo panel's content the first time it is selected, rather than every panel eagerly at page load —
/// which is what this gallery used to do, even though at most one panel is ever visible at once (every other
/// `<section>` sits behind `display: none`; see `.section`/`.section.active` in `style.css`). Mirrors `svg-dom`'s
/// own `init_panel`.
///
/// Call this from JavaScript each time a panel becomes the active one (see `demo/index.template.html`'s
/// `selectDemo`). It is idempotent — see [`PANEL_STATE_ATTR`] — so calling it again for an already-initialised
/// panel, e.g. navigating back to one visited earlier, is a no-op rather than a duplicate rebuild.
///
/// A demo that fails to build is recorded as `"failed"` rather than retried on a later visit; [`run_panel`] and
/// [`report_panel_error`] are what surface that failure in the gallery itself. A missing or mismatched panel id is
/// a different kind of problem — a catalogue error rather than a demo runtime error — and is caught at server
/// startup instead, before the gallery is ever served (see `demo-server/src/validate/mod.rs` and
/// `demo-server/src/panels/mod.rs`'s `assemble`), so `init_panel` never has to distinguish the two: by the time it
/// runs, the catalogue has already been validated.
#[wasm_bindgen]
pub fn init_panel(panel_id: &str) -> Result<(), JsValue> {
    let document = document().map_err(|e| JsValue::from_str(&e))?;

    let Some(section) = document.get_element_by_id(panel_id) else { return Ok(()) };
    if section.has_attribute(PANEL_STATE_ATTR) {
        return Ok(());
    }
    let Some(&(_, build, _)) = DEMO_PANELS.iter().find(|(id, ..)| *id == panel_id) else {
        return Ok(());
    };

    let state = if run_panel(panel_id, build) { "ready" } else { "failed" };
    section
        .set_attribute(PANEL_STATE_ATTR, state)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    // Show the Rust source of the function that just built this panel — including when it failed to build; the
    // source is still worth seeing even when the demo itself broke.
    append_demo_source(&document, panel_id).map_err(|e| JsValue::from_str(&e))?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs one demo's build function, catching its error rather than letting it propagate, and reports whether it
/// succeeded so [`init_panel`] can record that as the panel's state. A broken demo is this function's own runtime
/// behaviour, not a missing or mismatched panel — see [`init_panel`]'s doc comment for why those are handled
/// differently.
fn run_panel(panel_id: &str, build: fn() -> Result<(), String>) -> bool {
    match build() {
        Ok(()) => true,
        Err(err) => {
            report_panel_error(panel_id, &err);
            false
        },
    }
}

/// Appends a visible failure banner to `panel_id`'s own `<section>`, so a demo that fails to build is immediately
/// obvious in the gallery itself instead of just leaving that panel's canvas blank. This is a development and
/// verification tool; a silently empty panel is exactly the outcome worth avoiding here.
///
/// Silently does nothing if the panel section itself cannot be found — by construction (see [`run_panel`]'s doc
/// comment) that cannot happen through the normal `cargo demo` pipeline, so this mirrors [`append_demo_source`]'s
/// own defensive fallback rather than treating an already-impossible case as fatal.
fn report_panel_error(panel_id: &str, message: &str) {
    let Ok(document) = document() else { return };
    let Some(section) = document.get_element_by_id(panel_id) else { return };
    let Ok(banner) = document.create_element("p") else { return };
    banner.set_attribute("class", "demo-error").ok();
    banner.set_text_content(Some(&format!("This demo failed to build: {message}")));
    let _ = section.append_child(&banner);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, straight, arrow-tipped edges. This
/// is a minimal directed tree — the simplest case of the general graph `svg-dom-graph` targets.
///
/// Uses [`ConnectorType::Straight`] deliberately, so this first, simplest demo also shows the crate's original
/// connector style — [`build_elbow_demo`] is where the elbow style, added later, gets its own demonstration.
///
/// The two child boxes are draggable. Their connectors stay attached to the root and redraw as each child moves.
fn build_demo_tree() -> Result<(), String> {
    let svg = SvgRoot::attach("diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let box_size = Size::new(90.0, 50.0);
    let root = scene.add_node(Point::new(155.0, 20.0), box_size, "Root").map_err(stringify)?;
    let left = scene
        .add_node(Point::new(25.0, 180.0), box_size, "Left child")
        .map_err(stringify)?;
    let right = scene
        .add_node(Point::new(285.0, 180.0), box_size, "Right child")
        .map_err(stringify)?;

    let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    scene.add_edge_with(root, left, straight).map_err(stringify)?;
    scene.add_edge_with(root, right, straight).map_err(stringify)?;

    // Bounded to the diagram's own viewBox: without this, a child dragged past the visible edge and dropped there
    // renders clipped, and can never be clicked to pick up again — see `DragOptions::bounds`'s own doc comment.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(left, drag_options).map_err(stringify)?;
    scene.make_draggable_with(right, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment above.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
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
///    follow it back up on its own: it represents the currently applicable radius, not some previously remembered
///    value — see [`refresh_radius_limit`]'s own doc comment for why that is the chosen behaviour, not an oversight.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if [`wire_connector_controls`] cannot wire up its controls
/// (see that function's own `# Errors` section).
fn build_elbow_demo() -> Result<(), String> {
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

    // Keeps this Scene's only strong handle alive for the page's lifetime — see ELBOW_SCENE's own doc comment.
    ELBOW_SCENE.with_borrow_mut(|slot| *slot = Some(scene));

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
/// Matches `index.html`'s own `#corner-radius` `max` attribute: the slider's pre-wasm-load fallback.
const FALLBACK_MAX_RADIUS: f64 = 80.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The smallest per-corner radius `elbow_path_into` actually renders into `connector_path`'s own `d` attribute.
/// Returns `None` if the current route has no corner, or the connector is [`ConnectorType::Straight`].
///
/// Every interior corner that `elbow_path_into` rounds writes an SVG arc command of the shape
/// `A {r} {r} 0 0 {sweep} {x} {y}` into `d`. The token right after `A` is the exact radius used for that corner,
/// already clamped to whatever room its own two segments allow.
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
/// include the query "how much room is actually left".  At that point, it is appropriate to implement a proper library
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
/// the user having touched it, which is considered to be the more surprising of the two slider behaviours a user can
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
/// page's whole lifetime, the same span `ELBOW_SCENE` itself covers. The same shared closure is registered on all three
/// connector-type/radius controls. Each one only ever reads the other two elements' live values, instead of relying on
/// which control fired the event.
///
/// One shared handler is enough for those three. The `pointermove` listener is a second, separate closure:
/// it fires for pointer movement anywhere on the page, not just a change to one of these three controls.
///
/// # Errors
///
/// Returns `Err` if:
///
/// - `index.html` is missing `#connector-type-straight`, `#connector-type-elbow`, `#corner-radius`, or
///   `#corner-radius-value`.
/// - Any of the first three is not an `<input>` element.
/// - `edge`'s connector was not rendered as a `<path>` under `#elbow-diagram`.
/// - A listener could not be attached to any control.
fn wire_connector_controls(scene: Scene, edge: EdgeId) -> Result<(), String> {
    let document = document()?;
    let straight_radio = required_input(&document, "connector-type-straight")?;
    let elbow_radio = required_input(&document, "connector-type-elbow")?;
    let radius_slider = required_input(&document, "corner-radius")?;
    let radius_output = required_element(&document, "corner-radius-value")?;
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
///
/// # Errors
///
/// Returns `Err` if any library call fails, if `index.html` is missing `#edge-anchors-diagram`, or if
/// [`wire_edge_anchors_controls`] cannot wire up its controls (see that function's own `# Errors` section).
fn build_edge_anchors_demo() -> Result<(), String> {
    let document = document()?;

    let demo = rebuild_edge_anchors_scene(&document, 0, ConnectorType::Straight)?;
    EDGE_ANCHORS_SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));

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
/// `Closure::forget` leaks them deliberately, for the page's whole lifetime, the same span `EDGE_ANCHORS_SCENE`
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
            EDGE_ANCHORS_SCENE.with_borrow_mut(|slot| *slot = Some(demo.scene.clone()));
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the data-node demo: one row per integer width (`u8`, `u16`, `u32`, `u64`), each row holding a single-value
/// box on the left and a multi-value box on the right — every box draggable, and every box sized to fit its own content
/// automatically, unlike [`build_demo_tree`]'s caller-sized boxes.
///
/// The four multi-value counts are deliberately chosen to cover every combination
/// [`DataNodeContent::shape`](svg_dom_graph::scene::DataNodeContent::shape)'s layout rule can produce:
///
/// - `u8`, 8 values: divides evenly by a power of two (`2`) but is not itself a square — renders as 2 rows of 4.
/// - `u16`, 5 values: no power-of-two divisor at all — falls back to the closest-to-square shape, 3 rows of 2 (one slot
///   left blank).
/// - `u32`, 9 values: a perfect square with no power-of-two divisor — falls back to the same closest-to-square rule,
///   which for a perfect square is an exact fit: 3 rows of 3.
/// - `u64`, 4 values: both a perfect square and evenly divisible by a power of two (`2`) — the two rules agree, landing
///   on the same 2×2 shape either way.
///
/// A fifth, standalone row below those four shows a single `u64` value under [`DataFormat::Binary`] — 64 digits,
/// nybble-grouped and byte-separated. [`GridLayout::Automatic`](svg_dom_graph::scene::GridLayout::Automatic) sizes
/// a grid by cell *count*, not physical width. So this is also the demo's own worked example of the extreme
/// cell-aspect-ratio case. That case motivates
/// [`GridLayout::MaxColumns`](svg_dom_graph::scene::GridLayout::MaxColumns) — see that type's own doc comment.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#data-diagram`.
fn build_data_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("data-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    // Bounded to the diagram's own viewBox — see build_demo_tree's own comment for why.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every row's single-value box shares this left-hand x; every row's multi-value box shares this one, to its
    // right, regardless of how wide either box actually turns out to be.
    const X_SINGLE: f64 = 20.0;
    const X_MULTI: f64 = 260.0;

    let place = |x: f64, y: f64, content: DataNodeContent| -> Result<(), String> {
        let node = scene.add_data_node(Point::new(x, y), content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)
    };

    // u8: a single value, and 8 values — divides evenly by a power of two (2 rows of 4).
    place(
        X_SINGLE,
        20.0,
        DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        20.0,
        DataNodeContent::new(
            NodeValues::U8(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]),
            DataFormat::Binary,
        ),
    )?;

    // u16: a single value, and 5 values — no power-of-two divisor, so this falls back to the closest-to-square
    // shape (3 rows of 2, one slot blank). Decimal here, rather than hex/binary, to also show a format with no
    // byte-group splitting.
    place(
        X_SINGLE,
        140.0,
        DataNodeContent::new(NodeValues::U16(vec![0x1234]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        140.0,
        DataNodeContent::new(NodeValues::U16(vec![100, 250, 500, 1000, 65535]), DataFormat::Decimal),
    )?;

    // u32: a single value, and 9 values — a perfect square with no power-of-two divisor, landing on an exact 3x3
    // square via the same fallback rule.
    place(
        X_SINGLE,
        290.0,
        DataNodeContent::new(NodeValues::U32(vec![0xDEAD_BEEF]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        290.0,
        DataNodeContent::new(
            NodeValues::U32(vec![
                0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444, 0x5555_5555, 0x6666_6666, 0x7777_7777, 0x8888_8888,
                0x9999_9999,
            ]),
            DataFormat::Hexadecimal,
        ),
    )?;

    // u64: a single value, and 4 values — both a perfect square and evenly divisible by a power of two, so the
    // two layout rules agree on the same 2x2 shape.
    place(
        X_SINGLE,
        440.0,
        DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal),
    )?;
    place(
        X_MULTI,
        440.0,
        DataNodeContent::new(
            NodeValues::U64(vec![
                0x0011223344556677,
                0x8899AABBCCDDEEFF,
                0x1234567890ABCDEF,
                0xFEDCBA0987654321,
            ]),
            DataFormat::Hexadecimal,
        ),
    )?;

    // A single u64 under Binary: 64 digits, nybble-grouped and byte-separated. This cell renders far wider than
    // it is tall — the extreme cell-aspect-ratio case GridLayout::Automatic's own doc comment describes. It is
    // also the one GridLayout::MaxColumns exists to let a caller cap. A standalone row of its own, not paired
    // with a multi-value box, since this single cell is already wide enough to need the room.
    place(
        X_SINGLE,
        590.0,
        DataNodeContent::new(NodeValues::U64(vec![0x0102030405060708]), DataFormat::Binary),
    )?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see DATA_SCENE's own doc comment.
    DATA_SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// These tests only prove every demo function's source is extractable — see `unit_tests.rs`'s own doc comment.
#[cfg(test)]
mod unit_tests;
