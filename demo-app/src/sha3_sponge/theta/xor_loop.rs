//! The nested `XOR loop` child: SHA3's own `A'(x,y) = A(x,y) ⊕ D(x)` step, which is the final fold of `Theta` back into
//! the state array. It has its own selection toolbar. It is rebuilt, via `Scene::replace_container_child`, every time
//! that toolbar steps to a new cell of `A`.
//!
//! Unlike `theta_c`/`theta_d`, which each step through one of five *rows*, this one steps through all 25 individual
//! *cells* of `A`. See [`build_scene`]'s own doc comment for why. See [`XorLoopState`]'s own doc comment for why it
//! needs its own state shape rather than sharing `support::SteppedChildState`.

use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
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
/// `SteppedChildState` is shaped `[u64; 5]` throughout — one value per row, since `theta_c`'s own `O` and `theta_d`'s
/// own `D` both really are five-value arrays. `XOR loop`'s own output, `A'`, is a genuine `[5; [5; u64]]` — the same
/// shape as `A` itself — not five values but twenty-five, so it needs its own `[[u64; 5]; 5]` shape instead. Forcing
/// that through `SteppedChildState`'s own fixed `[u64; 5]` field would mean a breaking change to two working,
/// already-tested modules. That would be for the sake of a third that does not actually share their own shape. This
/// crate follows the same reasoning for `toolbar::layout` vs. `selection_toolbar::layout`: a deliberate,
/// separately-tested copy rather than a shared dependency.
pub(super) struct XorLoopState {
    /// `outputs[row][col] = A(row, col) ⊕ D(row)` — every cell's own real result, computed once, up front. Which cells
    /// currently show is derived fresh from these and the walk's own current flat position on every step (see
    /// [`display_outputs`]'s own doc comment). It is not tracked here as a second, separately mutated flag per cell.
    pub(super) outputs: [[u64; 5]; 5],
    /// The `A[x][y]` the whole `Theta` walk runs over — see `SteppedChildState::input`.
    pub(super) input: [[u64; 5]; 5],
    /// The id of whichever `<svg>` currently backs this nested child — see [`rebuild_child`]'s own doc comment for why
    /// every step needs a fresh one.
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
/// `A'(row, col) = A(row, col) ⊕ D(row)` for every cell — `XOR loop`'s own complete result, computed once up front, the
/// same way `theta_d::outputs` computes `ThetaD`'s. [`build_theta_demo`](super::build_theta_demo)/ [`step`] use this to
/// know every cell's own real value regardless of how far the walk has actually stepped; [`display_outputs`] is what
/// reveals them progressively.
pub(super) fn outputs(a: [[u64; 5]; 5], d: [u64; 5]) -> [[u64; 5]; 5] {
    std::array::from_fn(|row| std::array::from_fn(|col| a[row][col] ^ d[row]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `[[u64; 5]; 5]` counterpart to `crate::selection::display_outputs`: cell `(row, col)` of `outputs` shows through
/// for `row * 5 + col <= to.unwrap()` — its own flat, row-major position at or before the walk's current one — `0`
/// everywhere else. `to` is the *only* state this reads: no separately mutated "has this cell ever been written" flag,
/// so "Previous" un-reveals a later cell exactly as it reveals an earlier one. `None` (unstarted, or walked/restarted
/// all the way back) reveals nothing.
pub(super) fn display_outputs(outputs: [[u64; 5]; 5], to: Option<usize>) -> [[u64; 5]; 5] {
    std::array::from_fn(|row| {
        std::array::from_fn(|col| if to.is_some_and(|n| row * 5 + col <= n) { outputs[row][col] } else { 0 })
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested `XOR loop` child for flat cell index `n` (`row = n / 5`, `col = n % 5`), in one row-1/row-2 pair.
/// In row 1, `D` sits at `A`'s own west, at the same height. `D` is `ThetaD`'s own real output
/// ([`super::theta_d::outputs`]), drawn as a `[5; 1]` column with row `row` focused. In row 2, `D[row]` sits centred
/// below `D`, and `A[row, col]` sits centred below `A`. Each is its own single-value node, fed by an edge from the
/// array above it. `XOR`, east of `A[row, col]` in that same row, combines them into `A'(row, col)`. Both of its own
/// inputs then approach it from the west. This is the same `theta_d::build_scene`'s own `MID_Y` reasoning, applied here
/// without a shared source array to put them in one row naturally. `A'`, at the bottom, is the same shape and layout as
/// `A` itself, showing `display`'s own current values with cell `(row, col)` focused.
///
/// Steps cell by cell, not row by row like `theta_c`/`theta_d` do. `A'(row, col)` is a genuinely per-cell result. Every
/// column of a row is `⊕`'d with the *same* `D(row)`, but each still needs its own real `A(row, col)`. So there is no
/// single "row's own result" this could reveal all at once, the way `theta_c`'s `O` and `theta_d`'s `D` can. Binding
/// the toolbar to `A'` itself, a genuine `[5; [5; u64]]` node, gets this for free.
/// `DataNodeContent::flat_index`/`natural_selection` already walk a two-dimensional grid's own values row-major, one
/// cell at a time. That is exactly the walk this step needs.
///
/// `Some(n)` computes the chain over `A[row, col]`/`D[row]`, and focuses that cell in both `A` and `A'`, and row `row`
/// in `D`. `None` is the unstarted state, before cell `0` is ever processed. It computes the same chain over `A[0,
/// 0]`/`D[0]` instead, and leaves `A`/`D`/`A'` unselected. See `theta_c::build_scene`'s own doc comment for why the
/// chain is always drawn, even unstarted, and why `display`'s own current values are passed in rather than computed
/// here.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing an `<svg id="{svg_id}">`, or if any library call fails.
pub(super) fn build_scene(
    svg_id: &str,
    a_input: [[u64; 5]; 5],
    n: Option<usize>,
    display: [[u64; 5]; 5],
) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    crate::util::frame_nested_scene(&document, svg_id)?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Keccak Theta XOR Loop", SceneTitleOptions::default())
        .map_err(stringify)?;

    let d = super::theta_d::outputs(crate::selection::theta_c_outputs(a_input));
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);

    const LEFT_X: f64 = 20.0;
    const TOP_Y: f64 = 50.0;
    // The gap every row transition in this diagram leaves. See `theta_d::build_scene`'s own `PREV_NEXT_Y` doc comment
    // for why a gap this size, not the tighter one an earlier version of that diagram used.
    const V_GAP: f64 = 58.0;
    // The horizontal counterpart, between `D`/`A` and between `A[row, col]`/`XOR`. It is narrower, since these sit side
    // by side rather than stacked, and have no label row of their own competing for the same space.
    const H_GAP: f64 = 40.0;

    // `D`, not `A`, sits first, to `A`'s own west. Both are in one row, and both are the same height (each a five-row
    // grid, one column or five), so their tops and bottoms line up exactly.
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
                NodeValues::U64(a_input.iter().flatten().copied().collect()),
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

    // Unstarted (`n` is `None`): the chain still exists, over `A[0, 0]`/`D[0]` — see this function's own doc comment
    // for why that reads better than not drawing it at all.
    let a_value = n.map_or(a_input[0][0], |n| a_input[n / 5][n % 5]);
    let d_value = n.map_or(d[0], |n| d[n / 5]);

    // `D[row]` sits centred under `D`, and `A[row, col]` sits centred under `A`. Each is its own column's own working
    // value, directly below the array it came from. They do not share a row, unlike `theta_d::build_scene`'s own
    // `prev`/`next`. There is no wide shared array feeding both here, so there is no shared row for them to naturally
    // fall into. Centred using each box's own real measured width (`Scene::measure_named_data_node`) rather than an
    // estimated constant. That method exists to make this exact problem go away: "I don't know a node's own size until
    // it's drawn, but need it to position this one." It replaces the add-then-estimate-then-correct cycle an earlier
    // version of this file used.
    let working_y = d_rect.origin.y + d_rect.size.height.max(a_rect.size.height) + V_GAP;

    let d_label = format!("D[{row}]");
    let d_content = hex(d_value);
    let d_size = scene.measure_named_data_node(&d_label, &d_content).map_err(stringify)?;
    let d_cell_x = d_rect.origin.x + (d_rect.size.width - d_size.width) / 2.0;

    let a_label = format!("A[{row}, {col}]");
    let a_content = hex(a_value);
    let a_size = scene.measure_named_data_node(&a_label, &a_content).map_err(stringify)?;
    let a_cell_x = a_rect.origin.x + (a_rect.size.width - a_size.width) / 2.0;

    let d_cell = scene
        .add_named_data_node(Point::new(d_cell_x, working_y), &d_label, d_content)
        .map_err(stringify)?;
    let a_cell = scene
        .add_named_data_node(Point::new(a_cell_x, working_y), &a_label, a_content)
        .map_err(stringify)?;

    // Forced to `D`'s/`A`'s own South side and `D[row]`'s/`A[row, col]`'s own North sides. Centring alone already makes
    // the automatic ray-cast resolve the same way (see `d_cell_x`'s/`a_cell_x`'s own doc comment above). Forcing it
    // explicitly matches this codebase's own convention of never relying on that implicitly.
    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    scene.add_edge_with(d_array, d_cell, vertical()).map_err(stringify)?;
    scene.add_edge_with(a, a_cell, vertical()).map_err(stringify)?;

    // West of `A[row, col]`, in the same row. With no vertical gap between them, the ray-cast into `XOR` always
    // resolves horizontal regardless of how far west (see `theta_d::build_scene`'s own `MID_Y` doc comment). Both
    // `D[row]` and `A[row, col]` then enter `XOR` from the east, splitting across that one side. `xor_x` is measured,
    // not estimated — see `d_cell_x`'s own comment above for why.
    let result = a_value ^ d_value;
    let result_content = hex(result);
    let xor_size = scene.measure_operator_box("XOR", &result_content).map_err(stringify)?;
    let xor_x = a_cell_x - xor_size.width - H_GAP;
    let xor_node = scene
        .add_binary_operator_node(
            Point::new(xor_x, working_y),
            BinaryOperator::Xor,
            (a_cell, d_cell),
            result_content,
        )
        .map_err(stringify)?;
    let xor_rect = scene.node_rect(xor_node).map_err(stringify)?;

    // `A'`: the same `[5; [5; u64]]` shape and layout as `A` itself, per request — see this function's own doc comment.
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

    let right = [d_array, a, a_cell, d_cell, xor_node, output]
        .into_iter()
        .map(|id| scene.node_rect(id).map(|r| r.origin.x + r.size.width))
        .collect::<Result<Vec<_>, _>>()
        .map_err(stringify)?
        .into_iter()
        .fold(0.0, f64::max);
    let output_rect = scene.node_rect(output).map_err(stringify)?;
    crate::util::fit_nested_size(&scene, svg_id, right, output_rect.origin.y + output_rect.size.height, true)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok((scene, output))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same toolbar as `theta_c::attach_toolbar`, for the nested `XOR loop` child instead. It is bound to `A'`, which
/// holds 25 values, not 5, so the toolbar itself walks all 25 flat positions. See [`build_scene`]'s own doc comment for
/// why that is exactly the walk this step needs.
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
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the DOM,
/// or if any library call fails.
pub(super) fn rebuild_child(
    to: Option<usize>,
    display: [[u64; 5]; 5],
    state: Rc<RefCell<XorLoopState>>,
) -> Result<(), String> {
    let document = crate::util::document()?;
    let (input, previous_id) = {
        let state = state.borrow();
        (state.input, state.child_svg_id.clone())
    };
    let next_id = next_child_svg_id("theta-xorloop-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, output) = build_scene(&next_id, input, to, display)?;
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
/// The same `on_step` callback as `theta_c::step`, for the nested `XOR loop` child and [`rebuild_child`] instead — `to`
/// is a flat cell index here, not a row.
fn step(state: &Rc<RefCell<XorLoopState>>, to: Option<usize>) {
    let demo = state.borrow();
    let display = display_outputs(demo.outputs, to);
    drop(demo);
    crate::sha3_sponge::report_step(rebuild_child(to, display, state.clone()));
}
