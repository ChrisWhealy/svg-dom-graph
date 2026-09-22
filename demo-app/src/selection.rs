//! `panel-selection` / `#selection-1d-diagram`, `#selection-2d-diagram`, and `#selection-thetac-diagram`: three
//! examples, each with its own "Previous"/"Next" buttons stepping through an array's values. See
//! [`build_selection_demo`]'s own doc comment for what each demonstrates.

use crate::util::{required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeOptions, NodeValues, Scene, Selection,
    },
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::Element;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("selection.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own two, separate `Scene`s — one per
    // array dimension. The third example's own canvas needs no such handle — see
    // [`rebuild_theta_c_diagram`]'s own doc comment for why.
    static SCENE: RefCell<Option<(Scene, Scene)>> = const { RefCell::new(None) };
}

/// Live state `wire_selection_controls`' four button listeners share: each array's own `Scene`, node id, current
/// flat index, and (for the two-dimensional array) column count.
struct SelectionDemo {
    one_d_scene: Scene,
    one_d_node: NodeId,
    one_d_len: usize,
    one_d_index: usize,
    two_d_scene: Scene,
    two_d_node: NodeId,
    two_d_cols: usize,
    two_d_len: usize,
    two_d_index: usize,
}

/// How many columns the two-dimensional demo array uses. Its own value count (see [`build_selection_demo`])
/// deliberately does not divide evenly by this, so the grid's own last row renders short.
const SELECTION_TWO_D_COLS: usize = 4;

/// The third example's own fixed input, `A(row, col)` — made-up values, not a real Keccak state, chosen only to
/// give each row's own `ThetaC` chain a visibly distinct result. See [`build_selection_demo`]'s own doc comment
/// (point 4).
const THETA_C_INPUT: [[u64; 5]; 5] = [
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

/// Live state the third example's own button listeners share: the current row `n`, and every row's own
/// already-computed `ThetaC` result. Nothing here needs a `Scene`/`NodeId` of its own — see
/// [`rebuild_theta_c_diagram`]'s own doc comment for why the whole canvas is simply rebuilt every step instead.
struct ThetaCDemo {
    n: usize,
    /// Row `i`'s own `ThetaC` result — a pure function of [`THETA_C_INPUT`], computed once, up front. Stepping
    /// never recomputes these; it only changes which ones [`written`](Self::written) currently reveals.
    outputs: [u64; 5],
    /// `written[i]` is `true` once row `i` has been stepped into going forward, and not since stepped away from
    /// going backward. [`rebuild_theta_c_diagram`] shows [`outputs`](Self::outputs)`[i]` for a written row, and
    /// `0` — the output array's own initial value — for one that is not. See [`build_selection_demo`]'s own doc
    /// comment (point 5).
    written: [bool; 5],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the cell-selection demo: three examples, each with its own "Previous"/"Next" buttons stepping through an
/// array's values.
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
///    `u64` values folded through four [`BinaryOperator::Xor`] nodes, one operator chain per row. `n` is clamped to
///    `0..=4`, not wrapped like the first two examples — see [`step_theta_c_next`]/[`step_theta_c_prev`]. `A`
///    sits at the top of the canvas, above the chain it feeds; `O` sits directly below the chain's own final
///    `XOR` node, so a plain edge from there reaches `O` with nothing else in the way — see
///    [`rebuild_theta_c_diagram`]'s own doc comment.
/// 5. Stepping "Previous" resets the row being left — not the row arrived at — back to `O`'s own initial value,
///    demonstrating that a walked-past output is only ever valid because the walk itself produced it, not because
///    it is somehow always available. See [`ThetaCDemo::written`]'s own doc comment.
///
/// Both the first two examples wrap: stepping "Next" past the last value returns to the first, and "Previous" from
/// the first goes to the last — see [`wire_selection_controls`]'s own `step_one_d`/`step_two_d` helpers.
///
/// # Errors
///
/// Returns `Err` if any library call fails, if `index.html` is missing any of the three canvases this function and
/// [`wire_selection_controls`]/[`wire_theta_c_controls`] need, or if either wiring function cannot wire up its own
/// controls (see their own `# Errors` sections).
pub(crate) fn build_selection_demo() -> Result<(), String> {
    let document = crate::util::document()?;

    let one_d_values: Vec<u8> = vec![10, 20, 30, 40, 50, 60];
    let one_d_len = one_d_values.len();
    let one_d_svg = svg_dom::SvgRoot::attach("selection-1d-diagram").map_err(stringify)?;
    let one_d_scene = Scene::new(one_d_svg).map_err(stringify)?;
    let one_d_node = one_d_scene
        .add_data_node(
            Point::new(20.0, 20.0),
            DataNodeContent::new(NodeValues::U8(one_d_values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;

    // Ten values over four columns: a 3×4 shape with the last row short by two cells.
    // See this function's own doc comment (point 3) — a deliberately ragged grid, not the coincidentally-exact
    // fit twelve values would be.
    let two_d_values: Vec<u8> = (1..=10).collect();
    let two_d_len = two_d_values.len();
    let two_d_svg = svg_dom::SvgRoot::attach("selection-2d-diagram").map_err(stringify)?;
    let two_d_scene = Scene::new(two_d_svg).map_err(stringify)?;
    let two_d_node = two_d_scene
        .add_data_node(
            Point::new(20.0, 20.0),
            DataNodeContent::new(NodeValues::U8(two_d_values), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(SELECTION_TWO_D_COLS)),
        )
        .map_err(stringify)?;

    // Keeps both Scenes' only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some((one_d_scene.clone(), two_d_scene.clone())));

    let demo = SelectionDemo {
        one_d_scene,
        one_d_node,
        one_d_len,
        one_d_index: 0,
        two_d_scene,
        two_d_node,
        two_d_cols: SELECTION_TWO_D_COLS,
        two_d_len,
        two_d_index: 0,
    };
    wire_selection_controls(document.clone(), Rc::new(RefCell::new(demo)))?;

    let outputs = THETA_C_INPUT.map(|row| row[0] ^ row[1] ^ row[2] ^ row[3] ^ row[4]);
    let mut written = [false; 5];
    written[0] = true;
    let theta_c_demo = ThetaCDemo { n: 0, outputs, written };
    wire_theta_c_controls(document, Rc::new(RefCell::new(theta_c_demo)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Applies `demo`'s own current `one_d_index` as a [`Selection::Cell`], and shows it in `output`.
///
/// Errors from `set_selection` are ignored rather than propagated. `demo.one_d_index` is always kept in
/// `0..demo.one_d_len` by `step_one_d` below, so this call cannot fail in practice. A live button handler should never
/// panic or stop responding over a stray, already-impossible error.
fn apply_one_d_selection(demo: &SelectionDemo, output: &Element) {
    let _ = demo
        .one_d_scene
        .set_selection(demo.one_d_node, Selection::Cell(demo.one_d_index));
    output.set_text_content(Some(&demo.one_d_index.to_string()));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Applies `demo`'s own current `two_d_index` as a [`Selection::Row`] — the row it falls in, banded, and its own column
/// within that row focused — and shows both in `output`. Same "cannot fail in practice" reasoning as
/// [`apply_one_d_selection`].
fn apply_two_d_selection(demo: &SelectionDemo, output: &Element) {
    let row = demo.two_d_index / demo.two_d_cols;
    let col = demo.two_d_index % demo.two_d_cols;
    let _ = demo
        .two_d_scene
        .set_selection(demo.two_d_node, Selection::Row { row, col: Some(col) });
    output.set_text_content(Some(&format!("row {row}, col {col}")));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Steps `state`'s own `one_d_index` by `delta` (`1` for "Next", `-1` for "Previous"), wrapping at both ends via
/// [`isize::rem_euclid`], and reapplies the selection.
fn step_one_d(state: &Rc<RefCell<SelectionDemo>>, output: &Element, delta: isize) {
    let mut demo = state.borrow_mut();
    #[allow(clippy::cast_possible_wrap)]
    let len = demo.one_d_len as isize;
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let next = (demo.one_d_index as isize + delta).rem_euclid(len) as usize;
    demo.one_d_index = next;
    apply_one_d_selection(&demo, output);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two-dimensional counterpart to [`step_one_d`], stepping `state`'s own `two_d_index` instead.
fn step_two_d(state: &Rc<RefCell<SelectionDemo>>, output: &Element, delta: isize) {
    let mut demo = state.borrow_mut();
    #[allow(clippy::cast_possible_wrap)]
    let len = demo.two_d_len as isize;
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let next = (demo.two_d_index as isize + delta).rem_euclid(len) as usize;
    demo.two_d_index = next;
    apply_two_d_selection(&demo, output);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#selection-1d-prev`/`#selection-1d-next` and `#selection-2d-prev`/`#selection-2d-next` to `state`.
///
/// Also applies the initial selection (index/row/col `0`) to each array immediately, so the panel shows a
/// highlighted cell without needing a click first.
///
/// Every listener captures `state` (or a clone of one of its own fields) and is never dropped.
/// `Closure::forget` leaks them deliberately, for the page's whole lifetime — the same span `SCENE`
/// itself covers.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing any of `#selection-1d-prev`, `#selection-1d-next`,
/// `#selection-1d-index`, `#selection-2d-prev`, `#selection-2d-next`, or `#selection-2d-index`, or if a listener
/// could not be attached to any of them.
fn wire_selection_controls(document: web_sys::Document, state: Rc<RefCell<SelectionDemo>>) -> Result<(), String> {
    let one_d_prev = required_element(&document, "selection-1d-prev")?;
    let one_d_next = required_element(&document, "selection-1d-next")?;
    let one_d_output = required_element(&document, "selection-1d-index")?;
    let two_d_prev = required_element(&document, "selection-2d-prev")?;
    let two_d_next = required_element(&document, "selection-2d-next")?;
    let two_d_output = required_element(&document, "selection-2d-index")?;

    apply_one_d_selection(&state.borrow(), &one_d_output);
    apply_two_d_selection(&state.borrow(), &two_d_output);

    let s = state.clone();
    let out = one_d_output.clone();
    let one_d_prev_closure = Closure::<dyn FnMut()>::new(move || step_one_d(&s, &out, -1));
    one_d_prev
        .add_event_listener_with_callback("click", one_d_prev_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the 1D previous-button listener: {e:?}"))?;
    one_d_prev_closure.forget();

    let s = state.clone();
    let one_d_next_closure = Closure::<dyn FnMut()>::new(move || step_one_d(&s, &one_d_output, 1));
    one_d_next
        .add_event_listener_with_callback("click", one_d_next_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the 1D next-button listener: {e:?}"))?;
    one_d_next_closure.forget();

    let s = state.clone();
    let out = two_d_output.clone();
    let two_d_prev_closure = Closure::<dyn FnMut()>::new(move || step_two_d(&s, &out, -1));
    two_d_prev
        .add_event_listener_with_callback("click", two_d_prev_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the 2D previous-button listener: {e:?}"))?;
    two_d_prev_closure.forget();

    let two_d_next_closure = Closure::<dyn FnMut()>::new(move || step_two_d(&state, &two_d_output, 1));
    two_d_next
        .add_event_listener_with_callback("click", two_d_next_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the 2D next-button listener: {e:?}"))?;
    two_d_next_closure.forget();

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `outputs[i]` where `written[i]`, or `0` — the output array's own initial value — where not. See
/// [`ThetaCDemo::written`]'s own doc comment.
fn display_outputs(outputs: [u64; 5], written: [bool; 5]) -> [u64; 5] {
    let mut display = [0u64; 5];
    for i in 0..5 {
        if written[i] {
            display[i] = outputs[i];
        }
    }
    display
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds `#selection-thetac-diagram` from scratch, for row `n`: a fresh five-operand, four-`XOR` chain
/// computing `ThetaC` over `THETA_C_INPUT[n]`, a fresh output array showing `display`'s own current values with
/// cell `n` focused, and a fresh input array with row `n` banded — plus a plain edge from the chain's own final
/// `XOR` node into the output array.
///
/// `svg-dom-graph` has no way to change a node's own displayed value once drawn — only its selection (see
/// `Scene::set_selection`'s own doc comment). Since every value here — the row currently feeding the chain, the
/// chain's own intermediate results, and however many output cells have so far been "written" — changes on every
/// step, there is no existing node any of this could update in place. So, exactly like
/// `edge_anchors::rebuild_edge_anchors_scene`, each step clears `#selection-thetac-diagram`'s own children and
/// draws everything again, fresh, from this row's own real values.
///
/// `O` is its own `[5; u64]` node, not folded into `A`'s own `[5; [5; u64]]` shape — each keeps the type its own
/// values actually have. `A` sits at the very top of the canvas, above the chain it feeds; `O` sits right below
/// the chain's own final `XOR` node, so a connector from there reaches `O` directly, with nothing else in the way
/// — a connector can only land on a node's own outer perimeter, never a specific cell inside it, and `O` is the
/// node whose perimeter that connector actually reaches.
///
/// This canvas needs no `Scene` kept alive afterward, unlike every other demo in this crate: nothing here is
/// draggable, so nothing installs a listener that could later call back into a dropped `Scene`. The next step's
/// own [`web_sys::Element::set_inner_html`] clears the DOM directly, independently of whether any `Scene` handle
/// for the previous step still exists. So this function simply lets its own `Scene` drop at its own end.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#selection-thetac-diagram`, or if any library call fails.
fn rebuild_theta_c_diagram(n: usize, display: [u64; 5]) -> Result<(), String> {
    let document = crate::util::document()?;
    let container = required_element(&document, "selection-thetac-diagram")?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach("selection-thetac-diagram").map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let row = THETA_C_INPUT[n];
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);
    let place = |x: f64, y: f64, value: u64| -> Result<NodeId, String> {
        scene.add_data_node(Point::new(x, y), hex(value)).map_err(stringify)
    };

    // The input array: `A`'s own 25 values, its own `[5; [5; u64]]` node, at the very top of the canvas — see
    // this function's own doc comment.
    let array = scene
        .add_data_node(
            Point::new(20.0, 20.0),
            DataNodeContent::new(
                NodeValues::U64(THETA_C_INPUT.iter().flatten().copied().collect()),
                DataFormat::Hexadecimal,
            )
            .with_layout(GridLayout::Rows(5)),
        )
        .map_err(stringify)?;
    scene
        .set_selection(array, Selection::Row { row: n, col: None })
        .map_err(stringify)?;

    // The five operands sit in one horizontal row below the input array, in the same left-to-right order as
    // `row`'s own values there — so this canvas reads as "here is that row, unpacked." Their own spacing
    // approximates the array's own real cell stride (a single-value `u64` hex cell, plus the same `CELL_GAP` the
    // array's own adjacent cells use). `svg-dom-graph` only knows a cell's own real rendered width once it has
    // actually measured the text — those constants are private to it, unavailable here — so this is a close
    // estimate, not an exact figure, as requested.
    //
    // Each `XOR` stage cascades down and to the left of the operand row, combining the running total with the
    // next operand to its own right; every stage's own `x` sits strictly left of the operand it still has to
    // reach, and every operand's own connector drops straight down its own column before turning, so nothing here
    // ever crosses an earlier stage's box.
    const OPERAND_X: [f64; 5] = [20.0, 220.0, 420.0, 620.0, 820.0];
    const OPERAND_Y: f64 = 275.0;

    // `t1` sits close enough below the operand row that the gap between them — the operand box's own bottom
    // edge (30.2 units tall: a fixed constant, not measured text) to `t1`'s own top — is roughly half of what
    // every earlier version of this diagram left there (≈99.8 units, down to ≈50).
    let op0 = place(OPERAND_X[0], OPERAND_Y, row[0])?;
    let op1 = place(OPERAND_X[1], OPERAND_Y, row[1])?;
    let xor01 = row[0] ^ row[1];
    let t1 = scene
        .add_binary_operator_node(Point::new(120.0, 355.0), BinaryOperator::Xor, (op0, op1), hex(xor01))
        .map_err(stringify)?;

    // Each later stage sits 112 units below the previous one: an operator node's own height (71.8 units, again a
    // fixed constant) plus a ≈40-unit gap between them, per request.
    let op2 = place(OPERAND_X[2], OPERAND_Y, row[2])?;
    let xor012 = xor01 ^ row[2];
    let t2 = scene
        .add_binary_operator_node(Point::new(270.0, 467.0), BinaryOperator::Xor, (t1, op2), hex(xor012))
        .map_err(stringify)?;

    let op3 = place(OPERAND_X[3], OPERAND_Y, row[3])?;
    let xor0123 = xor012 ^ row[3];
    let t3 = scene
        .add_binary_operator_node(Point::new(445.0, 579.0), BinaryOperator::Xor, (t2, op3), hex(xor0123))
        .map_err(stringify)?;

    // The final stage always sits at the same position — the one it would occupy for `n == 3` — rather than
    // tracking column `n` the way earlier attempts here did. Nothing else occupies the space between it and the
    // output array below, so moving it was never necessary for the connector's own safety; it just moved without
    // a reason to.
    let op4 = place(OPERAND_X[4], OPERAND_Y, row[4])?;
    let xor01234 = xor0123 ^ row[4];
    let result = scene
        .add_binary_operator_node(Point::new(OPERAND_X[3], 691.0), BinaryOperator::Xor, (t3, op4), hex(xor01234))
        .map_err(stringify)?;

    // The output array: `O`'s own five values, its own `[5; u64]` node — see this function's own doc comment for
    // why it stays distinct from `A` rather than folding into `A`'s own shape. Five fixing points on every side,
    // so the connector below can snap to whichever of them best approximates column `n`, rather than landing
    // wherever a single, unconfigured anchor would pick.
    let output_options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(5)));
    let output = scene
        .add_data_node_with(
            Point::new(20.0, 823.0),
            DataNodeContent::new(NodeValues::U64(display.to_vec()), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Rows(1)),
            output_options,
        )
        .map_err(stringify)?;
    scene.set_selection(output, Selection::Cell(n)).map_err(stringify)?;

    scene.add_edge(result, output).map_err(stringify)?;

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shows `demo`'s own current row `n` in `output`, and rebuilds the whole diagram for it — see
/// [`rebuild_theta_c_diagram`].
///
/// A [`rebuild_theta_c_diagram`] failure would mean `index.html` no longer matches this module, or the library
/// itself failed — already ruled out by this same call succeeding once already, during
/// [`wire_theta_c_controls`]'s own initial call. So it is ignored here, rather than given a `Result` a button
/// click has nowhere to return — the same "cannot fail in practice" reasoning [`apply_one_d_selection`] gives.
fn apply_theta_c(demo: &ThetaCDemo, output: &Element) {
    output.set_text_content(Some(&demo.n.to_string()));
    let display = display_outputs(demo.outputs, demo.written);
    let _ = rebuild_theta_c_diagram(demo.n, display);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Steps `state`'s own row `n` forward by one, clamped at `4` — unlike [`step_one_d`]/[`step_two_d`], this does not
/// wrap. Marks the newly arrived row as written, then reapplies via [`apply_theta_c`].
fn step_theta_c_next(state: &Rc<RefCell<ThetaCDemo>>, output: &Element) {
    let mut demo = state.borrow_mut();
    if demo.n < 4 {
        demo.n += 1;
        let n = demo.n;
        demo.written[n] = true;
    }
    apply_theta_c(&demo, output);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Steps `state`'s own row `n` backward by one, clamped at `0`. Marks the row being left — not the one arrived at
/// — as no longer written, before moving, then reapplies via [`apply_theta_c`]. See [`build_selection_demo`]'s own
/// doc comment (point 5).
fn step_theta_c_prev(state: &Rc<RefCell<ThetaCDemo>>, output: &Element) {
    let mut demo = state.borrow_mut();
    if demo.n > 0 {
        let n = demo.n;
        demo.written[n] = false;
        demo.n -= 1;
    }
    apply_theta_c(&demo, output);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#selection-thetac-prev`/`#selection-thetac-next` to `state`, the third example's own counterpart to
/// [`wire_selection_controls`].
///
/// Also applies the initial row (`0`) immediately, so the panel shows a highlighted row and a freshly built chain
/// without needing a click first — the same reasoning [`wire_selection_controls`] already gives.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#selection-thetac-prev`, `#selection-thetac-next`, or
/// `#selection-thetac-index`, or if a listener could not be attached to either button.
fn wire_theta_c_controls(document: web_sys::Document, state: Rc<RefCell<ThetaCDemo>>) -> Result<(), String> {
    let prev = required_element(&document, "selection-thetac-prev")?;
    let next = required_element(&document, "selection-thetac-next")?;
    let output = required_element(&document, "selection-thetac-index")?;

    apply_theta_c(&state.borrow(), &output);

    let s = state.clone();
    let out = output.clone();
    let prev_closure = Closure::<dyn FnMut()>::new(move || step_theta_c_prev(&s, &out));
    prev.add_event_listener_with_callback("click", prev_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the ThetaC previous-button listener: {e:?}"))?;
    prev_closure.forget();

    let next_closure = Closure::<dyn FnMut()>::new(move || step_theta_c_next(&state, &output));
    next.add_event_listener_with_callback("click", next_closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the ThetaC next-button listener: {e:?}"))?;
    next_closure.forget();

    Ok(())
}
