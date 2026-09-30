//! `panel-theta` / `#theta-diagram`: SHA3's `Theta` function, as a parent `Scene` with three nested child `Scene`s
//! — see [`build_theta_demo`]'s own doc comment for what each node represents and what is, and is not, built yet.
//!
//! - [`theta_c`] — the nested `ThetaC` child: its own selection toolbar, and rebuilding it on every step.
//! - [`theta_d`] — the nested `ThetaD` child: the `ROTR`/`XOR` step itself, its own selection toolbar, and
//!   rebuilding it on every step.
//! - [`xor_loop`] — the nested `XOR loop` child: the final `A' = A ⊕ D` fold, its own selection toolbar (stepped
//!   cell by cell, not row by row — see its own module doc comment for why), and rebuilding it on every step.
//! - [`support`] — shared by `theta_c`/`theta_d`: the live state their own selection toolbars carry across steps,
//!   and cloning a fresh `<svg>` for every step. `xor_loop` reuses the cloning helpers but not the state shape —
//!   see [`xor_loop::XorLoopState`]'s own doc comment for why.

mod support;
pub(crate) mod theta_c;
mod theta_d;
mod xor_loop;

use crate::{
    selection::{INITIAL_5X5_BUFFER, THETA_C_INPUT},
    util::{required_element, stringify},
};
use support::SteppedChildState;

use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{
    ConnectorOptions, DataFormat, DataNodeContent, NodeValues, Scene, SceneTitleOptions, Side, ToolbarOptions,
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
/// Only [`build_theta_demo`] itself is ever shown from it (see that doc comment's own "SOURCE" note); `theta_c`,
/// `theta_d`, and `support` live in their own files precisely so this one stays that function alone.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested-Scene demo: SHA3's own `Theta` function, drawn top to bottom — `A` (the real 25-value input
/// array), `ThetaC`, then `ThetaD` and a clone of `A` side by side in one row, both feeding `XOR loop`, which feeds
/// `Theta Output` — with `ThetaC`, `ThetaD`, and `XOR loop` all genuine container nodes, each owning its own nested
/// `Scene`. Click any one to drill into it — see
/// [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable) — and the &times; in its own rounded
/// frame's corner to come back, the way a modal window's own close button would.
///
/// # What this demonstrates
///
/// `ThetaC`'s own nested `Scene` is [`theta_c::build_scene`] — the exact same function the standalone Cell
/// Selection demo's own third example already draws with, called here against a second, initially hidden `<svg>`
/// (`#theta-thetac-child`) instead of `#selection-thetac-diagram`.
/// `ThetaD`'s own nested `Scene` is [`theta_d::build_scene`], and `XOR loop`'s own is [`xor_loop::build_scene`].
/// Grafting any of them under its own container node with `add_container_node` is the whole of what makes this a
/// nested-Scene demo: everything else — the boxes, the operator nodes, the toolbar each shows — is drawn by code
/// that has no idea it is being nested at all. That is the point: nesting is a property of how a `Scene` is
/// *used*, not something a `Scene` has to be built differently to support.
///
/// # Stepping the nested view
///
/// All three nested children carry a
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar) bar, bound the same way
/// the standalone Cell Selection demo's own third example is — see `crate::selection::rebuild_theta_c_diagram`'s
/// own doc comment. `theta_c`'s/`theta_d`'s own bar steps through five rows; `xor_loop`'s own steps through all
/// twenty-five cells instead — see [`xor_loop`]'s own module doc comment for why. None of
/// `theta_c::build_scene`/`theta_d::build_scene`/`xor_loop::build_scene` has a way to redraw an already-drawn
/// chain in place, so every step needs a genuinely fresh `Scene`: each one's own `rebuild_child` grafts the fresh
/// one in via `Scene::replace_container_child`, exiting back to `parent` first — that call's own precondition —
/// and re-entering the fresh child immediately after, so stepping never visibly leaves the nested view.
///
/// # What is, and is not, built yet
///
/// SHA3's real `Theta` function is `C(x) = A(x,0) ⊕ A(x,1) ⊕ A(x,2) ⊕ A(x,3) ⊕ A(x,4)` (`ThetaC`), `D(x) = C(x-1) ⊕
/// rotl(C(x+1), 1)` (`ThetaD`; built here as `ROTR(next, 1) ⊕ prev`, not `rotl` — see [`theta_d`]'s own module doc
/// comment for why), and finally `A'(x,y) = A(x,y) ⊕ D(x)` for every cell (`XOR loop`) — the real step needs both
/// `A` and `D` as its own two inputs, which is why `A`'s own clone sits alongside `ThetaD` feeding `XOR loop`
/// directly, not only through `ThetaC`. `ThetaC`, `ThetaD`, and `XOR loop` are all real, working nested `Scene`s
/// now. `Theta Output`, fed by `XOR loop`, is still a plain placeholder box, not a container node — there is
/// nothing behind it yet to nest. `A` itself is drawn as the real `[5; [5; u64]]` array `ThetaC`'s/`XOR loop`'s own
/// nested views already show in full — the same content, three times over.
///
/// Extending `Theta Output` into its own nested `Scene` — folding `A'` back into the state array across all 24
/// further rounds SHA3 actually runs — is a genuinely larger feature than the further "one more step, nested the
/// same way" cases this module's own history already covers; it is not sketched out here.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing any element this function or [`wire_theta_controls`] needs, or if any
/// library call fails.
pub(crate) fn build_theta_demo() -> Result<(), String> {
    let document = crate::util::document()?;

    let parent_svg = svg_dom::SvgRoot::attach("theta-diagram").map_err(stringify)?;
    let parent = Scene::new(parent_svg).map_err(stringify)?;
    parent
        .show_scene_title("Keccak Theta Function", SceneTitleOptions::default())
        .map_err(stringify)?;

    // Unstarted: no row has been processed yet — see `crate::selection::build_selection_demo`'s own doc comment
    // (point 5) for why the chain is still drawn, over five zero operands, rather than left out entirely.
    let outputs = crate::selection::theta_c_outputs();
    let written = [false; 5];
    let display = crate::selection::display_outputs(outputs, written);
    let (child, output) = theta_c::build_scene("theta-thetac-child", None, display)?;

    let state = Rc::new(RefCell::new(SteppedChildState {
        outputs,
        written,
        child_svg_id: "theta-thetac-child".to_string(),
    }));
    theta_c::attach_toolbar(&child, output, None, state)?;

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

    let mut row_top = 50.0;

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

    // Unstarted, the same "no row processed yet" convention as `ThetaC`'s own initial state above.
    let theta_d_outputs = theta_d::outputs(crate::selection::theta_c_outputs());
    let theta_d_written = [false; 5];
    let theta_d_display = crate::selection::display_outputs(theta_d_outputs, theta_d_written);
    let (theta_d_child, theta_d_output) = theta_d::build_scene("theta-thetad-child", None, theta_d_display)?;

    let theta_d_state = Rc::new(RefCell::new(SteppedChildState {
        outputs: theta_d_outputs,
        written: theta_d_written,
        child_svg_id: "theta-thetad-child".to_string(),
    }));
    theta_d::attach_toolbar(&theta_d_child, theta_d_output, None, theta_d_state)?;

    let theta_d = parent
        .add_container_node(
            Point::new(20.0, row_top + a_rect.size.height - fn_height),
            fn_size,
            "ThetaD",
            theta_d_child.clone(),
        )
        .map_err(stringify)?;
    parent.make_enterable(theta_d).map_err(stringify)?;
    row_top += (row_gap / 2.0) + row_height;

    // Unstarted, the same "no cell processed yet" convention as `ThetaC`'s/`ThetaD`'s own initial state above —
    // except stepped cell by cell, not row by row; see `xor_loop::build_scene`'s own doc comment for why.
    let xor_loop_outputs = xor_loop::outputs(THETA_C_INPUT, theta_d_outputs);
    let xor_loop_written = [[false; 5]; 5];
    let xor_loop_display = xor_loop::display_outputs(xor_loop_outputs, xor_loop_written);
    let (xor_loop_child, xor_loop_output) = xor_loop::build_scene("theta-xorloop-child", None, xor_loop_display)?;

    let xor_loop_state = Rc::new(RefCell::new(xor_loop::XorLoopState {
        outputs: xor_loop_outputs,
        written: xor_loop_written,
        child_svg_id: "theta-xorloop-child".to_string(),
    }));
    xor_loop::attach_toolbar(&xor_loop_child, xor_loop_output, None, xor_loop_state)?;

    let xor_loop_node = parent
        .add_container_node(
            Point::new(a_rect.origin.x + (a_rect.size.width / 2.0) - 45.0, row_top),
            fn_size,
            "XOR loop",
            xor_loop_child.clone(),
        )
        .map_err(stringify)?;
    parent.make_enterable(xor_loop_node).map_err(stringify)?;
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
            xor_loop_node,
            ConnectorOptions::default()
                .with_from_side(Some(Side::South))
                .with_to_side(Some(Side::West)),
        )
        .map_err(stringify)?;
    parent.add_edge(a_dup, xor_loop_node).map_err(stringify)?;
    parent.add_edge(xor_loop_node, theta_out).map_err(stringify)?;

    parent.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    add_backdrop_clone(&document)?;

    theta_c::init_scene(parent.clone(), child, theta_c);
    theta_d::init_scene(parent.clone(), theta_d_child, theta_d);
    xor_loop::init_scene(parent, xor_loop_child, xor_loop_node);

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
/// Wires `#theta-close` — the nested view's own &times; close button, not an external "Exit" — to whichever of
/// `theta_c`'s, `theta_d`'s, or `xor_loop`'s own nested child is currently entered. Entering any of them needs no
/// wiring of its own here: clicking the node itself is
/// [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable)'s own doing, installed once in
/// [`build_theta_demo`] when each is added.
///
/// `#theta-close` is only ever visible while one of them actually is — see `.nested-scene-close`'s own `:has()`
/// rule in `style.css`, which matches all three — but the listener still checks each in turn via its own
/// `exit_if_focused`, and ignores every `Result`, the same "nowhere to report an error to" reasoning every other
/// button-click listener in this crate already follows.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-close`, or if a listener could not be attached to it.
fn wire_theta_controls(document: web_sys::Document) -> Result<(), String> {
    let close = required_element(&document, "theta-close")?;

    let close_closure = Closure::<dyn FnMut()>::new(move || {
        if theta_c::exit_if_focused() {
            return;
        }
        if theta_d::exit_if_focused() {
            return;
        }
        xor_loop::exit_if_focused();
    });
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the Theta close-button listener: {e:?}"))?;
    close_closure.forget();

    Ok(())
}
