//! `panel-selection` / `#selection-1d-diagram`, `#selection-2d-diagram`, and `#selection-thetac-diagram`: three
//! examples, each stepping through an array's values via its own in-canvas [`Scene::show_selection_toolbar`] bar —
//! no external HTML buttons. See [`build_selection_demo`]'s own doc comment for what each demonstrates.

use crate::util::{ensure_svg_in, required_element, resize_svg, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    NodeId,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection, SelectionToolbarOptions},
};

#[cfg(test)]
mod unit_tests;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("selection.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own two, separate `Scene`s — one per
    // array dimension. Kept alive for the page's own lifetime: `show_selection_toolbar`'s own button listeners hold
    // only a `Weak` reference back to the `Scene` they step — see its own doc comment ("Ownership").
    static SCENE: RefCell<Option<(Scene, Scene)>> = const { RefCell::new(None) };

    // The third example's own canvas has a `Scene` too, replaced by each step's own rebuild — see
    // [`rebuild_theta_c_diagram`]'s own doc comment for why it must be kept.
    static THETA_C_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// How many columns the two-dimensional demo array uses. Its own value count (see [`build_selection_demo`])
/// deliberately does not divide evenly by this, so the grid's own last row renders short.
const SELECTION_TWO_D_COLS: usize = 4;

pub(crate) const INITIAL_5X5_BUFFER: [[u64; 5]; 5] = [
    [0x0, 0x0, 0x0, 0x0, 0x0],
    [0x0, 0x0, 0x0, 0x0, 0x0],
    [0x0, 0x0, 0x0, 0x0, 0x0],
    [0x0, 0x0, 0x0, 0x0, 0x0],
    [0x0, 0x0, 0x0, 0x0, 0x0],
];

/// The third example's own fixed input, `A(row, col)` — made-up values, not a real Keccak state, chosen only to
/// give each row's own `ThetaC` chain a visibly distinct result. See [`build_selection_demo`]'s own doc comment
/// (point 4).
pub(crate) const THETA_C_INPUT: [[u64; 5]; 5] = [
    [
        0x1C80_317F_A3B1_799D,
        0xBDD6_40FB_0667_1AD1,
        0x3EB1_3B90_4668_5257,
        0x23B8_C1E9_3924_56DE,
        0x1A3D_1FA7_BC89_60A9,
    ],
    [
        0xBD9C_66B3_AD3C_2D6D,
        0x8B9D_2434_E465_E150,
        0x972A_8469_1641_9F82,
        0x0822_E8F3_6C03_1199,
        0x17FC_695A_07A0_CA6E,
    ],
    [
        0x3B8F_AA18_37F8_A88B,
        0x9A1D_E644_815E_F6D1,
        0x8FAD_C1A6_06CB_0FB3,
        0xB74D_0FB1_32E7_0629,
        0xB38A_088C_A65E_D389,
    ],
    [
        0x6B65_A6A4_8B81_48F6,
        0x72FF_5D2A_386E_CBE0,
        0x4737_8190_96DA_1DAC,
        0xDE8A_774B_CF36_D58B,
        0xC241_330B_01A9_E71F,
    ],
    [
        0x28DF_6EC4_CE4A_2BBD,
        0x6C30_7511_B2B9_437A,
        0x4722_9389_571A_A876,
        0x371E_CD7B_27CD_8130,
        0xC374_59EE_F50B_EA63,
    ],
];

/// Live state the third example's own [`Scene::show_selection_toolbar`] callback shares across steps: every row's
/// own already-computed `ThetaC` result. There is no current row kept here either — the
/// [`SelectionToolbarOptions`] toolbar rebuilt on every step is itself the only record of that (see
/// [`rebuild_theta_c_diagram`]'s own doc comment), which hands its own `to: Option<usize>` straight to
/// [`display_outputs`] on every step — so which rows currently show is a pure function of `outputs` and that one
/// position, recomputed fresh each time, not a second, separately mutated flag per row that could drift out of
/// step with it. See [`display_outputs`]'s own doc comment.
struct ThetaCDemo {
    /// Row `i`'s own `ThetaC` result — a pure function of [`THETA_C_INPUT`], computed once, up front. Stepping
    /// never recomputes these; it only changes which prefix of them [`display_outputs`] currently reveals.
    outputs: [u64; 5],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the cell-selection demo: three examples stepping through an array's values, each via its own in-canvas
/// [`Scene::show_selection_toolbar`] bar — Prev, Next, Restart — rather than any external HTML button. The first
/// two bind the toolbar directly to their own already-drawn array and update nothing else on a step but the
/// selection and a status line; the third needs a fresh toolbar on every step instead, since its own diagram is
/// redrawn from scratch each time — see [`rebuild_theta_c_diagram`]'s own doc comment for why.
///
/// Demonstrates [`Selection`]:
///
/// 1. The one-dimensional array (`GridLayout::Rows(1)`) only ever needs [`Selection::Cell`] — there is no separate
///    row to highlight distinctly from the one element within it.
/// 2. The two-dimensional array (`GridLayout::Columns`([`SELECTION_TWO_D_COLS`])) steps through its values in
///    row-major order. Each step highlights the whole row currently being processed, in
///    [`Selection::Row`]'s own band colour. It also highlights the specific cell currently being processed within
///    that row, in the stronger focus colour — the two-tier highlight a data-flow walk over a matrix needs.
/// 3. Its own value count leaves the last row short by two cells, a deliberately ragged grid. Stepping into that
///    row bands only its own real cells, demonstrating live what `resolve_selection`'s own incomplete-grid tests
///    already cover: a nominal row/column position is not always a real one.
/// 4. The third example steps an operator chain across an array, rather than just highlighting one. For each row
///    `n` of [`THETA_C_INPUT`], SHA-3's own `ThetaC` step computes
///    `C(n) = A(n,0) XOR A(n,1) XOR A(n,2) XOR A(n,3) XOR A(n,4)`, and writes it to output array `O(n)` — five
///    `u64` values folded through four [`BinaryOperator::Xor`] nodes, one operator chain per row. The selection
///    toolbar is bound to `O` itself: `O` holds exactly five values, one per row, so its own flat position *is*
///    `n` — no separate cursor is needed. `A` sits at the top of the canvas, above the chain it feeds; `O` sits
///    directly below the chain's own final `XOR` node, so a plain edge from there reaches `O` with nothing else in
///    the way — see [`rebuild_theta_c_diagram`]'s own doc comment.
/// 5. All three walks start unstarted — before element `0` is ever processed, nothing is highlighted and (for the
///    third) `O` is entirely blank. The third example's chain is still drawn, over five zero operands, rather than
///    left out entirely — see [`theta::theta_c::build_scene`](crate::theta::theta_c::build_scene)'s own doc comment
///    for why: an absent chain would read as "this does not exist yet," when what is actually true is "this has not
///    run yet." `O` shows exactly rows `0..=n` of `outputs` for the walk's current position `Some(n)`, and nothing
///    for `None` — see [`display_outputs`]'s own doc comment. Since that is recomputed fresh from the walk's own
///    current position on every step, "Previous" un-reveals a row exactly as readily as "Next" reveals one.
///
/// None of the three wrap: [`Scene::show_selection_toolbar`]'s own Next/Prev clamp at both ends instead — stepping
/// "Next" past the last value, or "Previous" before the first, simply disables that button rather than cycling
/// around.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing any of the three canvases this function
/// and [`rebuild_theta_c_diagram`] need (see its own `# Errors` section).
pub(crate) fn build_selection_demo() -> Result<(), String> {
    let document = crate::util::document()?;
    ensure_svg_in(
        &document,
        "selection-thetac-diagram-stage",
        "selection-thetac-diagram",
        Size::new(1180.0, 1010.0),
    )?;

    let one_d_values: Vec<u8> = vec![10, 20, 30, 40, 50, 60];
    crate::util::ensure_svg_in(
        &document,
        "selection-1d-diagram-stage",
        "selection-1d-diagram",
        Size::new(400.0, 120.0),
    )?;
    let one_d_svg = svg_dom::SvgRoot::attach("selection-1d-diagram").map_err(stringify)?;
    let one_d_scene = Scene::new(one_d_svg).map_err(stringify)?;
    let one_d_node = one_d_scene
        .add_named_data_node(
            Point::new(20.0, 20.0),
            "Some array",
            DataNodeContent::new(NodeValues::U8(one_d_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    let one_d_rect = one_d_scene.node_rect(one_d_node).map_err(stringify)?;
    fit_canvas_to_toolbar(&document, "selection-1d-diagram", one_d_rect)?;

    // Ten values over four columns: a 3×4 shape with the last row short by two cells.
    // See this function's own doc comment (point 3) — a deliberately ragged grid, not the coincidentally-exact
    // fit twelve values would be.
    let two_d_values: Vec<u8> = (1..=10).collect();
    crate::util::ensure_svg_in(
        &document,
        "selection-2d-diagram-stage",
        "selection-2d-diagram",
        Size::new(400.0, 200.0),
    )?;
    let two_d_svg = svg_dom::SvgRoot::attach("selection-2d-diagram").map_err(stringify)?;
    let two_d_scene = Scene::new(two_d_svg).map_err(stringify)?;
    let two_d_node = two_d_scene
        .add_named_data_node(
            Point::new(20.0, 20.0),
            "Some other array",
            DataNodeContent::new(NodeValues::U8(two_d_values), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(SELECTION_TWO_D_COLS)),
        )
        .map_err(stringify)?;
    let two_d_rect = two_d_scene.node_rect(two_d_node).map_err(stringify)?;
    fit_canvas_to_toolbar(&document, "selection-2d-diagram", two_d_rect)?;

    // Keeps both Scenes' only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some((one_d_scene.clone(), two_d_scene.clone())));

    show_one_d_toolbar(&document, &one_d_scene, one_d_node)?;
    show_two_d_toolbar(&document, &two_d_scene, two_d_node)?;

    // Unstarted: no row has been processed yet — see this function's own doc comment (point 5).
    let outputs = theta_c_outputs();
    let theta_c_demo = Rc::new(RefCell::new(ThetaCDemo { outputs }));
    rebuild_theta_c_diagram(None, display_outputs(outputs, None), theta_c_demo)
}

/// The clear space left between an array's own bottom edge and the selection toolbar drawn below it — a chosen
/// visual gap, not a measured one, the same way `V_GAP`-style constants elsewhere in this crate's own diagrams are.
const TOOLBAR_GAP: f64 = 20.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Resizes `id`'s own `<svg>` — via [`resize_svg`] — to exactly fit `content_rect` plus a default
/// [`SelectionToolbarOptions`] bar below it, with [`TOOLBAR_GAP`] clear between the two.
///
/// `content_rect` is `node_rect`'s own real, already-drawn size, not an estimate: unlike
/// [`crate::theta::xor_loop`]'s own `measure_named_data_node` calls, which need a box's size *before* deciding
/// where else to draw relative to it, this canvas's own size depends on nothing drawn after the array itself, so
/// there is nothing to gain from measuring ahead of drawing it for real.
///
/// Doing this from Rust, rather than hand-editing `index.html`'s own `viewBox` to match, is exactly the point: a
/// name added or removed from the array (changing its own rendered height) no longer needs a matching manual edit
/// to the canvas's own size anywhere else — this recomputes it from whatever the array actually rendered at.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#id`, or a DOM write fails.
fn fit_canvas_to_toolbar(document: &web_sys::Document, id: &str, content_rect: Rect) -> Result<(), String> {
    let toolbar = SelectionToolbarOptions::default();
    let width = 2.0 * content_rect.origin.x + content_rect.size.width;
    let height =
        content_rect.origin.y + content_rect.size.height + TOOLBAR_GAP + toolbar.margin + toolbar.button_height;
    resize_svg(document, id, Size::new(width, height))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text `#selection-1d-index` shows for flat position `to` — the plain index itself, or "not started" for the
/// unstarted state ([`Scene::show_selection_toolbar`]'s own initial `Selection::None`, before `on_step` has ever
/// run).
fn one_d_status(to: Option<usize>) -> String {
    to.map_or_else(|| "not started".to_string(), |i| i.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two-dimensional counterpart to [`one_d_status`]: `to`'s own row and column within a grid of `cols` columns.
fn two_d_status(to: Option<usize>, cols: usize) -> String {
    to.map_or_else(|| "not started".to_string(), |i| format!("row {}, col {}", i / cols, i % cols))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows a selection toolbar on `scene`, bound to `node` — the one-dimensional example's own control. `on_step`
/// only ever needs to update `#selection-1d-index`'s own text: the toolbar already owns `node`'s own [`Selection`]
/// entirely (see [`Scene::show_selection_toolbar`]'s own doc comment, "The managed node's `Selection` is the only
/// state"), so there is no separate index to keep in sync with it here.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#selection-1d-index`, or if showing the toolbar fails.
fn show_one_d_toolbar(document: &web_sys::Document, scene: &Scene, node: NodeId) -> Result<(), String> {
    let output = required_element(document, "selection-1d-index")?;
    output.set_text_content(Some(&one_d_status(None)));
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            output.set_text_content(Some(&one_d_status(transition.to)));
        })
        .map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two-dimensional counterpart to [`show_one_d_toolbar`] — same reasoning, just [`two_d_status`]'s own
/// "row `r`, col `c`" text in place of a plain index. The toolbar itself is what turns each step's own flat
/// position into the right [`Selection::Row`] for this node's own two-dimensional shape, via
/// `DataNodeContent::natural_selection`; nothing here constructs a `Selection` directly.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#selection-2d-index`, or if showing the toolbar fails.
fn show_two_d_toolbar(document: &web_sys::Document, scene: &Scene, node: NodeId) -> Result<(), String> {
    let output = required_element(document, "selection-2d-index")?;
    output.set_text_content(Some(&two_d_status(None, SELECTION_TWO_D_COLS)));
    scene
        .show_selection_toolbar(node, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            output.set_text_content(Some(&two_d_status(transition.to, SELECTION_TWO_D_COLS)));
        })
        .map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Row `i`'s own `ThetaC` result — a pure function of [`THETA_C_INPUT`]. Shared by every caller that needs it:
/// [`build_selection_demo`]'s own initial state, and [`crate::theta`]'s own nested walk, both standalone and
/// nested starting from row `0` already stepped.
pub(crate) fn theta_c_outputs() -> [u64; 5] {
    THETA_C_INPUT.map(|row| row[0] ^ row[1] ^ row[2] ^ row[3] ^ row[4])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Row `i` of `outputs` shows through for `i <= to.unwrap()`, `0` everywhere else — the walk's own current
/// position `to` is the *only* state this reads: no separately mutated "has row `i` ever been written" flag, so
/// "Previous" un-reveals a later row exactly as it reveals an earlier one, with nothing left over from before to
/// forget to clear. `None` (unstarted, or walked/restarted all the way back) reveals nothing.
pub(crate) fn display_outputs(outputs: [u64; 5], to: Option<usize>) -> [u64; 5] {
    std::array::from_fn(|i| if to.is_some_and(|n| i <= n) { outputs[i] } else { 0 })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds `#selection-thetac-diagram` from scratch for `n`, and attaches a fresh
/// [`Scene::show_selection_toolbar`] — bound to the freshly drawn output array `O` — to drive the *next* step. See
/// [`theta::theta_c::build_scene`](crate::theta::theta_c::build_scene) for what is drawn and why a fresh `Scene` is
/// unavoidable here; this is the standalone Cell Selection demo's own wrapper around it, the counterpart to
/// [`show_one_d_toolbar`]/[`show_two_d_toolbar`] for the third example.
///
/// `Scene::show_selection_toolbar` always resets its own managed node back to [`Selection::None`] as its first
/// *committed* act (see its own doc comment) — exactly wrong here whenever `n` is `Some`, since the diagram was just
/// (re)drawn to show that row's own real position. So `n`'s own real selection is reapplied immediately afterward.
/// This is not working around the library; it is exactly what its own design already allows for: with no private
/// cursor of its own, the toolbar simply treats this second `set_selection` call as the new position its own next
/// click advances from — see `Scene::show_selection_toolbar`'s own doc comment ("The managed node's `Selection` is
/// the only state").
///
/// `on_step` closes over `state`, ready for the *next* step: [`step_theta_c`] updates `state`'s own `written`
/// flags for the walk's new position, then calls this same function again with the result — a fresh `Scene`, a
/// fresh output node, and a fresh toolbar wired the same way, to keep the chain going. There is no separate row
/// counter anywhere in this module: `n`, and the toolbar's own Prev/Next/Restart enabled state, are entirely the
/// selection toolbar's own doing.
///
/// The new `Scene` is kept in [`THETA_C_SCENE`], replacing the previous step's. Nothing here is draggable, but
/// every listener this crate installs — the zoom toolbar's, the selection toolbar's — holds only a `Weak`
/// reference to its own `Scene`, so all of them stop responding the moment the last handle is dropped.
/// `theta::theta_c::build_scene`'s own `set_inner_html("")` clears the previous step's DOM, and replacing the
/// stored handle then frees the previous `Scene` — along with the selection toolbar's own `on_step` closure it was
/// the sole owner of, per [`Scene::hide_selection_toolbar`]'s own doc comment ("Ownership").
///
/// Each step draws a new `Scene`, which would otherwise reset zoom/pan back to `1.0`/`(0, 0)` — jarring, if the
/// previous step's own view had been zoomed or panned in first. So the outgoing `Scene`'s own
/// [`Scene::view`](svg_dom_graph::scene::Scene::view) is read before it is replaced, and carried over onto the
/// fresh one via [`Scene::set_view`](svg_dom_graph::scene::Scene::set_view).
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#selection-thetac-diagram`, or if any library call fails.
fn rebuild_theta_c_diagram(n: Option<usize>, display: [u64; 5], state: Rc<RefCell<ThetaCDemo>>) -> Result<(), String> {
    let view = THETA_C_SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));

    let (scene, output) = crate::theta::theta_c::build_scene("selection-thetac-diagram", n, display)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    scene
        .show_selection_toolbar(output, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            step_theta_c(&state, transition.to);
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(output, Selection::Cell(n)).map_err(stringify)?;
    }

    THETA_C_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`Scene::show_selection_toolbar`]'s own `on_step` callback for the third example: rebuilds the whole diagram
/// for the walk's new position `to`, via [`rebuild_theta_c_diagram`].
///
/// `to` alone is enough to know what `O` should show — see [`display_outputs`]'s own doc comment — so there is no
/// separate flag to update here first. [`SelectionTransition`](svg_dom_graph::scene::SelectionTransition)'s own
/// doc comment explains why the toolbar itself cannot tell "Previous" and "Restart" apart (both land on
/// `to.is_none()`), and, since `display_outputs` derives its result fresh from `to` alone either way, why this
/// never needed to.
///
/// A [`rebuild_theta_c_diagram`] failure here would mean `index.html` no longer matches this module, or the
/// library itself failed — already ruled out by this same call having succeeded once already, to get this far. So
/// it is ignored, rather than given a `Result` a toolbar click has nowhere to return.
fn step_theta_c(state: &Rc<RefCell<ThetaCDemo>>, to: Option<usize>) {
    let demo = state.borrow();
    let display = display_outputs(demo.outputs, to);
    drop(demo);
    let _ = rebuild_theta_c_diagram(to, display, state.clone());
}
