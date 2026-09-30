//! `panel-theta` / `#theta-diagram`: SHA3's `Theta` function, as a parent `Scene` with a nested child `Scene` —
//! see [`build_theta_demo`]'s own doc comment for what each node represents and what is, and is not, built yet.

use crate::{
    selection::{INITIAL_5X5_BUFFER, THETA_C_INPUT},
    util::{required_element, stringify},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    NodeId,
    scene::{
        ConnectorOptions, DataFormat, DataNodeContent, NodeValues, Scene, Selection, SelectionToolbarOptions, Side,
        ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("theta.rs");

thread_local! {
    // The parent Scene, the nested ThetaC child Scene, and the container NodeId that owns it — kept alive for the
    // page's own lifetime, the same reasoning every other demo's own `SCENE` thread_local already follows.
    // `enter`/`exit` are driven from this same trio: `parent.enter(thetac_node)` to descend, `child.exit()` to
    // return, with no further state to track — `Scene::is_focused` already answers "which one is active right
    // now" without this module keeping a duplicate copy of it. The child itself is replaced wholesale on every
    // step — see [`rebuild_thetac_child`]'s own doc comment.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };

    // A fresh numeric suffix for every step's own ThetaC child `<svg>` id — see [`next_child_svg_id`]'s own doc
    // comment.
    static NEXT_CHILD_SVG_SUFFIX: Cell<u32> = const { Cell::new(0) };
}

/// Live state the nested ThetaC child's own selection toolbar shares across steps — the same shape as
/// `crate::selection::ThetaCDemo`, since this is the same demo, stepped from inside its own nested view instead of
/// standalone.
struct ThetaCState {
    /// Row `i`'s own `ThetaC` result — see `crate::selection::theta_c_outputs`'s own doc comment.
    outputs: [u64; 5],
    /// `written[i]` is `true` once row `i` has been stepped into going forward, and not since stepped away from
    /// going backward — see `crate::selection::ThetaCDemo::written`'s own doc comment.
    written: [bool; 5],
    /// The id of whichever `<svg>` currently backs the nested child — see [`rebuild_thetac_child`]'s own doc
    /// comment for why every step needs a fresh one.
    child_svg_id: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested-Scene demo: SHA3's own `Theta` function, drawn top to bottom — `A` (the real 25-value input
/// array), `ThetaC`, then `ThetaD` and a clone of `A` side by side in one row, both feeding `XOR loop`, which feeds
/// `ThetaC Output` — with `ThetaC` a genuine container node owning its own nested `Scene`. Click `ThetaC` itself to
/// drill into it — see [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable) — and the &times; in
/// its own rounded frame's corner to come back, the way a modal window's own close button would.
///
/// # What this demonstrates
///
/// `ThetaC`'s own nested `Scene` is [`selection::build_theta_c_scene`](crate::selection::build_theta_c_scene) —
/// the exact same function the standalone Cell Selection demo's own third example already draws with, called here
/// against a second, initially hidden `<svg>` (`#theta-thetac-child`) instead of
/// `#selection-thetac-diagram`. Grafting it under `A`'s own container node with `add_container_node` is the whole
/// of what makes this a nested-Scene demo: everything else — the box, the operator chain, the toolbar it shows —
/// is drawn by code that has no idea it is being nested at all. That is the point: nesting is a property of how a
/// `Scene` is *used*, not something a `Scene` has to be built differently to support.
///
/// # Stepping the nested view
///
/// `ThetaC`'s own nested `Scene` carries a [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar)
/// bar too, bound the same way the standalone demo's own third example is — see
/// `crate::selection::rebuild_theta_c_diagram`'s own doc comment. `build_theta_c_scene` has no way to redraw an
/// already-drawn chain in place, so every step needs a genuinely fresh `Scene`. [`rebuild_thetac_child`] grafts the
/// fresh one in via `Scene::replace_container_child`, exiting back to `parent` first — that call's own precondition
/// — and re-entering the fresh child immediately after, so stepping never visibly leaves the nested view.
///
/// # What is, and is not, built yet
///
/// SHA3's real `Theta` function is `C(x) = A(x,0) ⊕ A(x,1) ⊕ A(x,2) ⊕ A(x,3) ⊕ A(x,4)` (`ThetaC`, already fully
/// built — see above), `D(x) = C(x-1) ⊕ rotl(C(x+1), 1)` (`ThetaD`), and finally `A'(x,y) = A(x,y) ⊕ D(x)` for
/// every cell (`XOR loop`, feeding `ThetaC Output`) — the real step needs both `A` and `D` as its own two inputs,
/// which is why `A`'s own clone sits alongside `ThetaD` feeding `XOR loop` directly, not only through `ThetaC`.
/// Only `ThetaC` is a real, working nested `Scene` here. `ThetaD`, `XOR loop`, and `ThetaC Output` are plain
/// placeholder boxes, not container nodes — there is nothing behind them yet to nest. `A` itself is drawn as the
/// real `[5; [5; u64]]` array `ThetaC`'s own nested view already shows in full — the same content, twice over.
///
/// Extending either placeholder into its own nested `Scene`, the same way `ThetaC` already is, is exactly the kind
/// of "a further piece nested one level deeper" case this crate's nested-Scene architecture was designed to keep
/// cheap: neither would need any change to `A`, `ThetaC`, or the container mechanism itself.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing any element this function or [`wire_theta_controls`] needs, or if any
/// library call fails.
pub(crate) fn build_theta_demo() -> Result<(), String> {
    let document = crate::util::document()?;

    let parent_svg = svg_dom::SvgRoot::attach("theta-diagram").map_err(stringify)?;
    let parent = Scene::new(parent_svg).map_err(stringify)?;

    // The same starting state (row 0 already stepped, rows 1-4 still blank) the standalone Cell Selection demo
    // opens on — see `crate::selection::ThetaCDemo::written`'s own doc comment.
    let outputs = crate::selection::theta_c_outputs();
    let mut written = [false; 5];
    written[0] = true;
    let display = crate::selection::display_outputs(outputs, written);
    let (child, output) = crate::selection::build_theta_c_scene("theta-thetac-child", Some(0), display)?;

    let state = Rc::new(RefCell::new(ThetaCState {
        outputs,
        written,
        child_svg_id: "theta-thetac-child".to_string(),
    }));
    attach_thetac_toolbar(&child, output, Some(0), state)?;

    let a_content = || {
        DataNodeContent::new(
            NodeValues::U64(THETA_C_INPUT.iter().flatten().copied().collect()),
            DataFormat::Hexadecimal,
        )
    };

    let array_left_margin = 150.0;
    let row_height = 237.0;
    let row_gap = 75.0;
    let fn_width = 90.0;
    let fn_height = 70.0;
    let fn_size = Size::new(fn_width, fn_height);

    let mut row_top = 40.0;

    let a = parent
        .add_named_data_node(Point::new(array_left_margin, row_top), "A Bytes", a_content())
        .map_err(stringify)?;
    let a_rect = parent.node_rect(a).map_err(stringify)?;
    row_top += row_gap + row_height;

    let theta_c = parent
        .add_container_node(Point::new(20.0, row_top), fn_size, "ThetaC", child.clone())
        .map_err(stringify)?;
    parent.make_enterable(theta_c).map_err(stringify)?;

    let a_dup = parent
        .add_named_data_node(Point::new(array_left_margin, row_top), "A Bytes", a_content())
        .map_err(stringify)?;

    let theta_d = parent
        .add_node(Point::new(20.0, row_top + a_rect.size.height - fn_height), fn_size, "ThetaD")
        .map_err(stringify)?;
    row_top += (row_gap / 2.0) + row_height;

    let xor_loop = parent
        .add_node(
            Point::new(a_rect.origin.x + (a_rect.size.width / 2.0) - 45.0, row_top),
            fn_size,
            "XOR loop",
        )
        .map_err(stringify)?;
    row_top += (row_gap / 2.0) + fn_height;

    let theta_out = parent
        .add_named_data_node(
            Point::new(150.0, row_top),
            "Theta Output",
            DataNodeContent::new(
                NodeValues::U64(INITIAL_5X5_BUFFER.iter().flatten().copied().collect()),
                DataFormat::Hexadecimal,
            ),
        )
        .map_err(stringify)?;

    // Forced to `ThetaC`'s own North side: left to the automatic ray-cast, `A`'s own centre — pulled far to the
    // right by its own wide 5x5 grid — would otherwise cross `ThetaC`'s East side first. See
    // `svg_dom_graph::scene::ConnectorOptions::with_to_side`'s own doc comment.
    parent
        .add_edge_with(a, theta_c, ConnectorOptions::default().with_to_side(Some(Side::North)))
        .map_err(stringify)?;
    parent.add_edge(theta_c, theta_d).map_err(stringify)?;
    // Forced to leave `ThetaD`'s own South side and enter `XOR loop`'s own West side — `XOR loop` sits well to the
    // right of, and below, `ThetaD`, so the automatic ray-cast would otherwise cross a different pair of sides.
    parent
        .add_edge_with(
            theta_d,
            xor_loop,
            ConnectorOptions::default()
                .with_from_side(Some(Side::South))
                .with_to_side(Some(Side::West)),
        )
        .map_err(stringify)?;
    parent.add_edge(a_dup, xor_loop).map_err(stringify)?;
    parent.add_edge(xor_loop, theta_out).map_err(stringify)?;

    parent.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    add_backdrop_clone(&document)?;

    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, theta_c)));

    wire_theta_controls(document)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `child`, bound to `output`, driving the nested walk's *next* step — the nested
/// counterpart to `crate::selection::rebuild_theta_c_diagram`'s own toolbar. Reapplies `Selection::Cell(n)`
/// afterward for the same reason that function's own doc comment gives: `show_selection_toolbar` always resets to
/// unstarted first.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
fn attach_thetac_toolbar(
    child: &Scene,
    output: NodeId,
    n: Option<usize>,
    state: Rc<RefCell<ThetaCState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(output, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            step_thetac(&state, transition.to);
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child.set_selection(output, Selection::Cell(n)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A fresh, never-reused id for this step's own ThetaC child `<svg>` — see [`rebuild_thetac_child`]'s own doc
/// comment for why every step needs one.
fn next_child_svg_id() -> String {
    NEXT_CHILD_SVG_SUFFIX.with(|counter| {
        let n = counter.get();
        counter.set(n + 1);
        format!("theta-thetac-child-{n}")
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Clones `previous_id`'s own `<svg>` — shallow, attributes only, no content — gives the clone `next_id`, and
/// inserts it as `previous_id`'s own next sibling. Inherits size, `viewBox`, and `class="nested-scene"` from
/// whichever element is currently in the DOM, rather than a second, hardcoded copy of them.
///
/// # Errors
///
/// Returns `Err` if `previous_id` names no element currently in the DOM, or if cloning or inserting the fresh
/// element fails.
fn create_child_svg(document: &web_sys::Document, previous_id: &str, next_id: &str) -> Result<(), String> {
    let previous = required_element(document, previous_id)?;
    let fresh = previous
        .clone_node_with_deep(false)
        .map_err(|e| format!("could not clone #{previous_id} for its own next step: {e:?}"))?;
    let fresh: web_sys::Element = fresh
        .dyn_into()
        .map_err(|_| "cloning the ThetaC child <svg> did not produce an Element".to_string())?;
    fresh
        .set_attribute("id", next_id)
        .map_err(|e| format!("could not id the fresh ThetaC child <svg>: {e:?}"))?;
    previous
        .after_with_node_1(&fresh)
        .map_err(|e| format!("could not insert the fresh ThetaC child <svg>: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the nested ThetaC child for `to`, and grafts it into [`SCENE`]'s own `parent` in place of whichever
/// child is currently shown — the nested counterpart to `crate::selection::rebuild_theta_c_diagram`.
/// `crate::selection::build_theta_c_scene` has no way to update an already-drawn chain in place (see its own doc
/// comment), so each step needs a genuinely fresh `Scene`; grafting it in is exactly what
/// `Scene::replace_container_child` is for.
///
/// A fresh `Scene` needs a fresh `<svg>` of its own too: reusing the element the outgoing child already drew into
/// would leave two `Scene`s — the one about to be replaced, and this step's own new one — both quietly backed by
/// the identical DOM node while `replace_container_child` runs. [`create_child_svg`] avoids that by cloning a
/// genuinely new, empty sibling for every step; the element the outgoing child used is removed once this step has
/// fully succeeded, and the outgoing `Scene` itself is simply dropped along with it.
///
/// Exits the nested view back to `parent` first, since `replace_container_child` requires `self` — here, `parent`
/// — to be the tree's own currently focused `Scene`, then re-enters the freshly grafted child immediately
/// afterward — so from the caller's own perspective, stepping never actually leaves the nested view at all.
///
/// A fresh `Scene` would otherwise also reset zoom/pan back to `1.0`/`(0, 0)` — jarring, mid-walk, if the outgoing
/// child's own view had been zoomed or panned in first. So the outgoing child's own
/// [`Scene::view`](svg_dom_graph::scene::Scene::view) is read before it is replaced, and carried over onto the
/// fresh one via [`Scene::set_view`](svg_dom_graph::scene::Scene::set_view) — the nested counterpart to
/// `crate::selection::rebuild_theta_c_diagram`'s own same fix.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the
/// DOM, or if any library call fails.
fn rebuild_thetac_child(to: Option<usize>, display: [u64; 5], state: Rc<RefCell<ThetaCState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let previous_id = state.borrow().child_svg_id.clone();
    let next_id = next_child_svg_id();
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, output) = crate::selection::build_theta_c_scene(&next_id, to, display)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_thetac_toolbar(&new_child, output, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, thetac_node) = slot.take().ok_or("the Theta scene was not initialised")?;
        old_child.exit().map_err(stringify)?;
        parent
            .replace_container_child(thetac_node, new_child.clone())
            .map_err(stringify)?;
        parent.enter(thetac_node).map_err(stringify)?;
        *slot = Some((parent, new_child, thetac_node));
        Ok(())
    })?;

    required_element(&document, &previous_id)?.remove();
    state.borrow_mut().child_svg_id = next_id;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar)'s own `on_step` callback
/// for the nested view — the nested counterpart to `crate::selection::step_theta_c`. Updates `state`'s own
/// `written` flags for the walk's new position `to`, then rebuilds the nested child for it via
/// [`rebuild_thetac_child`].
///
/// A [`rebuild_thetac_child`] failure here is ignored, the same "cannot fail in practice, and nowhere to report it
/// to" reasoning `crate::selection::step_theta_c` already follows.
fn step_thetac(state: &Rc<RefCell<ThetaCState>>, to: Option<usize>) {
    let mut demo = state.borrow_mut();
    match to {
        None => demo.written = [false; 5],
        Some(n) => demo.written[n] = true,
    }
    let display = crate::selection::display_outputs(demo.outputs, demo.written);
    drop(demo);
    let _ = rebuild_thetac_child(to, display, state.clone());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a one-time, click-through, decorative duplicate of `#theta-diagram`, sitting behind the nested Scene's
/// own frame.
///
/// `Scene::enter` hides the real `#theta-diagram` entirely while `ThetaC` is shown — that is the library's own,
/// deliberate "exactly one Scene visible at a time" invariant (see `svg_dom_graph::scene::navigation`'s own module
/// doc comment), not a bug to work around, and this demo has no reason to want the real parent interactive while
/// `ThetaC` has focus. But a modal window's own look wants the parent's content still visible in the margin
/// around a smaller nested view — so this clones what is currently on screen, once, as a plain DOM duplicate that
/// the library's own visibility toggling knows nothing about and never touches.
///
/// A one-time clone is only correct because `#theta-diagram`'s own content never changes after this point in this
/// particular demo — nothing here redraws `A`, `ThetaC`, `ThetaD`, `XOR loop`, or `ThetaC Output` once built. A
/// demo whose parent diagram *does* change over time would need to keep this clone in sync, or take a fresh one on
/// each change, instead.
///
/// `.nested-scene-backdrop`'s own `pointer-events: none` (see `style.css`) is what makes the clone a pure visual
/// backdrop: every click, drag, and wheel event passes straight through it to the real, interactive
/// `#theta-diagram` underneath, exactly as if the clone were not there at all. `aria-hidden="true"` excludes the
/// whole cloned subtree from the accessibility tree, and every `tabindex` inside it is stripped so a sighted
/// keyboard user tabbing through the page cannot land on one of these non-functional duplicates either — a click
/// or keypress on one would already do nothing even without that, since `cloneNode` never copies event listeners,
/// but it would still *look* clickable without this. Its own `id` is stripped too, since `#theta-diagram` naming
/// two elements at once would make `getElementById` calls elsewhere ambiguous.
///
/// An `inert` attribute was tried here first, and rejected: it does stop the clone's own descendants from being
/// focused or announced to assistive technology, but it does **not** make the element transparent to pointer
/// events the way `pointer-events: none` does — a click still lands on an inert element and stops there. With the
/// backdrop sitting on top of the real parent in paint order, that silently swallowed every click, drag, and wheel
/// event the parent's own pan/zoom/`make_enterable` listeners needed to see, breaking all three at once.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-diagram`, or if cloning, adjusting, or inserting the
/// duplicate fails.
fn add_backdrop_clone(document: &web_sys::Document) -> Result<(), String> {
    let parent_element = required_element(document, "theta-diagram")?;
    let backdrop = parent_element
        .clone_node_with_deep(true)
        .map_err(|e| format!("could not clone #theta-diagram for its backdrop: {e:?}"))?;
    let backdrop: web_sys::Element = backdrop
        .dyn_into()
        .map_err(|_| "cloning #theta-diagram did not produce an Element".to_string())?;

    backdrop
        .remove_attribute("id")
        .map_err(|e| format!("could not strip the backdrop clone's own id: {e:?}"))?;
    backdrop
        .set_attribute("class", "nested-scene-backdrop")
        .map_err(|e| format!("could not class the backdrop clone: {e:?}"))?;
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

    // Placed as `#theta-diagram`'s own next sibling: after it (so it paints over the real parent, harmless — the
    // two are pixel-identical at this point) and before `#theta-thetac-child` in document order (so the nested
    // Scene's own frame still paints on top of the backdrop once shown).
    parent_element
        .after_with_node_1(&backdrop)
        .map_err(|e| format!("could not insert the backdrop clone: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#theta-close` — the nested view's own &times; close button, not an external "Exit" — to [`SCENE`].
/// Entering `ThetaC` needs no wiring of its own here either: clicking the node itself is
/// [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable)'s own doing, installed once in
/// [`build_theta_demo`] when `thetac` is added.
///
/// `#theta-close` is only ever visible while `ThetaC` actually is — see `.nested-scene-close`'s own `:has()` rule
/// in `style.css` — but the listener still simply attempts `child.exit()` and ignores its `Result`, the same
/// "nowhere to report an error to" reasoning every other button-click listener in this crate already follows.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-close`, or if a listener could not be attached to it.
fn wire_theta_controls(document: web_sys::Document) -> Result<(), String> {
    let close = required_element(&document, "theta-close")?;

    let close_closure = Closure::<dyn FnMut()>::new(move || {
        SCENE.with_borrow(|slot| {
            if let Some((_, child, _)) = slot {
                let _ = child.exit();
            }
        });
    });
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the Theta close-button listener: {e:?}"))?;
    close_closure.forget();

    Ok(())
}
