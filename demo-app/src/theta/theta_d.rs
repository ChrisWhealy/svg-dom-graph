//! The nested `ThetaD` child: SHA3's own `D(x) = C(x-1) ⊕ rotl(C(x+1), 1)` step, its own selection toolbar, and
//! rebuilding it — via `Scene::replace_container_child` — every time that toolbar steps to a new row of `ThetaC`'s
//! own output.

use super::support::{SteppedChildState, create_child_svg, next_child_svg_id};
use crate::util::{required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, ConnectorOptions, DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeOptions,
        NodeValues, Scene, SceneTitleOptions, Selection, SelectionToolbarOptions, Side, ToolbarOptions, UnaryOperator,
    },
};

thread_local! {
    // The same trio as `theta_c`'s own `SCENE` — see its own doc comment.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `ThetaD` child currently grafted in — the same role
/// `theta_c::init_scene` plays for `ThetaC`.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `ThetaD` child if it is currently focused, and reports whether it was — the same role
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
/// `D[n] = C[(n + 4) % 5] ⊕ rotl(C[(n + 1) % 5], 1)` — SHA3's real `ThetaD` step, for the single row currently
/// selected. Once a Keccak lane is represented as a plain `u64`, `rotl` is exactly `u64::rotate_left` — no further
/// byte-order adjustment applies on top of it. Returns `(prev, rotated_next, d_n)`: `prev` and `rotated_next` are
/// `d_n`'s own two real inputs, so [`build_scene`] can label the `prev`/`ROTL` nodes it draws with the exact same
/// values used to compute `d_n`, rather than recomputing either separately.
fn row(c: [u64; 5], n: usize) -> (u64, u64, u64) {
    let prev = c[(n + 4) % 5];
    let next = c[(n + 1) % 5];
    let rotated = next.rotate_left(1);
    (prev, rotated, prev ^ rotated)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `D[n]` for every `n`, via [`row`] — `ThetaD`'s own complete result, computed once up front the same way
/// `crate::selection::theta_c_outputs` computes `ThetaC`'s. `build_theta_demo`/[`step`] use this to know every
/// row's own real value regardless of how far the walk has actually stepped;
/// [`display_outputs`](crate::selection::display_outputs) is what reveals them progressively.
pub(super) fn outputs(c: [u64; 5]) -> [u64; 5] {
    std::array::from_fn(|n| row(c, n).2)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the nested `ThetaD` child for `n`: `C` — `ThetaC`'s own real output,
/// [`crate::selection::theta_c_outputs`], as one `[5; u64]` array, cell `n` selected — sits at the top; `next`,
/// `C[(n + 1) % 5]`, sits directly below it; `prev` (`C[(n + 4) % 5]`), a `ROTL` node rotating `next`, and an `XOR`
/// node combining `prev` and that `ROTL` result into `D[n]` all share one further row below that, left to right, so
/// both of `XOR`'s own inputs approach it from the west rather than `prev` dropping straight down onto it from
/// directly above — see `MID_Y`'s own doc comment for why that matters. `D`, at the bottom, is the same shape as
/// `C`, showing `display`'s own current values with cell `n` focused. `Some(row)` computes and highlights row
/// `row`; `None` — the unstarted state, before row `0` is ever processed — computes the same chain over
/// `prev`/`next` both zero instead, and leaves `C` unselected. See
/// [`theta_c::build_scene`](super::theta_c::build_scene)'s own doc comment for why the chain is always drawn, even
/// unstarted, and why `display`'s own current values are passed in rather than computed here.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing an `<svg id="{svg_id}">`, or if any library call fails.
pub(super) fn build_scene(svg_id: &str, n: Option<usize>, display: [u64; 5]) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Keccak Theta D", SceneTitleOptions::default())
        .map_err(stringify)?;

    let c = crate::selection::theta_c_outputs();
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);

    const LEFT_X: f64 = 20.0;
    const INPUT_Y: f64 = 50.0;
    const PREV_NEXT_Y: f64 = INPUT_Y + 150.0;
    const MID_Y: f64 = PREV_NEXT_Y + 130.0;
    const OUTPUT_Y: f64 = MID_Y + 130.0;

    // 2 fixing points: `C` feeds both `prev` and `next`, so it needs room for two distinct outgoing connectors on
    // the same side, rather than both landing on the same midpoint — see `EdgeAnchors`'s own doc comment.
    let input_options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(2)));
    let input = scene
        .add_named_data_node_with(
            Point::new(LEFT_X, INPUT_Y),
            "Theta C Output Bytes",
            DataNodeContent::new(NodeValues::U64(c.to_vec()), DataFormat::Hexadecimal).with_layout(GridLayout::Rows(1)),
            input_options,
        )
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input).map_err(stringify)?;
    let col1 = input_rect.size.width / 4.0;
    let col2 = input_rect.size.width / 2.0;

    if let Some(n) = n {
        scene.set_selection(input, Selection::Cell(n)).map_err(stringify)?;
    }

    // Unstarted (`n` is `None`): the chain still exists, over a zero `prev`/`next` — see this function's own doc
    // comment for why that reads better than not drawing it at all.
    let (prev, rotated, d_n) = n.map_or((0, 0, 0), |n| row(c, n));
    let next = n.map_or(0, |n| c[(n + 1) % 5]);

    // `prev` sits in `MID_Y`'s own row, level with `ROTL` and `XOR` — see `MID_Y`'s own doc comment for why.
    let prev_node = scene
        .add_named_data_node(Point::new(col1, PREV_NEXT_Y), "prev", hex(prev))
        .map_err(stringify)?;
    let next_node = scene
        .add_named_data_node(Point::new(col2 + 85.0, PREV_NEXT_Y), "next", hex(next))
        .map_err(stringify)?;

    let vertical = || {
        ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North))
    };
    scene.add_edge_with(input, prev_node, vertical()).map_err(stringify)?;
    scene.add_edge_with(input, next_node, vertical()).map_err(stringify)?;

    let rotl_node = scene
        .add_unary_operator_node(
            Point::new(col2 + 85.0, MID_Y),
            UnaryOperator::RotateLeft(1),
            next_node,
            hex(rotated),
        )
        .map_err(stringify)?;
    let xor_node = scene
        .add_binary_operator_node(Point::new(col1, MID_Y), BinaryOperator::Xor, (prev_node, rotl_node), hex(d_n))
        .map_err(stringify)?;

    let output = scene
        .add_named_data_node(
            Point::new(LEFT_X, OUTPUT_Y),
            "Theta D Output",
            DataNodeContent::new(NodeValues::U64(display.to_vec()), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    scene.add_edge(xor_node, output).map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(output, Selection::Cell(n)).map_err(stringify)?;
    }

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok((scene, output))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same toolbar as `theta_c::attach_toolbar`, for the nested `ThetaD` child instead.
///
/// # Errors
///
/// Returns `Err` if showing the toolbar or reapplying the selection fails.
pub(super) fn attach_toolbar(
    child: &Scene,
    output: NodeId,
    n: Option<usize>,
    state: Rc<RefCell<SteppedChildState>>,
) -> Result<(), String> {
    child
        .show_selection_toolbar(output, SelectionToolbarOptions::default(), move |_scene, _node, transition| {
            step(&state, transition.to);
        })
        .map_err(stringify)?;
    if let Some(n) = n {
        child.set_selection(output, Selection::Cell(n)).map_err(stringify)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same rebuild as `theta_c::rebuild_child`, for the nested `ThetaD` child and [`SCENE`] instead — `ThetaC`'s
/// own real output never changes, so only `to` (which row of it `ThetaD` currently looks at) and `display` (which
/// rows of `ThetaD`'s own output have been written so far) vary from one step to the next.
///
/// # Errors
///
/// Returns `Err` if [`SCENE`] was never initialised, if the outgoing child's own `<svg>` is not currently in the
/// DOM, or if any library call fails.
fn rebuild_child(to: Option<usize>, display: [u64; 5], state: Rc<RefCell<SteppedChildState>>) -> Result<(), String> {
    let document = crate::util::document()?;
    let previous_id = state.borrow().child_svg_id.clone();
    let next_id = next_child_svg_id("theta-thetad-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, output) = build_scene(&next_id, to, display)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, output, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the ThetaD scene was not initialised")?;
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
/// The same `on_step` callback as `theta_c::step`, for the nested `ThetaD` child and [`rebuild_child`] instead.
fn step(state: &Rc<RefCell<SteppedChildState>>, to: Option<usize>) {
    let mut demo = state.borrow_mut();
    match to {
        None => demo.written = [false; 5],
        Some(n) => demo.written[n] = true,
    }
    let display = crate::selection::display_outputs(demo.outputs, demo.written);
    drop(demo);
    let _ = rebuild_child(to, display, state.clone());
}
