//! The nested `XOR loop` child: SHA3's own `A'(x,y) = A(x,y) ⊕ D(x)` step — the final fold of `Theta` back into the
//! state array — its own selection toolbar, and rebuilding it — via `Scene::replace_container_child` — every time
//! that toolbar steps to a new cell of `A`.
//!
//! Unlike `theta_c`/`theta_d`, which each step through one of five *rows*, this one steps through all 25
//! individual *cells* of `A` — see [`build_scene`]'s own doc comment for why, and [`XorLoopState`]'s own doc
//! comment for why it needs its own state shape rather than sharing `support::SteppedChildState`.

use super::support::{create_child_svg, next_child_svg_id};
use crate::{
    selection::THETA_C_INPUT,
    util::{required_element, stringify},
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};

thread_local! {
    // The same trio as `theta_c`'s own `SCENE` — see its own doc comment.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Live state this nested child's own selection toolbar carries across steps — the `XOR loop` counterpart to
/// `support::SteppedChildState`, kept separate from it rather than generalising that struct to fit both.
///
/// `SteppedChildState` is shaped `[u64; 5]`/`[bool; 5]` throughout — one value per row, since `theta_c`'s own `O`
/// and `theta_d`'s own `D` both really are five-value arrays. `XOR loop`'s own output, `A'`, is a genuine
/// `[5; [5; u64]]` — the same shape as `A` itself — not five values but twenty-five, so it needs its own
/// `[[u64; 5]; 5]`/`[[bool; 5]; 5]` shape throughout instead. Forcing that through `SteppedChildState`'s own fixed
/// `[T; 5]` fields would mean a breaking change to two working, already-tested modules for the sake of a third
/// that does not actually share their own shape — the same "a deliberate, separately-tested copy rather than a
/// shared dependency" reasoning this crate already follows for `toolbar::layout` vs. `selection_toolbar::layout`.
pub(super) struct XorLoopState {
    /// `outputs[row][col] = A(row, col) ⊕ D(row)` — every cell's own real result, computed once, up front. See
    /// [`outputs`]'s own doc comment.
    pub(super) outputs: [[u64; 5]; 5],
    /// `written[row][col]` is `true` once that cell has been stepped into going forward, and not since stepped
    /// away from going backward, or since a `Restart` swept every cell back to unstarted in one go — the same
    /// `written` contract `SteppedChildState::written`'s own doc comment gives, just indexed by `(row, col)`
    /// instead of a flat `i`.
    pub(super) written: [[bool; 5]; 5],
    /// The id of whichever `<svg>` currently backs this nested child — see [`rebuild_child`]'s own doc comment for
    /// why every step needs a fresh one.
    pub(super) child_svg_id: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `XOR loop` child currently grafted in — the same role
/// `theta_c::init_scene` plays for `ThetaC`.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `XOR loop` child if it is currently focused, and reports whether it was — the same role
/// `theta_c::exit_if_focused` plays for `ThetaC`.
pub(super) fn exit_if_focused() -> bool {
    SCENE.with_borrow(|slot| {
        let Some((_, child, _)) = slot else { return false };
        if !child.is_focused() {
            return false;
        }
        let _ = child.exit();
        true
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `A'(row, col) = A(row, col) ⊕ D(row)` for every cell — `XOR loop`'s own complete result, computed once up
/// front, the same way `theta_d::outputs` computes `ThetaD`'s. [`build_theta_demo`](super::build_theta_demo)/
/// [`step`] use this to know every cell's own real value regardless of how far the walk has actually stepped;
/// [`display_outputs`] is what reveals them progressively.
pub(super) fn outputs(a: [[u64; 5]; 5], d: [u64; 5]) -> [[u64; 5]; 5] {
    std::array::from_fn(|row| std::array::from_fn(|col| a[row][col] ^ d[row]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `[[u64; 5]; 5]` counterpart to `crate::selection::display_outputs`: every value from `outputs` where
/// `written` is `true`, `0` everywhere else. See [`XorLoopState::written`]'s own doc comment.
pub(super) fn display_outputs(outputs: [[u64; 5]; 5], written: [[bool; 5]; 5]) -> [[u64; 5]; 5] {
    let mut display = [[0u64; 5]; 5];
    for row in 0..5 {
        for col in 0..5 {
            if written[row][col] {
                display[row][col] = outputs[row][col];
            }
        }
    }
    display
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested `XOR loop` child for flat cell index `n` (`row = n / 5`, `col = n % 5`), in one row-1/row-2
/// pair: `D` — `ThetaD`'s own real output, [`super::theta_d::outputs`], drawn as a `[5; 1]` column (row `row`
/// focused) — sits at `A`'s own west, both in the same row and the same height; `D[row]` sits centred below `D`,
/// and `A[row, col]` sits centred below `A`, each its own single-value node fed by an edge from the array above
/// it. `XOR`, east of `A[row, col]` in that same row, combines them into `A'(row, col)` — both of its own inputs
/// then approach it from the west, the same `theta_d::build_scene`'s own `MID_Y` reasoning applied here without a
/// shared source array to put them in one row naturally. `A'`, at the bottom, is the same shape and layout as `A`
/// itself, showing `display`'s own current values with cell `(row, col)` focused.
///
/// Steps cell by cell, not row by row like `theta_c`/`theta_d` do: `A'(row, col)` is a genuinely per-cell result —
/// every column of a row is `⊕`'d with the *same* `D(row)`, but each still needs its own real `A(row, col)` — so
/// there is no single "row's own result" this could reveal all at once the way `theta_c`'s `O`/`theta_d`'s `D` can.
/// Binding the toolbar to `A'` itself, a genuine `[5; [5; u64]]` node, gets this for free:
/// `DataNodeContent::flat_index`/`natural_selection` already walk a two-dimensional grid's own values row-major,
/// one cell at a time, which is exactly the walk this step needs.
///
/// `Some(n)` computes the chain over `A[row, col]`/`D[row]` and focuses that cell in both `A` and `A'`, and row
/// `row` in `D`; `None` — the unstarted state, before cell `0` is ever processed — computes the same chain over
/// `A[0, 0]`/`D[0]` instead, and leaves `A`/`D`/`A'` unselected. See `theta_c::build_scene`'s own doc comment for
/// why the chain is always drawn, even unstarted, and why `display`'s own current values are passed in rather than
/// computed here.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing an `<svg id="{svg_id}">`, or if any library call fails.
pub(super) fn build_scene(svg_id: &str, n: Option<usize>, display: [[u64; 5]; 5]) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Keccak Theta XOR Loop", SceneTitleOptions::default())
        .map_err(stringify)?;

    let d = super::theta_d::outputs(crate::selection::theta_c_outputs());
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    // The gap every row transition in this diagram leaves — see `theta_d::build_scene`'s own `PREV_NEXT_Y` doc
    // comment for why a gap this size, not the tighter one an earlier version of that diagram used.
    const V_GAP: f64 = 58.0;
    // The horizontal counterpart, between `D`/`A` and between `A[row, col]`/`XOR` — narrower, since these sit
    // side by side rather than stacked, and have no label row of their own competing for the same space.
    const H_GAP: f64 = 40.0;
    // A single-value/operator box's own estimated width — see `theta_c::build_scene`'s own doc comment for where
    // this figure comes from. Used only to *centre* `A[row, col]`/`D[row]` under their own source arrays; a modest
    // misestimate here still leaves their own centres close enough to `A`'s/`D`'s own that the vertical ray-cast
    // into them resolves North regardless — see `theta_d::build_scene`'s own `MID_Y` doc comment for the formula
    // that makes that true even when the two centres are not *exactly* aligned.
    const CELL_BOX_WIDTH: f64 = 211.0;

    // `D`, not `A`, sits first — to `A`'s own west, both in one row, both the same height (each a five-row grid,
    // one column or five) so their tops and bottoms line up exactly.
    let d_array = scene
        .add_named_data_node(
            Point::new(LEFT_X, TOP_Y),
            "Theta D Output",
            DataNodeContent::new(NodeValues::U64(d.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(5)),
        )
        .map_err(stringify)?;
    let d_rect = scene.node_rect(d_array).map_err(stringify)?;

    let a = scene
        .add_named_data_node(
            Point::new(d_rect.origin.x + d_rect.size.width + H_GAP, TOP_Y),
            "A Bytes",
            DataNodeContent::new(
                NodeValues::U64(THETA_C_INPUT.iter().flatten().copied().collect()),
                DataFormat::Hexadecimal,
            )
            .with_layout(GridLayout::Rows(5)),
        )
        .map_err(stringify)?;
    let a_rect = scene.node_rect(a).map_err(stringify)?;

    let (row, col) = n.map_or((0, 0), |n| (n / 5, n % 5));
    if n.is_some() {
        scene.set_selection(d_array, Selection::Cell(row)).map_err(stringify)?;
        scene
            .set_selection(a, Selection::Row { row, col: Some(col) })
            .map_err(stringify)?;
    }

    // Unstarted (`n` is `None`): the chain still exists, over `A[0, 0]`/`D[0]` — see this function's own doc
    // comment for why that reads better than not drawing it at all.
    let a_value = n.map_or(THETA_C_INPUT[0][0], |n| THETA_C_INPUT[n / 5][n % 5]);
    let d_value = n.map_or(d[0], |n| d[n / 5]);

    // `D[row]` sits centred under `D`; `A[row, col]` sits centred under `A` — each its own column's own working
    // value, directly below the array it came from, rather than sharing a row with the other the way
    // `theta_d::build_scene`'s own `prev`/`next` do (there is no wide shared array feeding both here, so there is
    // no shared row for them to naturally fall into).
    let working_y = d_rect.origin.y + d_rect.size.height.max(a_rect.size.height) + V_GAP;
    let d_cell_x = d_rect.origin.x + (d_rect.size.width - CELL_BOX_WIDTH) / 2.0;
    let a_cell_x = a_rect.origin.x + (a_rect.size.width - CELL_BOX_WIDTH) / 2.0;

    let d_cell = scene
        .add_named_data_node(Point::new(d_cell_x, working_y), &format!("D[{row}]"), hex(d_value))
        .map_err(stringify)?;
    let a_cell = scene
        .add_named_data_node(Point::new(a_cell_x, working_y), &format!("A[{row}, {col}]"), hex(a_value))
        .map_err(stringify)?;

    // Forced to `D`'s/`A`'s own South side and `D[row]`'s/`A[row, col]`'s own North sides: centring alone already
    // makes the automatic ray-cast resolve the same way (see `d_cell_x`'s/`a_cell_x`'s own doc comment above), but
    // forcing it explicitly matches this codebase's own convention of never relying on that implicitly.
    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    scene.add_edge_with(d_array, d_cell, vertical()).map_err(stringify)?;
    scene.add_edge_with(a, a_cell, vertical()).map_err(stringify)?;

    // East of `A[row, col]`, in the same row: with no vertical gap between them, the ray-cast into `XOR` always
    // resolves horizontal regardless of how far east — see `theta_d::build_scene`'s own `MID_Y` doc comment. Both
    // `D[row]` and `A[row, col]` then enter `XOR` from the west, splitting across that one side.
    let xor_x = a_cell_x - CELL_BOX_WIDTH - H_GAP;
    let result = a_value ^ d_value;
    let xor_node = scene
        .add_binary_operator_node(Point::new(xor_x, working_y), BinaryOperator::Xor, (a_cell, d_cell), hex(result))
        .map_err(stringify)?;
    let xor_rect = scene.node_rect(xor_node).map_err(stringify)?;

    // `A'`: the same `[5; [5; u64]]` shape and layout as `A` itself, per request — see this function's own doc
    // comment.
    let output = scene
        .add_named_data_node(
            Point::new(
                d_rect.origin.x + d_rect.size.width + H_GAP,
                xor_rect.origin.y + xor_rect.size.height + V_GAP,
            ),
            "Theta Output",
            DataNodeContent::new(
                NodeValues::U64(display.iter().flatten().copied().collect()),
                DataFormat::Hexadecimal,
            )
            .with_layout(GridLayout::Rows(5)),
        )
        .map_err(stringify)?;
    scene.add_edge(xor_node, output).map_err(stringify)?;
    if n.is_some() {
        scene
            .set_selection(output, Selection::Row { row, col: Some(col) })
            .map_err(stringify)?;
    }

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok((scene, output))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same toolbar as `theta_c::attach_toolbar`, for the nested `XOR loop` child instead — bound to `A'`, which
/// holds 25 values, not 5, so the toolbar itself walks all 25 flat positions; see [`build_scene`]'s own doc
/// comment for why that is exactly the walk this step needs.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
pub(super) fn attach_toolbar(
    child: &Scene,
    output: NodeId,
    n: Option<usize>,
    state: Rc<RefCell<XorLoopState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(output, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            step(&state, transition.to);
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child
            .set_selection(output, Selection::Row { row: n / 5, col: Some(n % 5) })
            .map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same rebuild as `theta_c::rebuild_child`, for the nested `XOR loop` child and [`SCENE`] instead.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the
/// DOM, or if any library call fails.
pub(super) fn rebuild_child(
    to: Option<usize>,
    display: [[u64; 5]; 5],
    state: Rc<RefCell<XorLoopState>>,
) -> Result<(), String> {
    let document = crate::util::document()?;
    let previous_id = state.borrow().child_svg_id.clone();
    let next_id = next_child_svg_id("theta-xorloop-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, output) = build_scene(&next_id, to, display)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, output, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the XOR loop scene was not initialised")?;
        old_child.exit().map_err(stringify)?;
        parent.replace_container_child(node, new_child.clone()).map_err(stringify)?;
        parent.enter(node).map_err(stringify)?;
        *slot = Some((parent, new_child, node));
        Ok(())
    })?;

    required_element(&document, &previous_id)?.remove();
    state.borrow_mut().child_svg_id = next_id;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same `on_step` callback as `theta_c::step`, for the nested `XOR loop` child and [`rebuild_child`] instead —
/// `to` is a flat cell index here, not a row, so `written` is updated at `(to / 5, to % 5)`.
fn step(state: &Rc<RefCell<XorLoopState>>, to: Option<usize>) {
    let mut demo = state.borrow_mut();
    match to {
        None => demo.written = [[false; 5]; 5],
        Some(n) => demo.written[n / 5][n % 5] = true,
    }
    let display = display_outputs(demo.outputs, demo.written);
    drop(demo);
    let _ = rebuild_child(to, display, state.clone());
}
