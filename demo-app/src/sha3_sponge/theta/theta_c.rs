//! The nested `ThetaC` child: [`build_scene`] itself, attaching its own selection toolbar, and rebuilding it — via
//! `Scene::replace_container_child` — every time that toolbar steps to a new row.
//!
//! [`build_scene`] is shared with the standalone Cell Selection demo's own third example
//! (`crate::selection::rebuild_theta_c_diagram`), which draws exactly the same chain against
//! `#selection-thetac-diagram` instead of a nested child `<svg>`.

use super::support::SteppedChildState;
use crate::selection::THETA_C_INPUT;
use crate::util::{create_child_svg, next_child_svg_id, required_element, stringify};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::{
    NodeId,
    scene::{
        BinaryOperator, DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeOptions, NodeValues, Scene,
        SceneTitleOptions, Selection, SelectionToolbarOptions, Side, ToolbarOptions,
    },
};

thread_local! {
    // The parent Scene, the nested ThetaC child Scene, and the container NodeId that owns it — kept alive for the
    // page's own lifetime. `enter`/`exit` are driven from this same trio: `parent.enter(node)` to descend,
    // `child.exit()` to return, with no further state to track — `Scene::is_focused` already answers "which one
    // is active right now" without this module keeping a duplicate copy of it. The child itself is replaced
    // wholesale on every step — see [`rebuild_child`]'s own doc comment.
    static SCENE: RefCell<Option<(Scene, Scene, NodeId)>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Records `parent`/`child`/`node` as the nested `ThetaC` child currently grafted in — the one [`exit_if_focused`]
/// and [`rebuild_child`] act on from then on. Called once, from `build_theta_demo`, right after `child` is grafted
/// into `parent` under `node`.
pub(super) fn init_scene(parent: Scene, child: Scene, node: NodeId) {
    SCENE.with_borrow_mut(|slot| *slot = Some((parent, child, node)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exits the nested `ThetaC` child if it is currently focused, and reports whether it was — what
/// `wire_theta_controls`'s own close button needs to check before trying `ThetaD`'s.
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
/// Rebuilds `svg_id` from scratch, for `n`: a fresh five-operand, four-`XOR` chain, an input array, an output
/// array, and a plain edge from the chain's own final `XOR` node into the output array. `Some(row)` computes the
/// chain over `THETA_C_INPUT[row]` and bands that row in the input array; `None` — the unstarted state, before row
/// `0` is ever processed — computes the same chain over five zero operands instead, and leaves the input array
/// unbanded. The chain is always drawn, even unstarted: showing it with every value at zero, rather than not
/// drawing it at all, is what keeps it reading as "not yet run" instead of "does not exist until iteration
/// starts." Either way, the output array always shows `display`'s own current values, with cell `row` focused for
/// `Some(row)`.
///
/// `svg-dom-graph` has no way to change a node's own displayed value once drawn — only its selection (see
/// `Scene::set_selection`'s own doc comment). Since every value here — the row currently feeding the chain, the
/// chain's own intermediate results, and however many output cells have so far been "written" — changes on every
/// step, there is no existing node any of this could update in place. So, exactly like
/// `edge_anchors::rebuild_edge_anchors_scene`, each step clears `svg_id`'s own children and draws everything
/// again, fresh, from this row's own real values.
///
/// `O` is its own `[5; u64]` node, not folded into `A`'s own `[5; [5; u64]]` shape — each keeps the type its own
/// values actually have. `A` sits at the very top of the canvas, above the chain it feeds; `O` sits right below
/// the chain's own final `XOR` node, so a connector from there reaches `O` directly, with nothing else in the way
/// — a connector can only land on a node's own outer perimeter, never a specific cell inside it, and `O` is the
/// node whose perimeter that connector actually reaches.
///
/// Each of the five operand boxes is named after the exact element of `A` it holds — `A[row, i]` — for `Some(row)`.
/// Unstarted, there is no real row to name any of them after, so they are drawn unnamed instead — see the `place`
/// closure's own comment, inside this function, for why that is the closest this library can get to leaving the
/// label blank.
///
/// Returns `O`'s own [`NodeId`] alongside the `Scene`, so a caller can attach its own selection toolbar to it —
/// see [`attach_toolbar`]'s own doc comment for why that toolbar must be attached fresh on every rebuild rather
/// than kept across them.
///
/// Shared by this module's own [`rebuild_child`] (grafted into a nested child `<svg>` via
/// `Scene::replace_container_child`) and the standalone Cell Selection demo's own
/// `crate::selection::rebuild_theta_c_diagram` (attached to `#selection-thetac-diagram` directly) — both draw
/// exactly the same chain from exactly the same code, over `svg_id` rather than a hardcoded element id.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing an `<svg id="{svg_id}">`, or if any library call fails.
pub(crate) fn build_scene(svg_id: &str, n: Option<usize>, display: [u64; 5]) -> Result<(Scene, NodeId), String> {
    let document = crate::util::document()?;
    crate::util::frame_nested_scene(&document, svg_id)?;
    let container = required_element(&document, svg_id)?;
    container.set_inner_html("");
    let svg = svg_dom::SvgRoot::attach(svg_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    scene
        .show_scene_title("Keccak Theta C", SceneTitleOptions::default())
        .map_err(stringify)?;

    // The input array: `A`'s own 25 values, its own `[5; [5; u64]]` node, at the top of the canvas — `TOP_Y`
    // (50, not `0`) leaves the scene title's own band (`margin` 12 plus its own ≈28-unit-tall text, an estimate
    // as usual with no browser here to measure it) clear above it, rather than overlapping it.
    const TOP_Y: f64 = 50.0;
    let array = scene
        .add_named_data_node(
            Point::new(20.0, TOP_Y),
            "A Bytes",
            DataNodeContent::new(
                NodeValues::U64(THETA_C_INPUT.iter().flatten().copied().collect()),
                DataFormat::Hexadecimal,
            )
            .with_layout(GridLayout::Rows(5)),
        )
        .map_err(stringify)?;
    if let Some(n) = n {
        scene
            .set_selection(array, Selection::Row { row: n, col: None })
            .map_err(stringify)?;
    }

    // Unstarted (`n` is `None`): the chain still exists, over five zero operands — see this function's own doc
    // comment for why that reads better than not drawing it at all.
    let row = n.map_or([0u64; 5], |n| THETA_C_INPUT[n]);
    let hex = |value: u64| DataNodeContent::new(NodeValues::U64(vec![value]), DataFormat::Hexadecimal);

    // Labels operand `i` with the exact element of `A` it holds — `A[row, i]` — once a real row is selected.
    // Unstarted (`n` is `None`), there is no real row to name any operand after, and `add_named_data_node` itself
    // rejects an empty name outright, so this falls back to a plain, unnamed box instead: the closest this library
    // can get to "no label," short of a blank string it would refuse to draw at all.
    let operand_label = |i: usize| match n {
        Some(row) => format!("A[{row}, {i}]"),
        None => "A[-, -]".to_string(),
    };
    let place = |x: f64, y: f64, i: usize, value: u64| -> Result<NodeId, String> {
        scene
            .add_named_data_node(Point::new(x, y), &operand_label(i), hex(value))
            .map_err(stringify)
    };

    // The five elements of the current row sit in one horizontal row below the input array, in the same
    // left-to-right order as `row`'s own values there — so this canvas reads as "here is that row, unpacked."
    //
    // Every operand is a named box (`A[row, i]`, or the placeholder `A[-, -]` unstarted). Every label is the same
    // length (`A[row, i]`'s own `row`/`i` are always single digits, `0..=4`, and the placeholder is the same
    // seven characters), and every value is a full `u64` hex cell, so measuring just the first operand's own box
    // — `Scene::measure_named_data_node` — gives the real width every one of the five actually needs, not an
    // estimate of it. `OPERAND_GAP` is the only reasoned number left in this stride: a small, steady visual gap
    // between adjacent boxes, not a measured one.
    //
    // Each `XOR` stage cascades down and to the left of the operand row, combining the running total with the
    // next operand to its own right; every stage's own `x` sits strictly left of the operand it still has to
    // reach, and every operand's own connector drops straight down its own column before turning, so nothing
    // here ever crosses an earlier stage's box.
    const OPERAND_GAP: f64 = 4.0;
    let operand_width = scene
        .measure_named_data_node(&operand_label(0), &hex(row[0]))
        .map_err(stringify)?
        .width;
    let operand_stride = operand_width + OPERAND_GAP;
    let operand_x: [f64; 5] = std::array::from_fn(|i| 20.0 + i as f64 * operand_stride);
    // `A Bytes`'s own bottom edge sits at `TOP_Y + 236.6` (a named 5×5 grid's own fixed height) — `OPERAND_Y` was
    // once `TOP_Y + 255`, an ≈18-unit gap that turned out too tight once really rendered: measured text is never
    // exactly the estimate this file reasons from (see this function's own doc comment), so a gap this thin had no
    // slack to absorb the difference before the two boxes touched. `+ 295` leaves the same ≈58-unit gap every
    // later stage in this chain already uses.
    const OPERAND_Y: f64 = TOP_Y + 295.0;

    // `t1` sits close enough below the operand row that the gap between them — the operand box's own bottom edge
    // (71.8 units tall, now that every operand is named — see `place`'s own comment) to `t1`'s own top — leaves
    // the same ≈58-unit clearance `OPERAND_Y`'s own comment gives for `A Bytes` → the operand row.
    let op0 = place(operand_x[0], OPERAND_Y, 0, row[0])?;
    let op1 = place(operand_x[1], OPERAND_Y, 1, row[1])?;
    let xor01 = row[0] ^ row[1];
    let t1 = scene
        .add_binary_operator_node(Point::new(120.0, TOP_Y + 405.0), BinaryOperator::Xor, (op0, op1), hex(xor01))
        .map_err(stringify)?;

    // Each later stage sits 112 units below the previous one: an operator node's own height (71.8 units, again
    // a fixed constant) plus a ≈40-unit gap between them, per request.
    let op2 = place(operand_x[2], OPERAND_Y, 2, row[2])?;
    let xor012 = xor01 ^ row[2];
    let t2 = scene
        .add_binary_operator_node(Point::new(270.0, TOP_Y + 507.0), BinaryOperator::Xor, (t1, op2), hex(xor012))
        .map_err(stringify)?;

    let op3 = place(operand_x[3], OPERAND_Y, 3, row[3])?;
    let xor0123 = xor012 ^ row[3];
    let t3 = scene
        .add_binary_operator_node(Point::new(445.0, TOP_Y + 609.0), BinaryOperator::Xor, (t2, op3), hex(xor0123))
        .map_err(stringify)?;

    // The final stage always sits at the same position — the one it would occupy for `n == 3` — rather than
    // tracking column `n` the way earlier attempts here did. Nothing else occupies the space between it and
    // the output array below, so moving it was never necessary for the connector's own safety; it just moved
    // without a reason to.
    let op4 = place(operand_x[4], OPERAND_Y, 4, row[4])?;
    let xor01234 = xor0123 ^ row[4];
    let result = scene
        .add_binary_operator_node(
            Point::new(operand_x[3], TOP_Y + 711.0),
            BinaryOperator::Xor,
            (t3, op4),
            hex(xor01234),
        )
        .map_err(stringify)?;

    // The output array: `O`'s own five values, its own `[5; u64]` node — see this function's own doc comment for
    // why it stays distinct from `A` rather than folding into `A`'s own shape. Five fixing points on every side,
    // so the connector below can snap to whichever of them best approximates column `n`, rather than landing
    // wherever a single, unconfigured anchor would pick.
    let output_options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(1)));
    let output = scene
        .add_named_data_node_with(
            Point::new(20.0, TOP_Y + 823.0),
            "Theta C Output",
            DataNodeContent::new(NodeValues::U64(display.to_vec()), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Rows(1)),
            output_options,
        )
        .map_err(stringify)?;
    if let Some(n) = n {
        scene.set_selection(output, Selection::Cell(n)).map_err(stringify)?;
    }

    scene.add_edge(result, output).map_err(stringify)?;
    let right = [array, op4, result, output]
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
/// Shows a selection toolbar on `child`, bound to `output`, driving the nested walk's *next* step — the nested
/// counterpart to `crate::selection::rebuild_theta_c_diagram`'s own toolbar. Reapplies `Selection::Cell(n)`
/// afterward for the same reason that function's own doc comment gives: `show_selection_toolbar` always resets to
/// unstarted first.
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
/// Rebuilds the nested `ThetaC` child for `to`, and grafts it into [`SCENE`]'s own `parent` in place of whichever
/// child is currently shown — the nested counterpart to `crate::selection::rebuild_theta_c_diagram`. [`build_scene`]
/// has no way to update an already-drawn chain in place (see its own doc comment), so each step needs a genuinely
/// fresh `Scene`; grafting it in is exactly what `Scene::replace_container_child` is for.
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
pub(super) fn rebuild_child(
    to: Option<usize>,
    display: [u64; 5],
    state: Rc<RefCell<SteppedChildState>>,
) -> Result<(), String> {
    let document = crate::util::document()?;
    let previous_id = state.borrow().child_svg_id.clone();
    let next_id = next_child_svg_id("theta-thetac-child");
    create_child_svg(&document, &previous_id, &next_id)?;

    let view = SCENE.with_borrow(|slot| slot.as_ref().map(|(_, child, _)| child.view()));

    let (new_child, output) = build_scene(&next_id, to, display)?;
    if let Some(view) = view {
        new_child.set_view(view).map_err(stringify)?;
    }
    attach_toolbar(&new_child, output, to, state.clone())?;

    SCENE.with_borrow_mut(|slot| -> Result<(), String> {
        let (parent, old_child, node) = slot.take().ok_or("the ThetaC scene was not initialised")?;
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
/// [`Scene::show_selection_toolbar`](svg_dom_graph::scene::Scene::show_selection_toolbar)'s own `on_step` callback
/// for the nested view — the nested counterpart to `crate::selection::step_theta_c`. Rebuilds the nested child for
/// the walk's new position `to`, via [`rebuild_child`].
///
/// A [`rebuild_child`] failure here is ignored, the same "cannot fail in practice, and nowhere to report it to"
/// reasoning `crate::selection::step_theta_c` already follows.
fn step(state: &Rc<RefCell<SteppedChildState>>, to: Option<usize>) {
    let demo = state.borrow();
    let display = crate::selection::display_outputs(demo.outputs, to);
    drop(demo);
    let _ = rebuild_child(to, display, state.clone());
}
