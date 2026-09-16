//! `panel-selection` / `#selection-1d-diagram` and `#selection-2d-diagram`: a one-dimensional and a
//! two-dimensional array, each with its own "Previous"/"Next" buttons stepping [`Scene::set_selection`] through
//! its values. See [`build_selection_demo`]'s own doc comment for what each demonstrates.

use crate::util::{required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection},
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::Element;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("selection.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own two, separate `Scene`s — one per
    // array dimension.
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

/// How many columns the two-dimensional demo array uses — 3 rows of 4, so the row-banding is visually obvious
/// without the grid being large enough to make counting cells tedious.
const SELECTION_TWO_D_COLS: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the cell-selection demo: a one-dimensional array and a two-dimensional array, each with its own
/// "Previous"/"Next" buttons stepping [`Scene::set_selection`] through its values.
///
/// Demonstrates [`Selection`]:
///
/// 1. The one-dimensional array (`GridLayout::Rows(1)`) only ever needs [`Selection::Cell`] — there is no separate
///    row to highlight distinctly from the one element within it.
/// 2. The two-dimensional array (`GridLayout::Columns`([`SELECTION_TWO_D_COLS`])) steps through its values in
///    row-major order. Each step highlights the whole row currently being processed, in
///    [`Selection::Row`]'s own band colour. It also highlights the specific cell currently being processed within
///    that row, in the stronger focus colour — the two-tier highlight a data-flow walk over a matrix needs.
///
/// Both wrap: stepping "Next" past the last value returns to the first, and "Previous" from the first goes to the
/// last — see [`wire_selection_controls`]'s own `step_one_d`/`step_two_d` helpers.
///
/// # Errors
///
/// Returns `Err` if any library call fails, if `index.html` is missing `#selection-1d-diagram`/
/// `#selection-2d-diagram`, or if [`wire_selection_controls`] cannot wire up its controls (see that function's own
/// `# Errors` section).
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

    let two_d_values: Vec<u8> = (1..=12).collect();
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

    wire_selection_controls(document, Rc::new(RefCell::new(demo)))
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
