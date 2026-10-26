use super::*;
use crate::test_support::check;

fn sizes() -> (Size, Size, Size) {
    (Size::new(70.0, 50.0), Size::new(90.0, 54.0), Size::new(80.0, 56.0))
}

fn inside(outer: Rect, inner: Rect) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.size.width <= outer.origin.x + outer.size.width
        && inner.origin.y + inner.size.height <= outer.origin.y + outer.size.height
}

fn rect(origin: Point, size: Size) -> Rect {
    Rect { origin, size }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_frame_holds_all_three_boxes_with_the_padding_round_them() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(100.0, 40.0), product, sum, difference);
    for (origin, size) in [(at.product, product), (at.sum, sum), (at.difference, difference)] {
        check(inside(at.frame, rect(origin, size)), "a box sticks out of the frame")?;
    }
    check(at.product.x - at.frame.origin.x >= FRAME_PADDING, "the left padding")?;
    check(
        at.frame.origin.x + at.frame.size.width - (at.sum.x + sum.width.max(difference.width)) >= FRAME_PADDING - 1e-9,
        "the right padding",
    )?;
    check(
        at.frame.origin.y + at.frame.size.height - (at.difference.y + difference.height) >= FRAME_PADDING - 1e-9,
        "the bottom padding",
    )
}

#[test]
fn the_addition_is_in_the_first_row_and_the_multiplication_and_subtraction_are_in_the_second() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(0.0, 0.0), product, sum, difference);
    check(
        at.sum.y + sum.height < at.product.y,
        "the addition is not above the multiplication",
    )?;
    check(
        at.sum.y + sum.height < at.difference.y,
        "the addition is not above the subtraction",
    )?;
    check(
        at.product.x + product.width < at.sum.x,
        "the multiplication is not left of the addition",
    )?;
    check(
        at.sum.x == at.difference.x,
        "the addition and the subtraction are not in one column",
    )
}

#[test]
fn the_product_leaves_level_with_the_subtractions_second_input_so_the_connector_is_horizontal() -> Result<(), String> {
    for (product, difference) in [
        (Size::new(70.0, 50.0), Size::new(80.0, 56.0)),
        (Size::new(70.0, 56.0), Size::new(80.0, 56.0)),
        (Size::new(70.0, 90.0), Size::new(80.0, 40.0)),
        (Size::new(70.0, 20.0), Size::new(80.0, 70.0)),
    ] {
        let at = layout(Point::new(5.0, 7.0), product, Size::new(90.0, 54.0), difference);
        let leaves = at.product.y + product.height / 2.0;
        let arrives = at.difference.y + input_slots(difference.height)[1];
        check(
            (leaves - arrives).abs() < 1e-9,
            &format!("{leaves} is not level with {arrives}"),
        )?;
        check(
            inside(at.frame, rect(at.product, product)),
            "the multiplication sticks out of the frame",
        )?;
        check(
            inside(at.frame, rect(at.difference, difference)),
            "the subtraction sticks out of the frame",
        )?;
    }
    Ok(())
}

#[test]
fn the_boxes_do_not_overlap_and_the_two_columns_are_a_gap_apart() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(0.0, 0.0), product, sum, difference);
    check(
        at.product.x + product.width + COLUMN_GAP <= at.sum.x + 1e-9,
        "the columns are too close",
    )?;
    check(
        at.sum.y + sum.height + ROW_GAP <= at.product.y.min(at.difference.y) + 1e-9,
        "the rows are too close",
    )
}

#[test]
fn there_is_room_above_the_addition_for_the_title() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(10.0, 20.0), product, sum, difference);
    check(at.sum.y - at.frame.origin.y >= TITLE_ROOM, "no room for a title")
}

#[test]
fn the_three_ports_are_circles_in_order_down_the_frames_west_edge() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(100.0, 40.0), product, sum, difference);
    let [a, b, w] = at.ports;
    check(
        a.x == at.frame.origin.x && b.x == a.x && w.x == a.x,
        "a port is not on the west edge",
    )?;
    check(a.y < b.y && b.y < w.y, "the ports are not in the order A, B, W")?;
    check(a.y - PORT_RADIUS >= at.frame.origin.y, "A sticks out of the top")?;
    check(
        w.y + PORT_RADIUS <= at.frame.origin.y + at.frame.size.height,
        "W sticks out of the bottom",
    )
}

#[test]
fn the_ports_that_feed_the_multiplication_do_not_touch_and_are_centred_on_it() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(0.0, 0.0), product, sum, difference);
    let [_, b, w] = at.ports;
    check(w.y - b.y >= 2.0 * PORT_RADIUS + PORT_GAP - 1e-9, "B and W touch")?;
    let middle = at.product.y + product.height / 2.0;
    check(
        ((b.y + w.y) / 2.0 - middle).abs() < 1e-9,
        "B and W are not centred on the multiplication",
    )
}

#[test]
fn port_a_is_level_with_the_additions_first_fixing_point() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(0.0, 0.0), product, sum, difference);
    let [slot, _] = input_slots(sum.height);
    check(
        (at.ports[0].y - (at.sum.y + slot)).abs() < 1e-9,
        "A is not level with the addition's first point",
    )
}

#[test]
fn the_two_inputs_of_a_side_are_a_quarter_and_three_quarters_of_the_way_down_it() -> Result<(), String> {
    let [first, second] = input_slots(60.0);
    check((first - 15.0).abs() < 1e-9 && (second - 45.0).abs() < 1e-9, "the input points")
}

#[test]
fn moving_the_frame_moves_the_whole_layout_by_the_same_amount() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let a = layout(Point::new(0.0, 0.0), product, sum, difference);
    let b = layout(Point::new(25.0, -10.0), product, sum, difference);
    for (p, q) in [(a.product, b.product), (a.sum, b.sum), (a.difference, b.difference)] {
        check(q.x - p.x == 25.0 && q.y - p.y == -10.0, "a box did not move with the frame")?;
    }
    check(a.frame.size == b.frame.size, "the frame changed shape")?;
    for (p, q) in a.ports.into_iter().zip(b.ports) {
        check(q.x - p.x == 25.0 && q.y - p.y == -10.0, "a port did not move with the frame")?;
    }
    Ok(())
}

#[test]
fn only_a_single_complex_number_is_a_result() -> Result<(), String> {
    use crate::{
        complex::Complex,
        scene::{DataFormat, NodeValues},
    };
    let one = DataNodeContent::new(NodeValues::Complex64(vec![Complex::new(1.0, 2.0)]), DataFormat::Decimal);
    check(require_complex_result(&one).is_ok(), "one complex number")?;
    let two = DataNodeContent::new(
        NodeValues::Complex64(vec![Complex::new(1.0, 2.0), Complex::new(3.0, 4.0)]),
        DataFormat::Decimal,
    );
    check(
        matches!(require_complex_result(&two), Err(Error::ButterflyNotComplex(None))),
        "two numbers",
    )?;
    let integer = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    check(
        matches!(require_complex_result(&integer), Err(Error::ButterflyNotComplex(None))),
        "an integer",
    )
}

#[test]
fn the_channel_in_front_of_the_subtraction_is_half_way_across_the_gap_after_the_multiplication() -> Result<(), String> {
    let (product, sum, difference) = sizes();
    let at = layout(Point::new(10.0, 10.0), product, sum, difference);
    let channel = at.difference.x - SAME_SIDE_CLEARANCE;
    let middle = (at.product.x + product.width + at.difference.x) / 2.0;
    check(
        (channel - middle).abs() < 1e-9,
        &format!("the channel is at {channel}, the middle is {middle}"),
    )
}
