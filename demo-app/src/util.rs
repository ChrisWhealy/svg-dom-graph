//! Small DOM/error helpers shared by more than one demo module.

use std::cell::Cell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

thread_local! {
    // A fresh numeric suffix for every step's own nested child `<svg>` id, shared by every demo module that steps
    // a nested child this way (`theta`'s own `theta_c`/`theta_d`/`xor_loop`, and `sha3_sponge::keccak`) — see
    // [`next_child_svg_id`]'s own doc comment.
    static NEXT_CHILD_SVG_SUFFIX: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A fresh, never-reused id for this step's own nested child `<svg>`, prefixed with `prefix`. Shares one counter
/// across every demo module that calls this, so every id handed out is unique regardless of which nested child
/// it backs.
pub(crate) fn next_child_svg_id(prefix: &str) -> String {
    NEXT_CHILD_SVG_SUFFIX.with(|counter| {
        let n = counter.get();
        counter.set(n + 1);
        format!("{prefix}-{n}")
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Clones `previous_id`'s own `<svg>` — shallow, attributes only, no content — gives the clone `next_id`, and
/// inserts it as `previous_id`'s own next sibling. Inherits size, `viewBox`, and `class` (`"nested-scene"`, plus
/// whichever further class — e.g. `theta-thetad-child`, `theta-xorloop-child` — that particular nested child's
/// own CSS sizing rule needs) from whichever element is currently in the DOM, rather than a second, hardcoded
/// copy of them.
///
/// # Errors
///
/// Returns `Err` if `previous_id` names no element currently in the DOM, or if cloning or inserting the fresh
/// element fails.
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
/// Adds a click-through, decorative duplicate of `svg_id`'s own current content, sitting behind the nested
/// Scene's own frame — shared by every demo that nests a `Scene` the way `theta`/`sha3_sponge::keccak` do.
///
/// `Scene::enter` hides the real `svg_id` entirely while a nested child is shown — that is the library's own,
/// deliberate "exactly one Scene visible at a time" invariant (see `svg_dom_graph::scene::navigation`'s own module
/// doc comment), not a bug to work around, and a demo nesting a `Scene` this way has no reason to want the real
/// parent interactive while a child has focus. But a modal window's own look wants the parent's content still
/// visible in the margin around a smaller nested view — so this clones what is currently on screen, as a plain
/// DOM duplicate that the library's own visibility toggling knows nothing about and never touches.
///
/// Safe to call more than once for the same `svg_id` — e.g. every time a demo whose own `build_scene` rebuilds
/// from scratch on every step (`sha3_sponge::build_scene`, `sha3_sponge::keccak::build_scene`) calls this again
/// for the same host. Each call first removes whichever backdrop clone *this function* previously left behind for
/// `svg_id` (tagged via `data-backdrop-for`), before inserting a fresh one. Without that removal, every rebuild
/// would leave its own clone permanently in the DOM — each one a plain, untoggled sibling with no `id` of its own
/// for `svg_id`'s `set_inner_html("")` to ever clear — stacking up, and the most recently added one, painting
/// last, would permanently obscure `svg_id`'s own real content from then on, regardless of its own `visibility`.
///
/// Correct even when the clone is of `svg_id`'s content *at a point after some of its own state has changed*,
/// unlike a true one-time clone: taking a fresh clone on every rebuild keeps it in sync with whatever `svg_id`
/// currently shows, rather than freezing it at its first-ever content the way a single clone, never retaken,
/// would.
///
/// `.nested-scene-backdrop`'s own `pointer-events: none` (see `style.css`) is what makes the clone a pure visual
/// backdrop: every click, drag, and wheel event passes straight through it to the real, interactive `svg_id`
/// underneath, exactly as if the clone were not there at all. `aria-hidden="true"` excludes the whole cloned
/// subtree from the accessibility tree, and every `tabindex` inside it is stripped so a sighted keyboard user
/// tabbing through the page cannot land on one of these non-functional duplicates either — a click or keypress on
/// one would already do nothing even without that, since `cloneNode` never copies event listeners, but it would
/// still *look* clickable without this. Its own `id` is stripped too, since naming two elements the same id at
/// once would make `getElementById` calls elsewhere ambiguous.
///
/// An `inert` attribute was tried here first, and rejected: it does stop the clone's own descendants from being
/// focused or announced to assistive technology, but it does **not** make the element transparent to pointer
/// events the way `pointer-events: none` does — a click still lands on an inert element and stops there. With the
/// backdrop sitting on top of the real parent in paint order, that silently swallowed every click, drag, and wheel
/// event the parent's own pan/zoom/`make_enterable` listeners needed to see, breaking all three at once.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `svg_id`, or if cloning, adjusting, or inserting the duplicate fails.
pub(crate) fn add_backdrop_clone(document: &web_sys::Document, svg_id: &str) -> Result<(), String> {
    // Removes whichever backdrop clone a previous call of this function already left behind for `svg_id` — see
    // this function's own doc comment for why a leftover one would otherwise go on obscuring real content forever.
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
    // pixel-identical at this point) and before whichever nested child `<svg>` comes next in document order (so
    // the nested Scene's own frame still paints on top of the backdrop once shown).
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
/// Written directly to the DOM via `web_sys`, not through `SvgRoot`/`Scene`: by the time a panel knows a diagram's
/// own real content size — after drawing it, or after [`Scene::measure_named_data_node`](svg_dom_graph::scene::Scene::measure_named_data_node)
/// measures it — `Scene::new` has already taken ownership of the `SvgRoot` that could resize it, and neither
/// `Scene` nor `svg-dom-graph` hands that access back. This is the write-side counterpart to [`view_box_rect`]'s
/// own read-side reasoning: `viewBox` is already something this crate reaches for directly on the DOM, because
/// `SvgRoot` deliberately does not cache it either way.
///
/// Safe to call after a `Scene` is already showing content: `svg-dom-graph`'s own internal toolbar/zoom layout
/// (`visible_area`) reads `viewBox` fresh from the DOM on every layout pass, not from any cache of its own, so a
/// plain attribute write here is picked up immediately — nothing is left stale for a panel with no drag bounds to
/// desync (a panel that *does* bound dragging to its own viewBox, via `DragOptions::bounds`/[`view_box_rect`],
/// would need to recompute those bounds after calling this, since `view_box_rect` itself reads the same attribute
/// fresh each time rather than caching it).
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
        .map_err(|e| format!("could not set #{id}'s own viewBox: {e:?}"))
}
