//! Small DOM/error helpers shared by more than one demo module.

use std::cell::Cell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

thread_local! {
    // A fresh numeric suffix for every step's own nested child `<svg>` id, shared by every demo module that steps a
    // nested child this way (`theta`'s own `theta_c`/`theta_d`/`xor_loop`, and `sha3_sponge::keccak`) — see
    // [`next_child_svg_id`]'s own doc comment.
    static NEXT_CHILD_SVG_SUFFIX: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A fresh, never-reused id for this step's own nested child `<svg>`, prefixed with `prefix`. Shares one counter across
/// every demo module that calls this, so every id handed out is unique regardless of which nested child it backs.
pub(crate) fn next_child_svg_id(prefix: &str) -> String {
    NEXT_CHILD_SVG_SUFFIX.with(|counter| {
        let n = counter.get();
        counter.set(n + 1);
        format!("{prefix}-{n}")
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates `<svg id="{id}">` — sized `size`, with `viewBox` to match and `class` if given — as the previous sibling of
/// `before_id`, unless an element called `id` already exists, in which case this does nothing at all.
///
/// Lets Rust, not `index.html`, own a diagram's own initial dimensions. Doing nothing when `id` already exists makes
/// this safe to call on every rebuild — a size the diagram has since fitted to its own content is never reverted — and
/// leaves a panel that still declares its own `<svg>` in HTML working unchanged.
///
/// Inserting before `before_id` (rather than appending) keeps DOM order, and so paint order, deterministic: call this
/// for the shallowest `<svg>` first, and `before_id` can be an overlay — such as a close button — that must stay on
/// top.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `before_id`, or if creating or inserting the element fails.
pub(crate) fn ensure_svg(
    document: &web_sys::Document,
    before_id: &str,
    id: &str,
    class: Option<&str>,
    size: Size,
) -> Result<(), String> {
    if document.get_element_by_id(id).is_some() {
        return Ok(());
    }
    let anchor = required_element(document, before_id)?;
    let svg = new_svg(document, id, class, size)?;
    anchor
        .before_with_node_1(&svg)
        .map_err(|e| format!("could not insert <svg id={id:?}>: {e:?}"))?;
    frame_nested_scene(document, id)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`ensure_svg`]'s counterpart for a panel with just one plain `<svg>`: appends `<svg id="{id}">` — sized `size` — to
/// `parent_id`, unless an element called `id` already exists. The HTML then declares only the empty host element (a
/// `.canvas`), so Rust is the one place a diagram's own dimensions are ever written down.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `parent_id`, or if creating or appending the element fails.
pub(crate) fn ensure_svg_in(document: &web_sys::Document, parent_id: &str, id: &str, size: Size) -> Result<(), String> {
    if document.get_element_by_id(id).is_some() {
        return Ok(());
    }
    let parent = required_element(document, parent_id)?;
    let svg = new_svg(document, id, None, size)?;
    parent
        .append_with_node_1(&svg)
        .map_err(|e| format!("could not append <svg id={id:?}>: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A detached `<svg>`: `id`, `width`, `height`, and a matching `viewBox` set from `size`, plus `class` if given.
fn new_svg(
    document: &web_sys::Document,
    id: &str,
    class: Option<&str>,
    size: Size,
) -> Result<web_sys::Element, String> {
    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .map_err(|e| format!("could not create <svg id={id:?}>: {e:?}"))?;
    let (width, height) = (size.width, size.height);
    let attrs = [
        ("id", id.to_string()),
        ("width", width.to_string()),
        ("height", height.to_string()),
        ("viewBox", format!("0 0 {width} {height}")),
    ];
    for (name, value) in attrs.iter().chain(class.map(|c| ("class", c.to_string())).as_ref()) {
        svg.set_attribute(name, value)
            .map_err(|e| format!("could not set {name} on <svg id={id:?}>: {e:?}"))?;
    }
    Ok(svg)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sizes `id`'s own nested-Scene frame, which `style.css` then draws centred in its stage at exactly that size.
///
/// Starts from the `<svg>`'s own `width`/`height` attributes — its natural size — and scales both down by the same
/// factor if that would not fit the stage less `--nested-scene-min-margin` on every side. Scaling both keeps the frame
/// the same shape as its `viewBox`, so nothing is letterboxed. The result is written as `--frame-w`/`--frame-h` on the
/// `<svg>`.
///
/// It also records the same size for `.nested-scene-close`, which sits outside the `<svg>` and so cannot read those. A
/// rule in the `<style id="nested-scene-frame-rules">` element sets `--nested-scene-frame-w`/`-h` on the stage, but
/// only `:has()` this very `<svg>` is shown. Each `<svg>` is matched by a `data-frame-key` attribute it keeps across a
/// step's own clones, which get fresh ids. Several siblings of different sizes can therefore coexist in one stage.
///
/// Does nothing for an `<svg>` without the `nested-scene` class. Call it after writing `width`/`height`; [`resize_svg`]
/// and [`ensure_svg`] already do. A panel whose `<svg>` is still declared in HTML must call it itself, before anything
/// is drawn.
///
/// # Errors
///
/// Returns `Err` if `id` is missing, has no numeric `width`/`height`, or a DOM write fails.
pub(crate) fn frame_nested_scene(document: &web_sys::Document, id: &str) -> Result<(), String> {
    let svg = required_element(document, id)?;
    let class = svg.get_attribute("class").unwrap_or_default();
    if !class.split_whitespace().any(|c| c == "nested-scene") {
        return Ok(());
    }
    let number = |name: &str| -> Result<f64, String> {
        svg.get_attribute(name)
            .and_then(|v| v.trim_end_matches("px").parse().ok())
            .ok_or_else(|| format!("#{id} has no numeric {name}"))
    };
    let (width, height) = (number("width")?, number("height")?);

    // Read from the page's own CSS, not mirrored here. This is the least margin a frame may be squeezed to, not its
    // decorative per-depth inset (`--nested-scene-margin`), which only applies when there is room for it.
    let margin = web_sys::window()
        .and_then(|w| w.get_computed_style(&svg).ok().flatten())
        .and_then(|style| style.get_property_value("--nested-scene-min-margin").ok())
        .and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(0.0);
    let stage = svg.parent_element().ok_or_else(|| format!("#{id} has no parent element"))?;
    // A hidden panel has no layout yet, so its stage measures `0`: leave the frame at its natural size then.
    let (avail_w, avail_h) = (
        f64::from(stage.client_width()) - 2.0 * margin,
        f64::from(stage.client_height()) - 2.0 * margin,
    );
    let scale = if avail_w > 0.0 && avail_h > 0.0 {
        (avail_w / width).min(avail_h / height).min(1.0)
    } else {
        1.0
    };
    let (frame_w, frame_h) = (width * scale, height * scale);

    set_style_vars(
        &svg,
        &[("--frame-w", format!("{frame_w}px")), ("--frame-h", format!("{frame_h}px"))],
    )?;

    let key = match svg.get_attribute("data-frame-key") {
        Some(key) => key,
        None => {
            svg.set_attribute("data-frame-key", id)
                .map_err(|e| format!("could not key #{id}'s own frame: {e:?}"))?;
            id.to_string()
        },
    };
    record_frame_rule(document, key, frame_w, frame_h)
}

thread_local! {
    // Every nested `<svg>`'s own frame size, by `data-frame-key` — the source of `<style
    // id="nested-scene-frame-rules">`'s own text. See [`frame_nested_scene`].
    static FRAME_RULES: std::cell::RefCell<Vec<(String, f64, f64)>> = const { std::cell::RefCell::new(Vec::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds or replaces `key`'s own entry in [`FRAME_RULES`] and rewrites the whole `<style id="nested-scene-frame-rules">`
/// element from it, creating that element in `<head>` on first use.
fn record_frame_rule(document: &web_sys::Document, key: String, width: f64, height: f64) -> Result<(), String> {
    let css = FRAME_RULES.with_borrow_mut(|rules| {
        match rules.iter_mut().find(|(k, ..)| *k == key) {
            Some(rule) => *rule = (key, width, height),
            None => rules.push((key, width, height)),
        }
        rules
            .iter()
            .map(|(k, w, h)| {
                format!(
                    ".nested-scene-stage:has(svg[data-frame-key=\"{k}\"][visibility=\"visible\"]) \
                     {{ --nested-scene-frame-w: {w}px; --nested-scene-frame-h: {h}px; }}\n"
                )
            })
            .collect::<String>()
    });
    let style = match document.get_element_by_id("nested-scene-frame-rules") {
        Some(style) => style,
        None => {
            let style = document
                .create_element("style")
                .map_err(|e| format!("could not create the frame <style>: {e:?}"))?;
            style
                .set_attribute("id", "nested-scene-frame-rules")
                .map_err(|e| format!("could not id the frame <style>: {e:?}"))?;
            let head = document
                .query_selector("head")
                .map_err(|e| format!("could not find <head>: {e:?}"))?
                .ok_or("the page has no <head>")?;
            head.append_with_node_1(&style)
                .map_err(|e| format!("could not add the frame <style>: {e:?}"))?;
            style
        },
    };
    style.set_text_content(Some(&css));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Narrows `id`'s own `<svg>` to `content_right` — the rightmost edge of everything drawn — plus the left margin and
/// room for the East toolbar, then re-lays out `scene`'s own title and toolbar against the new size (the scene cannot
/// observe a resize itself).
///
/// `content_bottom` is the lowest edge of everything drawn. The height fits that edge, plus the same clear space
/// `selection::fit_canvas_to_toolbar` leaves if `step_toolbar` says a stepping toolbar is added below it. A frame with
/// no spare height scales down less when `frame_nested_scene` fits it to the stage, so its text stays closer to its
/// parent's own size.
///
/// # Errors
///
/// Returns `Err` if `id` is missing, or a DOM write or layout pass fails.
pub(crate) fn fit_nested_size(
    scene: &svg_dom_graph::scene::Scene,
    id: &str,
    content_right: f64,
    content_bottom: f64,
    step_toolbar: bool,
) -> Result<(), String> {
    const MARGIN: f64 = 20.0;
    const TOOLBAR_ALLOWANCE: f64 = 60.0;
    const STEP_TOOLBAR_GAP: f64 = 20.0;
    let document = document()?;
    let toolbar_room = if step_toolbar {
        let toolbar = svg_dom_graph::scene::SelectionToolbarOptions::default();
        STEP_TOOLBAR_GAP + toolbar.margin + toolbar.button_height
    } else {
        0.0
    };
    let height = content_bottom + toolbar_room + MARGIN;
    resize_svg(&document, id, Size::new(content_right + MARGIN + TOOLBAR_ALLOWANCE, height))?;
    scene.refresh_layout().map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets each `(name, value)` custom property in `element`'s own inline `style` attribute, replacing a previous value of
/// the same name and keeping every other declaration. Written as plain text because this crate does not enable
/// `web-sys`'s own `CssStyleDeclaration` feature.
fn set_style_vars(element: &web_sys::Element, vars: &[(&str, String)]) -> Result<(), String> {
    let existing = element.get_attribute("style").unwrap_or_default();
    let mut declarations: Vec<String> = existing
        .split(';')
        .map(str::trim)
        .filter(|d| {
            !d.is_empty()
                && !vars
                    .iter()
                    .any(|(name, _)| d.split(':').next().is_some_and(|n| n.trim() == *name))
        })
        .map(str::to_string)
        .collect();
    declarations.extend(vars.iter().map(|(name, value)| format!("{name}: {value}")));
    element
        .set_attribute("style", &declarations.join("; "))
        .map_err(|e| format!("could not set an inline style: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Clones `previous_id`'s own `<svg>` — shallow, attributes only, no content — gives the clone `next_id`, and inserts
/// it as `previous_id`'s own next sibling. Inherits size, `viewBox`, and `class` (`"nested-scene"`, plus whichever
/// further class — e.g. `theta-thetad-child`, `theta-xorloop-child` — that particular nested child's own CSS sizing
/// rule needs) from whichever element is currently in the DOM, rather than a second, hardcoded copy of them.
///
/// # Errors
///
/// Returns `Err` if `previous_id` names no element currently in the DOM, or if cloning or inserting the fresh element
/// fails.
pub(crate) fn create_child_svg(document: &web_sys::Document, previous_id: &str, next_id: &str) -> Result<(), String> {
    let previous = required_element(document, previous_id)?;
    let fresh = previous
        .clone_node_with_deep(false)
        .map_err(|e| format!("could not clone #{previous_id} for its own next step: {e:?}"))?;
    let fresh: web_sys::Element = fresh
        .dyn_into()
        .map_err(|_| "cloning the nested child <svg> did not produce an Element".to_string())?;
    fresh
        .set_attribute("id", next_id)
        .map_err(|e| format!("could not id the fresh nested child <svg>: {e:?}"))?;
    previous
        .after_with_node_1(&fresh)
        .map_err(|e| format!("could not insert the fresh nested child <svg>: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a click-through, decorative duplicate of `svg_id`'s own current content, sitting behind the nested Scene's own
/// frame — shared by every demo that nests a `Scene` the way `theta`/`sha3_sponge::keccak` do.
///
/// `Scene::enter` hides the real `svg_id` entirely while a nested child is shown — that is the library's own,
/// deliberate "exactly one Scene visible at a time" invariant (see `svg_dom_graph::scene::navigation`'s own module doc
/// comment), not a bug to work around, and a demo nesting a `Scene` this way has no reason to want the real parent
/// interactive while a child has focus. But a modal window's own look wants the parent's content still visible in the
/// margin around a smaller nested view — so this clones what is currently on screen, as a plain DOM duplicate that the
/// library's own visibility toggling knows nothing about and never touches.
///
/// Safe to call more than once for the same `svg_id` — e.g. every time a demo whose own `build_scene` rebuilds from
/// scratch on every step (`sha3_sponge::build_scene`, `sha3_sponge::keccak::build_scene`) calls this again for the same
/// host. Each call first removes whichever backdrop clone *this function* previously left behind for `svg_id` (tagged
/// via `data-backdrop-for`), before inserting a fresh one. Without that removal, every rebuild would leave its own
/// clone permanently in the DOM — each one a plain, untoggled sibling with no `id` of its own for `svg_id`'s
/// `set_inner_html("")` to ever clear — stacking up, and the most recently added one, painting last, would permanently
/// obscure `svg_id`'s own real content from then on, regardless of its own `visibility`.
///
/// Correct even when the clone is of `svg_id`'s content *at a point after some of its own state has changed*, unlike a
/// true one-time clone: taking a fresh clone on every rebuild keeps it in sync with whatever `svg_id` currently shows,
/// rather than freezing it at its first-ever content the way a single clone, never retaken, would.
///
/// `.nested-scene-backdrop`'s own `pointer-events: none` (see `style.css`) is what makes the clone a pure visual
/// backdrop: every click, drag, and wheel event passes straight through it to the real, interactive `svg_id`
/// underneath, exactly as if the clone were not there at all. `aria-hidden="true"` excludes the whole cloned subtree
/// from the accessibility tree, and every `tabindex` inside it is stripped so a sighted keyboard user tabbing through
/// the page cannot land on one of these non-functional duplicates either — a click or keypress on one would already do
/// nothing even without that, since `cloneNode` never copies event listeners, but it would still *look* clickable
/// without this. Its own `id` is stripped too, since naming two elements the same id at once would make
/// `getElementById` calls elsewhere ambiguous.
///
/// An `inert` attribute was tried here first, and rejected: it does stop the clone's own descendants from being focused
/// or announced to assistive technology, but it does **not** make the element transparent to pointer events the way
/// `pointer-events: none` does — a click still lands on an inert element and stops there. With the backdrop sitting on
/// top of the real parent in paint order, that silently swallowed every click, drag, and wheel event the parent's own
/// pan/zoom/`make_enterable` listeners needed to see, breaking all three at once.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `svg_id`, or if cloning, adjusting, or inserting the duplicate fails.
pub(crate) fn add_backdrop_clone(document: &web_sys::Document, svg_id: &str) -> Result<(), String> {
    // Removes whichever backdrop clone a previous call of this function already left behind for `svg_id` — see this
    // function's own doc comment for why a leftover one would otherwise go on obscuring real content forever.
    if let Ok(Some(stale)) = document.query_selector(&format!("[data-backdrop-for={svg_id:?}]")) {
        stale.remove();
    }

    let parent_element = required_element(document, svg_id)?;
    let backdrop = parent_element
        .clone_node_with_deep(true)
        .map_err(|e| format!("could not clone #{svg_id} for its backdrop: {e:?}"))?;
    let backdrop: web_sys::Element = backdrop
        .dyn_into()
        .map_err(|_| format!("cloning #{svg_id} did not produce an Element"))?;

    backdrop
        .remove_attribute("id")
        .map_err(|e| format!("could not strip the backdrop clone's own id: {e:?}"))?;
    backdrop
        .set_attribute("class", "nested-scene-backdrop")
        .map_err(|e| format!("could not class the backdrop clone: {e:?}"))?;
    backdrop
        .set_attribute("data-backdrop-for", svg_id)
        .map_err(|e| format!("could not tag the backdrop clone with its own host: {e:?}"))?;
    backdrop
        .set_attribute("aria-hidden", "true")
        .map_err(|e| format!("could not hide the backdrop clone from assistive tech: {e:?}"))?;

    let tabbable = backdrop
        .query_selector_all("[tabindex]")
        .map_err(|e| format!("could not search the backdrop clone for tabbable elements: {e:?}"))?;
    for i in 0..tabbable.length() {
        if let Some(node) = tabbable.item(i) {
            if let Ok(element) = node.dyn_into::<web_sys::Element>() {
                let _ = element.remove_attribute("tabindex");
            }
        }
    }

    // Placed as `svg_id`'s own next sibling: after it (so it paints over the real parent, harmless — the two are
    // pixel-identical at this point) and before whichever nested child `<svg>` comes next in document order (so the
    // nested Scene's own frame still paints on top of the backdrop once shown).
    parent_element
        .after_with_node_1(&backdrop)
        .map_err(|e| format!("could not insert the backdrop clone: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts any displayable error into this crate's own `String` error type.
///
/// Lets a library `Error` and a demo-only DOM failure share one `Result` and one `?`, throughout this crate.
pub(crate) fn stringify<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The current page's `Document`, or `Err` if there is no global `window` or no `document` on it.
///
/// Neither should be possible in a real browser. Returned as a graceful `Err` anyway, not a panic. This crate already
/// has clean ways to report a failure — `init_panel`'s own returned `Result`, or `report_panel_error`'s visible banner
/// for one inside a demo's own build function.
pub(crate) fn document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .ok_or_else(|| "no global window".to_owned())?
        .document()
        .ok_or_else(|| "no document on window".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document`, or `Err` if `index.html` does not define it.
pub(crate) fn required_element(document: &web_sys::Document, id: &str) -> Result<web_sys::Element, String> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| format!("index.html must define #{id}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Looks up `id` in `document` as an `<input>` element.
///
/// `Err` if `index.html` does not define `#id`, or defines it as something other than `<input>`.
pub(crate) fn required_input(document: &web_sys::Document, id: &str) -> Result<HtmlInputElement, String> {
    required_element(document, id)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("#{id} must be an <input>"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `selector` against `document`, or `Err` if nothing matches.
pub(crate) fn required_query(document: &web_sys::Document, selector: &str) -> Result<web_sys::Element, String> {
    document
        .query_selector(selector)
        .map_err(|e| format!("query_selector({selector:?}) failed: {e:?}"))?
        .ok_or_else(|| format!("nothing in the DOM matches {selector:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads `svg`'s own `viewBox` attribute back as a [`Rect`], to bound dragging within it.
///
/// `svg-dom`'s own `SvgRoot` deliberately does not cache `viewBox` — see that crate's own doc comment on `set_view_box`
/// for why. Every demo here sets `viewBox` once, directly in `index.html`, so reading the attribute straight from the
/// DOM is simpler than caching it a second time in this crate too.
///
/// # Errors
///
/// Returns `Err` if `svg` has no `viewBox` attribute, or its value is not exactly four numbers.
pub(crate) fn view_box_rect(svg: &SvgRoot) -> Result<Rect, String> {
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
/// Sizes `#id`'s own `<svg>` to `size`, at the same 1-user-unit-per-pixel scale every canvas in this crate already
/// uses: `width`/`height` (the on-page pixel footprint) and `viewBox` (the internal coordinate system) are set
/// together, to the same numbers, rather than letting either one imply a different scale than the other.
///
/// Written directly to the DOM via `web_sys`, not through `SvgRoot`/`Scene`: by the time a panel knows a diagram's own
/// real content size — after drawing it, or after
/// [`Scene::measure_named_data_node`](svg_dom_graph::scene::Scene::measure_named_data_node) measures it — `Scene::new`
/// has already taken ownership of the `SvgRoot` that could resize it, and neither `Scene` nor `svg-dom-graph` hands
/// that access back. This is the write-side counterpart to [`view_box_rect`]'s own read-side reasoning: `viewBox` is
/// already something this crate reaches for directly on the DOM, because `SvgRoot` deliberately does not cache it
/// either way.
///
/// Safe to call after a `Scene` is already showing content: `svg-dom-graph`'s own internal toolbar/zoom layout
/// (`visible_area`) reads `viewBox` fresh from the DOM on every layout pass, not from any cache of its own, so a plain
/// attribute write here is picked up immediately — nothing is left stale for a panel with no drag bounds to desync (a
/// panel that *does* bound dragging to its own viewBox, via `DragOptions::bounds`/[`view_box_rect`], would need to
/// recompute those bounds after calling this, since `view_box_rect` itself reads the same attribute fresh each time
/// rather than caching it).
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#id`, or a DOM write fails.
pub(crate) fn resize_svg(document: &web_sys::Document, id: &str, size: Size) -> Result<(), String> {
    let svg = required_element(document, id)?;
    let (width, height) = (size.width, size.height);
    svg.set_attribute("width", &width.to_string())
        .map_err(|e| format!("could not set #{id}'s own width: {e:?}"))?;
    svg.set_attribute("height", &height.to_string())
        .map_err(|e| format!("could not set #{id}'s own height: {e:?}"))?;
    svg.set_attribute("viewBox", &format!("0 0 {width} {height}"))
        .map_err(|e| format!("could not set #{id}'s own viewBox: {e:?}"))?;
    frame_nested_scene(document, id)
}
