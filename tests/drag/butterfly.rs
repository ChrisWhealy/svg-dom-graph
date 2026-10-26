//! The butterfly: a multiplication, an addition and a subtraction drawn inside one draggable frame.

use crate::common::{
    FailingWrites, check, connector_count, dispatch_pointer_event, group_translate, last_point_of_path, make_svg,
    nth_connector, nth_group, path_d,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    Error, NodeId,
    complex::Complex,
    scene::{Butterfly, ButterflyOperands, ButterflyResults, DataFormat, DataNodeContent, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

fn e(error: Error) -> String {
    error.to_string()
}

fn number(re: f64, im: f64) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Complex64(vec![Complex::new(re, im)]), DataFormat::Decimal)
}

/// A scene holding `a`, `b` and the twiddle factor, in that order.
fn scene_with_operands(id: &str) -> Result<(Scene, ButterflyOperands), String> {
    let scene = Scene::new(make_svg(id, Size::new(900.0, 600.0), Size::new(900.0, 600.0))).map_err(e)?;
    let add = |at: Point, re: f64, im: f64| scene.add_data_node(at, number(re, im)).map_err(e);
    let a = add(Point::new(20.0, 20.0), 1.0, 2.0)?;
    let b = add(Point::new(20.0, 200.0), 3.0, 4.0)?;
    let twiddle = add(Point::new(20.0, 380.0), 0.0, -1.0)?;
    Ok((scene, ButterflyOperands::new(a, b, twiddle)))
}

/// `w·b = 4 - 3i`, so the sum is `5 - i` and the difference `-3 + 5i`.
fn results() -> ButterflyResults {
    ButterflyResults::new(number(4.0, -3.0), number(5.0, -1.0), number(-3.0, 5.0))
}

fn add_butterfly(id: &str) -> Result<(Scene, Butterfly), String> {
    let (scene, operands) = scene_with_operands(id)?;
    let butterfly = scene
        .add_butterfly_node(Point::new(300.0, 100.0), operands, results())
        .map_err(e)?;
    Ok((scene, butterfly))
}

fn inside(outer: Rect, inner: Rect) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.size.width <= outer.origin.x + outer.size.width
        && inner.origin.y + inner.size.height <= outer.origin.y + outer.size.height
}

fn origin_of(scene: &Scene, id: NodeId) -> Result<Point, String> {
    Ok(scene.node_rect(id).map_err(e)?.origin)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_butterfly_draws_a_frame_holding_a_multiplication_an_addition_and_a_subtraction() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-draw")?;
    let frame = scene.node_rect(butterfly.frame).map_err(e)?;
    for member in [butterfly.product, butterfly.sum, butterfly.difference] {
        check(
            inside(frame, scene.node_rect(member).map_err(e)?),
            "a member sticks out of the frame",
        )?;
    }
    // The three operands, then the frame, the three ports and the three operator nodes, in the order they were added.
    let labels = ["Butterfly", "A", "B", "W", "MUL", "ADD", "SUB"];
    for (i, label) in labels.iter().enumerate() {
        let group = nth_group("bf-draw", 3 + u32::try_from(i).map_err(|e| e.to_string())?)?;
        let text = group.text_content().unwrap_or_default();
        check(text.contains(label), &format!("group {} is {text:?}, expected {label}", 3 + i))?;
    }
    Ok(())
}

#[wasm_bindgen_test]
fn a_butterfly_wires_the_multiplication_to_the_twiddle_and_b_and_the_others_to_a_and_the_product() -> Result<(), String>
{
    let (_scene, _butterfly) = add_butterfly("bf-wires")?;
    // Three from the operands to the ports, two into MUL, and two each into ADD and SUB.
    check(connector_count("bf-wires")? == 9, "expected nine connectors")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The addition is in the first row, the multiplication and the subtraction are in the second, and the multiplication is
/// left of both.
#[wasm_bindgen_test]
fn the_operator_nodes_are_in_two_rows_and_two_columns() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-grid")?;
    let (product, sum, difference) = (
        scene.node_rect(butterfly.product).map_err(e)?,
        scene.node_rect(butterfly.sum).map_err(e)?,
        scene.node_rect(butterfly.difference).map_err(e)?,
    );
    check(
        sum.origin.y + sum.size.height < product.origin.y,
        "the addition is not above the multiplication",
    )?;
    check(
        sum.origin.y + sum.size.height < difference.origin.y,
        "the addition is not above the subtraction",
    )?;
    check(
        product.origin.x + product.size.width < sum.origin.x,
        "the multiplication is not left of the addition",
    )?;
    check(
        (sum.origin.x - difference.origin.x).abs() < 0.01,
        "the addition and subtraction are not in a column",
    )
}

/// The product runs horizontally from the middle of the multiplication's east edge into the subtraction's second input,
/// and the elbow from `A` into the subtraction's first input goes round the multiplication and not through it.
#[wasm_bindgen_test]
fn the_product_runs_level_into_the_subtraction_and_a_does_not_run_through_the_multiplication() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-level")?;
    let product = scene.node_rect(butterfly.product).map_err(e)?;
    let numbers_of =
        |d: &str| -> Vec<f64> { d.split_whitespace().filter_map(|token| token.parse::<f64>().ok()).collect() };
    // Edge 8 runs from the multiplication into the subtraction.
    let d = path_d(&nth_connector("bf-level", 8)?)?;
    let numbers = numbers_of(&d);
    check(numbers.len() == 4, &format!("{d:?} is not one straight segment"))?;
    check((numbers[1] - numbers[3]).abs() < 0.01, &format!("{d:?} is not horizontal"))?;
    check(
        (numbers[0] - (product.origin.x + product.size.width)).abs() < 0.01,
        &format!("{d:?} does not leave the east edge of the multiplication"),
    )?;
    check(
        (numbers[1] - (product.origin.y + product.size.height / 2.0)).abs() < 0.01,
        &format!("{d:?} does not leave from the middle of the east edge"),
    )?;

    // Edge 7 is the elbow from `A`. No segment of it may pass through the inside of the multiplication.
    let d = path_d(&nth_connector("bf-level", 7)?)?;
    let points: Vec<(f64, f64)> = numbers_of(&d).chunks(2).map(|pair| (pair[0], pair[1])).collect();
    check(points.len() >= 4, &format!("{d:?} is not an elbow with two bends"))?;
    let (left, right) = (product.origin.x, product.origin.x + product.size.width);
    let (top, bottom) = (product.origin.y, product.origin.y + product.size.height);
    // Its descender is half way across the gap between the multiplication and the subtraction.
    let subtraction = scene.node_rect(butterfly.difference).map_err(e)?;
    let middle = (right + subtraction.origin.x) / 2.0;
    check(
        points
            .windows(2)
            .any(|pair| (pair[0].0 - middle).abs() < 0.01 && (pair[1].0 - middle).abs() < 0.01),
        &format!("{d:?} has no vertical run at {middle}"),
    )?;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let crosses_x = a.0.min(b.0) < right - 0.01 && a.0.max(b.0) > left + 0.01;
        let crosses_y = a.1.min(b.1) < bottom - 0.01 && a.1.max(b.1) > top + 0.01;
        check(!(crosses_x && crosses_y), &format!("{d:?} runs through the multiplication"))?;
    }
    Ok(())
}

/// Every input of every operator node arrives on its west edge, at two different points. Every connector is a straight line
/// but two: the product into the addition, and `A` into the subtraction, which are elbows.
#[wasm_bindgen_test]
fn every_operator_takes_both_inputs_on_its_west_side_and_two_connectors_are_elbows() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-inputs")?;
    check(connector_count("bf-inputs")? == 9, "nine connectors")?;
    // Edges are added in order: three into the ports, then two into the product, two into the sum and two into the
    // difference. The elbows are the second into the sum (the product) and the first into the difference (`A`).
    let elbows = [6_usize, 7];
    for (pair, operator) in [butterfly.product, butterfly.sum, butterfly.difference].into_iter().enumerate() {
        let rect = scene.node_rect(operator).map_err(e)?;
        let mut ends = Vec::new();
        for edge in [3 + 2 * pair, 4 + 2 * pair] {
            let d = path_d(&nth_connector("bf-inputs", u32::try_from(edge).map_err(|e| e.to_string())?)?)?;
            if elbows.contains(&edge) {
                check(is_elbow(&d)?, &format!("{d:?} is not an elbow"))?;
            } else {
                check(
                    d.starts_with('M') && d.matches('L').count() == 1 && !d.contains('A'),
                    &format!("{d:?} is not straight"),
                )?;
            }
            let (x, y) = last_point_of_path(&d)?;
            check(
                (x - rect.origin.x).abs() < 0.01,
                &format!("{d:?} does not end on the west edge"),
            )?;
            check(
                y > rect.origin.y && y < rect.origin.y + rect.size.height,
                "the end is off the edge",
            )?;
            ends.push(y);
        }
        check((ends[0] - ends[1]).abs() > 5.0, "the two inputs share a point")?;
    }
    Ok(())
}

/// `true` if the path `d` has at least two segments, and each runs along one axis.
fn is_elbow(d: &str) -> Result<bool, String> {
    let numbers: Vec<f64> = d
        .split_whitespace()
        .filter(|token| token.parse::<f64>().is_ok())
        .map(|token| token.parse::<f64>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let points: Vec<(f64, f64)> = numbers.chunks(2).map(|pair| (pair[0], pair[1])).collect();
    Ok(points.len() >= 3
        && points
            .windows(2)
            .all(|pair| (pair[0].0 - pair[1].0).abs() < 0.01 || (pair[0].1 - pair[1].1).abs() < 0.01))
}

/// The output of the addition and of the subtraction leaves from the middle of its east edge.
#[wasm_bindgen_test]
fn the_addition_and_the_subtraction_output_from_the_middle_of_their_east_edges() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-outputs")?;
    let before = connector_count("bf-outputs")?;
    for (i, operator) in [butterfly.sum, butterfly.difference].into_iter().enumerate() {
        let rect = scene.node_rect(operator).map_err(e)?;
        let out = scene
            .add_data_node(
                Point::new(rect.origin.x + 300.0, rect.origin.y + 20.0 * i as f64),
                number(0.0, 0.0),
            )
            .map_err(e)?;
        scene.add_edge(operator, out).map_err(e)?;
        let d = path_d(&nth_connector(
            "bf-outputs",
            before + u32::try_from(i).map_err(|e| e.to_string())?,
        )?)?;
        let start: Vec<f64> = d
            .split_whitespace()
            .skip(1)
            .take(2)
            .map(|t| t.parse().unwrap_or(f64::NAN))
            .collect();
        check(
            (start[0] - (rect.origin.x + rect.size.width)).abs() < 0.01,
            &format!("{d:?} does not leave the east edge"),
        )?;
        check(
            (start[1] - (rect.origin.y + rect.size.height / 2.0)).abs() < 0.01,
            &format!("{d:?} is not from the middle"),
        )?;
    }
    Ok(())
}

/// The ports are small circles that do not overlap each other or the multiplication.
#[wasm_bindgen_test]
fn the_ports_are_small_and_overlap_nothing() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-small")?;
    let rects: Vec<Rect> = [butterfly.port_a, butterfly.port_b, butterfly.port_w]
        .iter()
        .map(|&id| scene.node_rect(id).map_err(e))
        .collect::<Result<_, _>>()?;
    // A letter is a little wider in one browser than another, so a circle is not the same size in all of them. None is
    // more than 20 across.
    check(
        rects.iter().all(|r| r.size.width <= 20.0),
        &format!("the ports are not small: {rects:?}"),
    )?;
    let overlap = |a: Rect, b: Rect| {
        a.origin.x < b.origin.x + b.size.width
            && b.origin.x < a.origin.x + a.size.width
            && a.origin.y < b.origin.y + b.size.height
            && b.origin.y < a.origin.y + a.size.height
    };
    let product = scene.node_rect(butterfly.product).map_err(e)?;
    for (i, a) in rects.iter().enumerate() {
        check(!overlap(*a, product), "a port overlaps the multiplication")?;
        for b in &rects[i + 1..] {
            check(!overlap(*a, *b), "two ports overlap")?;
        }
    }
    Ok(())
}

/// The frame has an input port on its west edge for each of `A`, `B` and `W`, in that order. Each is a circle with its own
/// connector from `a`, `b` or the twiddle factor ending at it, and the connectors inside the frame start at them.
#[wasm_bindgen_test]
fn the_frame_has_three_input_ports_that_are_fixing_points_for_the_connectors() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-ports")?;
    let frame = scene.node_rect(butterfly.frame).map_err(e)?;
    let ports = [butterfly.port_a, butterfly.port_b, butterfly.port_w];
    let rects: Vec<Rect> = ports
        .iter()
        .map(|&id| scene.node_rect(id).map_err(e))
        .collect::<Result<_, _>>()?;
    let centre = |r: Rect| Point::new(r.origin.x + r.size.width / 2.0, r.origin.y + r.size.height / 2.0);
    // The ports are centred on the west edge of the drawn box, so their centres are level with one another. The frame's
    // rectangle reaches west far enough to hold every one whole, which keeps them from being clipped at the edge of the
    // visible area when the frame is dragged to it.
    for r in &rects {
        check(
            (centre(*r).x - centre(rects[0]).x).abs() < 0.01,
            "the ports are not in one column",
        )?;
        check(
            r.origin.x >= frame.origin.x - 0.01
                && r.origin.y >= frame.origin.y
                && r.origin.y + r.size.height <= frame.origin.y + frame.size.height,
            &format!("a port {r:?} sticks out of the frame's rectangle {frame:?}"),
        )?;
    }
    check(
        centre(rects[0]).y < centre(rects[1]).y && centre(rects[1]).y < centre(rects[2]).y,
        "not in the order A, B, W",
    )?;
    let drawn = nth_group("bf-ports", 4)?.text_content().unwrap_or_default();
    check(drawn.contains('A'), &format!("the first port says {drawn:?}"))?;

    // Connectors 0 to 2 run from `a`, `b` and the twiddle factor, and each ends on its own port.
    for (edge, rect) in rects.iter().enumerate() {
        let d = path_d(&nth_connector("bf-ports", u32::try_from(edge).map_err(|e| e.to_string())?)?)?;
        let (x, y) = last_point_of_path(&d)?;
        let (cx, cy) = (centre(*rect).x, centre(*rect).y);
        let distance = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        check(
            (distance - rect.size.width / 2.0).abs() < 0.5,
            &format!("edge {edge} ends {distance} from the centre"),
        )?;
    }
    // The first connector into each operator node starts at a port: `A` for the addition and the subtraction, `B` and `W`
    // for the multiplication.
    let starts_at = |edge: u32, port: Rect| -> Result<bool, String> {
        let d = path_d(&nth_connector("bf-ports", edge)?)?;
        let first = d
            .split_whitespace()
            .skip(1)
            .take(2)
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let (cx, cy) = (centre(port).x, centre(port).y);
        Ok((((first[0] - cx).powi(2) + (first[1] - cy).powi(2)).sqrt() - port.size.width / 2.0).abs() < 0.5)
    };
    check(
        starts_at(5, rects[0])? && starts_at(7, rects[0])?,
        "the addition and the subtraction do not start at A",
    )?;
    check(
        starts_at(3, rects[2])? || starts_at(4, rects[2])?,
        "the multiplication does not start at W",
    )?;
    check(
        starts_at(3, rects[1])? || starts_at(4, rects[1])?,
        "the multiplication does not start at B",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn dragging_the_frame_carries_the_three_operator_nodes_by_the_same_amount() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-drag")?;
    let members = [
        butterfly.port_a,
        butterfly.port_b,
        butterfly.port_w,
        butterfly.product,
        butterfly.sum,
        butterfly.difference,
    ];
    let before: Vec<Point> = members.iter().map(|&id| origin_of(&scene, id)).collect::<Result<_, _>>()?;
    let frame_before = origin_of(&scene, butterfly.frame)?;

    let frame_group = nth_group("bf-drag", 3)?;
    let (x, y) = (320, 120);
    dispatch_pointer_event(&frame_group, "pointerdown", x, y, 1)?;
    dispatch_pointer_event(&frame_group, "pointermove", x + 50, y + 30, 1)?;
    dispatch_pointer_event(&frame_group, "pointerup", x + 50, y + 30, 1)?;

    let frame_after = origin_of(&scene, butterfly.frame)?;
    let moved = (frame_after.x - frame_before.x, frame_after.y - frame_before.y);
    check(moved.0.abs() > 1.0 || moved.1.abs() > 1.0, "the frame did not move")?;
    for (i, &id) in members.iter().enumerate() {
        let now = origin_of(&scene, id)?;
        check(
            (now.x - before[i].x - moved.0).abs() < 1e-6 && (now.y - before[i].y - moved.1).abs() < 1e-6,
            &format!(
                "member {i} moved by ({}, {}), not {moved:?}",
                now.x - before[i].x,
                now.y - before[i].y
            ),
        )?;
    }
    // The drawn groups agree with the model.
    for (i, &id) in members.iter().enumerate() {
        let (gx, gy) = group_translate(&nth_group("bf-drag", 4 + u32::try_from(i).map_err(|e| e.to_string())?)?)?;
        let rect = scene.node_rect(id).map_err(e)?;
        check(
            (gx - rect.origin.x).abs() < 0.01 && (gy - rect.origin.y).abs() < 0.01,
            "the drawing is behind the model",
        )?;
    }
    Ok(())
}

#[wasm_bindgen_test]
fn two_drags_in_a_row_each_carry_the_members() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-drag-twice")?;
    let group = nth_group("bf-drag-twice", 3)?;
    for _ in 0..2 {
        dispatch_pointer_event(&group, "pointerdown", 320, 120, 1)?;
        dispatch_pointer_event(&group, "pointermove", 380, 160, 1)?;
        dispatch_pointer_event(&group, "pointerup", 380, 160, 1)?;
    }
    let frame = scene.node_rect(butterfly.frame).map_err(e)?;
    for member in [butterfly.product, butterfly.sum, butterfly.difference] {
        check(inside(frame, scene.node_rect(member).map_err(e)?), "a member was left behind")?;
    }
    check(frame.origin != Point::new(300.0, 100.0), "the frame did not move")
}

/// Dragging the frame a long way lands it where the pointer let go. The frame lies over its own operator nodes, so a
/// drop must not treat them as something to be pushed clear of.
#[wasm_bindgen_test]
fn a_long_drag_lands_the_frame_where_the_pointer_was_released_and_not_pushed_aside() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-drag-far")?;
    let group = nth_group("bf-drag-far", 3)?;
    let drag = |from: (i32, i32), to: (i32, i32)| -> Result<(), String> {
        dispatch_pointer_event(&group, "pointerdown", from.0, from.1, 1)?;
        dispatch_pointer_event(&group, "pointermove", to.0, to.1, 1)?;
        dispatch_pointer_event(&group, "pointerup", to.0, to.1, 1)
    };
    // A first drag settles the offset between the pointer and the page, which is the same for every drag after it.
    drag((320, 120), (330, 130))?;
    let before = origin_of(&scene, butterfly.frame)?;
    drag((330, 130), (530, 190))?;
    let after = origin_of(&scene, butterfly.frame)?;
    check(
        (after.x - before.x - 200.0).abs() < 1e-6 && (after.y - before.y - 60.0).abs() < 1e-6,
        &format!("the frame moved from {before:?} to {after:?}, not by (200, 60)"),
    )?;
    let frame = scene.node_rect(butterfly.frame).map_err(e)?;
    for member in [butterfly.product, butterfly.sum, butterfly.difference] {
        check(inside(frame, scene.node_rect(member).map_err(e)?), "a member was left behind")?;
    }
    Ok(())
}

#[wasm_bindgen_test]
fn move_butterfly_moves_the_frame_and_every_member() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-move")?;
    let relative = |scene: &Scene, id: NodeId| -> Result<(f64, f64), String> {
        let (f, m) = (origin_of(scene, butterfly.frame)?, origin_of(scene, id)?);
        Ok((m.x - f.x, m.y - f.y))
    };
    let before = relative(&scene, butterfly.sum)?;
    scene.move_butterfly(&butterfly, Point::new(500.0, 250.0)).map_err(e)?;
    check(origin_of(&scene, butterfly.frame)? == Point::new(500.0, 250.0), "the frame")?;
    check(
        relative(&scene, butterfly.sum)? == before,
        "the sum kept its place in the frame",
    )?;
    let frame = scene.node_rect(butterfly.frame).map_err(e)?;
    check(
        inside(frame, scene.node_rect(butterfly.difference).map_err(e)?),
        "the difference",
    )
}

#[wasm_bindgen_test]
fn replacing_the_results_changes_the_values_and_leaves_the_butterfly_in_place() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-values")?;
    let before = scene.node_rect(butterfly.sum).map_err(e)?;
    scene
        .set_data_values(butterfly.sum, NodeValues::Complex64(vec![Complex::new(9.0, 9.0)]))
        .map_err(e)?;
    check(scene.node_rect(butterfly.sum).map_err(e)? == before, "the sum moved")?;
    check(
        nth_group("bf-values", 8)?.text_content().unwrap_or_default().contains('9'),
        "the new value is not drawn",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many groups a scene holds, which is how many nodes it has drawn.
fn groups(id: &str) -> Result<u32, String> {
    let selector = format!("#{id} > g.svg-dom-graph-content > g");
    let all = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all(&selector)
        .map_err(|e| format!("{e:?}"))?;
    Ok(all.length())
}

#[wasm_bindgen_test]
fn a_butterfly_must_be_made_of_complex_numbers() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-not-complex")?;
    let integer = scene
        .add_data_node(
            Point::new(600.0, 20.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(e)?;
    let before = groups("bf-not-complex")?;
    let bad_operand = ButterflyOperands::new(integer, operands.b, operands.twiddle);
    let result = scene.add_butterfly_node(Point::new(300.0, 100.0), bad_operand, results());
    check(
        matches!(result, Err(Error::ButterflyNotComplex(Some(id))) if id == integer),
        "an integer operand",
    )?;

    let bad_result = ButterflyResults::new(
        number(4.0, -3.0),
        DataNodeContent::new(NodeValues::U8(vec![5]), DataFormat::Decimal),
        number(-3.0, 5.0),
    );
    let result = scene.add_butterfly_node(Point::new(300.0, 100.0), operands, bad_result);
    check(matches!(result, Err(Error::ButterflyNotComplex(None))), "an integer result")?;
    check(groups("bf-not-complex")? == before, "a rejected call drew something")
}

/// An operand that holds several complex numbers is refused, as a result that does is, and nothing is drawn.
#[wasm_bindgen_test]
fn an_operand_holding_several_complex_numbers_is_refused_and_the_scene_is_left_alone() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-many-operand")?;
    let several = scene
        .add_data_node(
            Point::new(600.0, 20.0),
            DataNodeContent::new(
                NodeValues::Complex64(vec![Complex::new(1.0, 2.0), Complex::new(3.0, 4.0)]),
                DataFormat::Decimal,
            ),
        )
        .map_err(e)?;
    let (nodes, connectors) = (groups("bf-many-operand")?, connector_count("bf-many-operand")?);
    // Each of the three operand places is tried in turn with the several-valued node.
    for replaced in [
        ButterflyOperands::new(several, operands.b, operands.twiddle),
        ButterflyOperands::new(operands.a, several, operands.twiddle),
        ButterflyOperands::new(operands.a, operands.b, several),
    ] {
        let result = scene.add_butterfly_node(Point::new(300.0, 100.0), replaced, results());
        check(
            matches!(result, Err(Error::ButterflyNotComplex(Some(id))) if id == several),
            &format!("{result:?}"),
        )?;
    }
    check(groups("bf-many-operand")? == nodes, "a rejected call drew a node")?;
    check(
        connector_count("bf-many-operand")? == connectors,
        "a rejected call drew a connector",
    )
}

#[wasm_bindgen_test]
fn the_twiddle_factor_and_b_must_be_different_nodes() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-duplicate")?;
    let before = groups("bf-duplicate")?;
    let same = ButterflyOperands::new(operands.a, operands.b, operands.b);
    let result = scene.add_butterfly_node(Point::new(300.0, 100.0), same, results());
    check(
        matches!(result, Err(Error::DuplicateOperands(id)) if id == operands.b),
        "duplicate operands",
    )?;
    check(groups("bf-duplicate")? == before, "a rejected call drew something")
}

#[wasm_bindgen_test]
fn operands_and_results_must_share_one_precision() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-precision")?;
    let single = DataNodeContent::new(
        NodeValues::Complex32(vec![svg_dom_graph::complex::Complex32::new(1.0, 1.0)]),
        DataFormat::Decimal,
    );
    let mixed = ButterflyResults::new(number(4.0, -3.0), single, number(-3.0, 5.0));
    let result = scene.add_butterfly_node(Point::new(300.0, 100.0), operands, mixed);
    check(matches!(result, Err(Error::OperatorTypeMismatch { .. })), "mixed precision")
}

#[wasm_bindgen_test]
fn a_position_that_is_not_finite_is_refused() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-nan")?;
    let result = scene.add_butterfly_node(Point::new(f64::NAN, 0.0), operands, results());
    check(matches!(result, Err(Error::InvalidNodeGeometry(_))), "NaN position")
}

#[wasm_bindgen_test]
fn an_unknown_operand_is_refused() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-unknown")?;
    let (other, other_operands) = scene_with_operands("bf-unknown-other")?;
    drop(other);
    let foreign = ButterflyOperands::new(other_operands.a, operands.b, operands.twiddle);
    let result = scene.add_butterfly_node(Point::new(300.0, 100.0), foreign, results());
    check(matches!(result, Err(Error::UnknownNode(_))), "a node from another scene")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn a_failure_while_drawing_removes_everything_the_call_added() -> Result<(), String> {
    let (scene, operands) = scene_with_operands("bf-rollback")?;
    let (nodes, connectors) = (groups("bf-rollback")?, connector_count("bf-rollback")?);
    {
        // The first connector cannot be written, which is after the frame has been drawn.
        let _failing = FailingWrites::start(&["d"])?;
        let result = scene.add_butterfly_node(Point::new(300.0, 100.0), operands, results());
        check(result.is_err(), "the write was meant to fail")?;
    }
    check(groups("bf-rollback")? == nodes, "a node was left behind")?;
    check(connector_count("bf-rollback")? == connectors, "a connector was left behind")?;
    // And the scene still works: the same call now succeeds.
    scene
        .add_butterfly_node(Point::new(300.0, 100.0), operands, results())
        .map_err(e)?;
    check(
        groups("bf-rollback")? == nodes + 7,
        "the retry drew seven nodes: the frame, three ports and three operator nodes",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every port and operator node is at the frame's corner plus the offset it had when the butterfly was made.
fn check_in_place(scene: &Scene, butterfly: &Butterfly, offsets: &Offsets) -> Result<(), String> {
    let frame = origin_of(scene, butterfly.frame)?;
    for &(id, (dx, dy)) in offsets.iter() {
        let at = origin_of(scene, id)?;
        check(
            (at.x - frame.x - dx).abs() < 1e-6 && (at.y - frame.y - dy).abs() < 1e-6,
            &format!("{id:?} is at {at:?}, not at the frame {frame:?} plus ({dx}, {dy})"),
        )?;
    }
    Ok(())
}

/// A node, and how far it is from the frame's top left corner.
type Offsets = Vec<(NodeId, (f64, f64))>;

/// The offset of every member from the frame, as the butterfly was drawn.
fn offsets_of(scene: &Scene, butterfly: &Butterfly) -> Result<Offsets, String> {
    let frame = origin_of(scene, butterfly.frame)?;
    [
        butterfly.port_a,
        butterfly.port_b,
        butterfly.port_w,
        butterfly.product,
        butterfly.sum,
        butterfly.difference,
    ]
    .into_iter()
    .map(|id| {
        let at = origin_of(scene, id)?;
        Ok((id, (at.x - frame.x, at.y - frame.y)))
    })
    .collect()
}

/// A move that fails part way can be made again to the same place. The node that failed catches up, and the nodes that
/// had already moved are not moved a second time.
#[wasm_bindgen_test]
fn a_move_that_failed_part_way_is_put_right_by_moving_to_the_same_place_again() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-retry")?;
    let offsets = offsets_of(&scene, &butterfly)?;
    let target = Point::new(520.0, 240.0);

    // The second port's own move cannot be written, once.
    let port_b = nth_group("bf-retry", 5)?;
    let failed = {
        let _failing = FailingWrites::on_element(&port_b, &["transform"], true)?;
        scene.move_butterfly(&butterfly, target)
    };
    check(failed.is_err(), "the move was meant to fail")?;
    check(origin_of(&scene, butterfly.frame)? == target, "the frame did not move")?;
    // Every other node was still moved, and the one that failed was left where it was.
    check(
        origin_of(&scene, butterfly.port_a)?.x > 400.0 && origin_of(&scene, butterfly.sum)?.x > 400.0,
        "the nodes after the failed one were not moved",
    )?;
    check(origin_of(&scene, butterfly.port_b)?.x < 400.0, "the node that failed moved")?;

    // The same move again: nothing is injected now, and the node left behind catches up.
    scene.move_butterfly(&butterfly, target).map_err(e)?;
    check_in_place(&scene, &butterfly, &offsets)?;
    // And a third time changes nothing.
    scene.move_butterfly(&butterfly, target).map_err(e)?;
    check_in_place(&scene, &butterfly, &offsets)
}

/// A drag in which one member's move cannot be written does not leave that member behind, or push any member past the
/// frame, whether it is put right by a later report of the same drag or by the next drag.
#[wasm_bindgen_test]
fn a_drag_that_loses_a_write_does_not_make_any_member_overshoot_or_stay_behind() -> Result<(), String> {
    let (scene, butterfly) = add_butterfly("bf-drag-failure")?;
    let offsets = offsets_of(&scene, &butterfly)?;
    let frame_group = nth_group("bf-drag-failure", 3)?;
    let port_b = nth_group("bf-drag-failure", 5)?;

    // The first move that is applied cannot write the second port, and after that everything writes.
    {
        let _failing = FailingWrites::on_element(&port_b, &["transform"], true)?;
        dispatch_pointer_event(&frame_group, "pointerdown", 320, 120, 1)?;
        dispatch_pointer_event(&frame_group, "pointermove", 380, 160, 1)?;
        dispatch_pointer_event(&frame_group, "pointerup", 380, 160, 1)?;
    }
    check(
        origin_of(&scene, butterfly.frame)? != Point::new(300.0, 100.0),
        "the frame did not move",
    )?;
    // The end of the drag, and every report before it, puts each member at the frame's corner plus its offset.
    check_in_place(&scene, &butterfly, &offsets)?;

    // The next drag adds a distance to each member's own place, and not to a stale one.
    dispatch_pointer_event(&frame_group, "pointerdown", 380, 160, 1)?;
    dispatch_pointer_event(&frame_group, "pointermove", 440, 200, 1)?;
    dispatch_pointer_event(&frame_group, "pointerup", 440, 200, 1)?;
    check_in_place(&scene, &butterfly, &offsets)
}

/// A butterfly made by another scene is refused, and nothing moves.
#[wasm_bindgen_test]
fn moving_a_butterfly_that_belongs_to_another_scene_is_refused() -> Result<(), String> {
    let (_scene, butterfly) = add_butterfly("bf-own")?;
    let (other, _) = add_butterfly("bf-other")?;
    let before = origin_of(&other, butterfly.frame).ok();
    let refused = other.move_butterfly(&butterfly, Point::new(10.0, 10.0));
    check(matches!(refused, Err(Error::UnknownNode(_))), &format!("{refused:?}"))?;
    check(origin_of(&other, butterfly.frame).ok() == before, "something moved")
}
