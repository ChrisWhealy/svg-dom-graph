//! `panel-theta` / `#theta-diagram`: SHA3's `Theta` function, as a parent `Scene` with three nested child `Scene`s
//! — see [`build_scene`]'s own doc comment for what each node represents and what is, and is not, built yet.
//!
//! - [`theta_c`] — the nested `ThetaC` child: its own selection toolbar, and rebuilding it on every step.
//! - [`theta_d`] — the nested `ThetaD` child: the `ROTL`/`XOR` step itself, its own selection toolbar, and
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

#[cfg(test)]
mod unit_tests;

use crate::{
    selection::{INITIAL_5X5_BUFFER, THETA_C_INPUT},
    util::{add_backdrop_clone, required_element, stringify},
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

thread_local! {
    // Keeps this module's own top-level "parent" `Scene` alive across however many times `build_scene` is called
    // — see that function's own doc comment ("Reuse across more than one host") for why this cannot simply rely
    // on `theta_c`/`theta_d`/`xor_loop`'s own thread-locals the way a single standalone call already implicitly
    // does.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

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
/// (`{svg_id}-thetac-child` — `#theta-diagram-thetac-child` for `panel-theta`'s own standalone call) instead of
/// `#selection-thetac-diagram`.
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
/// rotl(C(x+1), 1)` (`ThetaD`; built here exactly as that — `rotl` is `u64::rotate_left`, with no further
/// byte-order adjustment once a lane is a plain `u64` — see `theta_d`'s own `row` function for the exact formula),
/// and finally `A'(x,y) = A(x,y) ⊕ D(x)` for every cell (`XOR loop`) — the real step needs both
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
/// # Reuse across more than one host
///
/// `svg_id` lets this whole function be grafted as a container node's own nested child elsewhere — see
/// `sha3_sponge::keccak`'s own "Theta" node — not just drawn standalone as `panel-theta` itself.
/// `theta_c`'s/`theta_d`'s/`xor_loop`'s own initial child ids are derived from `svg_id`, so a second call keeps
/// its own three elements distinct from the first's; every id generated after that stays globally unique anyway,
/// via `support::next_child_svg_id`'s own shared counter.
///
/// `theta_c`/`theta_d`/`xor_loop` each still track their own currently-entered child in one thread-local apiece,
/// shared by every call to this function, regardless of which host built it. Two separate calls can each stay
/// correctly focused and steppable on their own. But drilling into a nested child of *both* at once confuses
/// whichever child's own close button was wired first, since the other's own thread-local write wins. Treat this
/// as a known limitation, not a reason to avoid nesting this function at all.
///
/// `with_backdrop` controls whether this call adds its own [`add_backdrop_clone`] of `svg_id`. Pass `true` only
/// when `svg_id` is the shallowest, outermost element in its own `.nested-scene-stage` — `panel-theta`'s own
/// standalone use, where `#theta-diagram` has nothing shallower above it.
///
/// Pass `false` when nesting this function under a host that is itself already nested one or more levels deep —
/// `sha3_sponge::keccak`'s own "Theta" node, under `sha3_sponge::build_scene`'s own already-backdropped
/// `#sha3-sponge-diagram`. There, `svg_id` starts `visibility="hidden"` the instant its own container node is
/// constructed, long before any of its own children are ever entered. A backdrop clone has no visibility
/// toggling of its own — it always paints. So, called unconditionally, it would immediately cover whatever
/// shallower sibling sits behind it in document order, every time this function rebuilds.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing any element this function needs, or if any library call fails.
pub(crate) fn build_scene(svg_id: &str, with_backdrop: bool) -> Result<Scene, String> {
    let document = crate::util::document()?;

    // Derived from `svg_id` itself, not a literal, so a second call — nested elsewhere, under a different
    // `svg_id` — targets its own three distinct elements instead of colliding with this one's.
    let thetac_child_id = format!("{svg_id}-thetac-child");
    let thetad_child_id = format!("{svg_id}-thetad-child");
    let xorloop_child_id = format!("{svg_id}-xorloop-child");

    crate::util::frame_nested_scene(&document, svg_id)?;
    let parent_svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let parent = Scene::new(parent_svg).map_err(stringify)?;
    parent
        .show_scene_title("Keccak Theta Function", SceneTitleOptions::default())
        .map_err(stringify)?;

    // Unstarted: no row has been processed yet — see `crate::selection::build_selection_demo`'s own doc comment
    // (point 5) for why the chain is still drawn, over five zero operands, rather than left out entirely.
    let outputs = crate::selection::theta_c_outputs();
    let display = crate::selection::display_outputs(outputs, None);
    let (child, output) = theta_c::build_scene(&thetac_child_id, None, display)?;

    let state = Rc::new(RefCell::new(SteppedChildState {
        outputs,
        child_svg_id: thetac_child_id.clone(),
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
    let theta_d_display = crate::selection::display_outputs(theta_d_outputs, None);
    let (theta_d_child, theta_d_output) = theta_d::build_scene(&thetad_child_id, None, theta_d_display)?;

    let theta_d_state = Rc::new(RefCell::new(SteppedChildState {
        outputs: theta_d_outputs,
        child_svg_id: thetad_child_id.clone(),
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
    let xor_loop_display = xor_loop::display_outputs(xor_loop_outputs, None);
    let (xor_loop_child, xor_loop_output) = xor_loop::build_scene(&xorloop_child_id, None, xor_loop_display)?;

    let xor_loop_state = Rc::new(RefCell::new(xor_loop::XorLoopState {
        outputs: xor_loop_outputs,
        child_svg_id: xorloop_child_id.clone(),
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

    let theta_out_rect = parent.node_rect(theta_out).map_err(stringify)?;
    crate::util::fit_nested_size(
        &parent,
        svg_id,
        (a_rect.origin.x + a_rect.size.width).max(theta_out_rect.origin.x + theta_out_rect.size.width),
        theta_out_rect.origin.y + theta_out_rect.size.height,
        false,
    )?;

    // Fitting `svg_id` just resized the stage whenever it is the stage's own base diagram (`panel-theta`), after each
    // child was already framed against the old, larger stage. Frame them again so none overhangs the new one.
    for child_id in [&thetac_child_id, &thetad_child_id, &xorloop_child_id] {
        crate::util::frame_nested_scene(&document, child_id)?;
    }

    parent.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    if with_backdrop {
        add_backdrop_clone(&document, svg_id)?;
    }

    theta_c::init_scene(parent.clone(), child, theta_c);
    theta_d::init_scene(parent.clone(), theta_d_child, theta_d);
    xor_loop::init_scene(parent.clone(), xor_loop_child, xor_loop_node);

    // Keeps this Scene's own strong handle alive for as long as this host needs it — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(parent.clone()));
    Ok(parent)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 25 lanes this scene's own `Theta` produces — what its nested `XOR loop` finally writes to "Theta Output",
/// flattened row-major, exactly as that node shows them once the walk has finished. `Rho` takes these as its input.
pub(crate) fn output_lanes() -> [u64; 25] {
    let d = theta_d::outputs(crate::selection::theta_c_outputs());
    let outputs = xor_loop::outputs(THETA_C_INPUT, d);
    std::array::from_fn(|lane| outputs[lane / 5][lane % 5])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `panel-theta`'s own entry point: [`build_scene`] against `#theta-diagram`, with its own close button wired.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-diagram`/`#theta-close`, or if any library call fails.
pub(crate) fn build_theta_demo() -> Result<(), String> {
    let document = crate::util::document()?;
    create_stage_svgs(&document)?;
    build_scene("theta-diagram", true)?;
    wire_theta_controls(document)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates every `<svg>` `panel-theta`'s own `.nested-scene-stage` needs, if they do not already exist: the base
/// diagram, then each nested child, shallowest first — document order is paint order. Each is inserted before
/// `#theta-close`, which therefore stays on top. These sizes are the *initial* ones only: each scene fits its own
/// `<svg>` to its content as it is built.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-close`, or if creating any `<svg>` fails.
fn create_stage_svgs(document: &web_sys::Document) -> Result<(), String> {
    const ANCHOR: &str = "theta-close";
    let levels: [(&str, Option<&str>, Size); 4] = [
        ("theta-diagram", None, Size::new(1260.0, 1090.0)),
        ("theta-diagram-thetac-child", Some("nested-scene"), Size::new(1180.0, 1010.0)),
        ("theta-diagram-thetad-child", Some("nested-scene"), Size::new(1130.0, 600.0)),
        ("theta-diagram-xorloop-child", Some("nested-scene"), Size::new(1400.0, 780.0)),
    ];
    for (id, class, size) in levels {
        crate::util::ensure_svg(document, ANCHOR, id, class, size)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits whichever of this scene's own `ThetaC`/`ThetaD`/`XOR loop` is currently focused, if any, and reports
/// whether one was. A host nesting [`build_scene`] elsewhere uses this for its own close button, the same way
/// [`wire_theta_controls`] uses it for `panel-theta`'s own.
pub(crate) fn exit_if_focused() -> bool {
    theta_c::exit_if_focused() || theta_d::exit_if_focused() || xor_loop::exit_if_focused()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#theta-close` — the nested view's own &times; close button, not an external "Exit" — to whichever of
/// `theta_c`'s, `theta_d`'s, or `xor_loop`'s own nested child is currently entered, via [`exit_if_focused`].
/// Entering any of them needs no wiring of its own here: clicking the node itself is
/// [`Scene::make_enterable`](svg_dom_graph::scene::Scene::make_enterable)'s own doing, installed once in
/// [`build_scene`] when each is added.
///
/// `#theta-close` is only ever visible while one of them actually is — see `.nested-scene-close`'s own `:has()`
/// rule in `style.css`, which matches all three — but the listener still ignores [`exit_if_focused`]'s own
/// `bool`, the same "nowhere to report an error to" reasoning every other button-click listener in this crate
/// already follows.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#theta-close`, or if a listener could not be attached to it.
fn wire_theta_controls(document: web_sys::Document) -> Result<(), String> {
    let close = required_element(&document, "theta-close")?;

    let close_closure = Closure::<dyn FnMut()>::new(move || {
        exit_if_focused();
    });
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the Theta close-button listener: {e:?}"))?;
    close_closure.forget();

    Ok(())
}
