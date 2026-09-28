//! `panel-theta` / `#theta-diagram`: SHA3's `Theta` function, as a parent `Scene` with a nested child `Scene` —
//! see [`build_theta_demo`]'s own doc comment for what each node represents and what is, and is not, built yet.

use crate::util::{required_element, stringify};
use std::cell::RefCell;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    NodeId,
    scene::{Scene, Side, ToolbarOptions},
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("theta.rs");

thread_local! {
    // The parent Scene, the nested ThetaC child Scene, and the container NodeId that owns it — kept alive for the
    // page's own lifetime, the same reasoning every other demo's own `SCENE` thread_local already follows.
    // `enter`/`exit` are driven from this same trio: `parent.enter(thetac_node)` to descend, `child.exit()` to
    // return, with no further state to track — `Scene::is_focused` already answers "which one is active right
    // now" without this module keeping a duplicate copy of it.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested-Scene demo: SHA3's own `Theta` function, drawn as five stages left to right — `A` (the input
/// state), `ThetaC`, `ThetaD`, the `XOR` loop that folds `D` back into `A`, and `A'` (the updated state) — with
/// `ThetaC` a genuine container node owning its own nested `Scene`. Click `ThetaC` itself to drill into it — see
/// [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable) — and the &times; in its own rounded
/// frame's corner to come back, the way a modal window's own close button would.
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
/// # What is, and is not, built yet
///
/// SHA3's real `Theta` function is `C(x) = A(x,0) ⊕ A(x,1) ⊕ A(x,2) ⊕ A(x,3) ⊕ A(x,4)` (`ThetaC`, already fully
/// built — see above), `D(x) = C(x-1) ⊕ rotl(C(x+1), 1)` (`ThetaD`), and finally `A'(x,y) = A(x,y) ⊕ D(x)` for
/// every cell (the `XOR` loop). Only `ThetaC` is a real, working nested `Scene` here. `ThetaD` and the `XOR` loop
/// are plain placeholder boxes, not container nodes — there is nothing behind them yet to nest. `A` and `A'` are
/// plain boxes too, standing in for the real `[5; [5; u64]]` state `ThetaC`'s own nested view already shows in
/// full.
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
    // opens on — see `initial_theta_c_display`'s own doc comment.
    let child =
        crate::selection::build_theta_c_scene("theta-thetac-child", 0, crate::selection::initial_theta_c_display())?;

    let a = parent
        .add_node(Point::new(20.0, 75.0), Size::new(90.0, 70.0), "A")
        .map_err(stringify)?;

    // `ThetaC` is drawn larger than its plain-box neighbours, so it visually reads as the one node here that is
    // more than a box — the same "a container looks like an ordinary node, only a little more so" choice
    // `add_container_node`'s own doc comment leaves to the caller, since the library itself draws a container
    // exactly like a plain node otherwise.
    let thetac = parent
        .add_container_node(Point::new(150.0, 65.0), Size::new(160.0, 90.0), "ThetaC", child.clone())
        .map_err(stringify)?;
    parent.make_enterable(thetac).map_err(stringify)?;

    let thetad = parent
        .add_node(Point::new(350.0, 75.0), Size::new(130.0, 70.0), "ThetaD (planned)")
        .map_err(stringify)?;
    let xor_loop = parent
        .add_node(Point::new(520.0, 75.0), Size::new(140.0, 70.0), "XOR loop (planned)")
        .map_err(stringify)?;
    let a_prime = parent
        .add_node(Point::new(700.0, 75.0), Size::new(90.0, 70.0), "A'")
        .map_err(stringify)?;

    parent.add_edge(a, thetac).map_err(stringify)?;
    parent.add_edge(thetac, thetad).map_err(stringify)?;
    parent.add_edge(thetad, xor_loop).map_err(stringify)?;
    parent.add_edge(xor_loop, a_prime).map_err(stringify)?;

    parent.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    add_backdrop_clone(&document)?;

    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, thetac)));

    wire_theta_controls(document)
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
/// particular demo — nothing here redraws `A`, `ThetaC`, `ThetaD`, the `XOR` loop, or `A'` once built. A demo whose
/// parent diagram *does* change over time would need to keep this clone in sync, or take a fresh one on each
/// change, instead.
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
