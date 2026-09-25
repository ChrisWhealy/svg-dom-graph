use super::*;
use crate::test_support::check;

fn area() -> Rect {
    Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 300.0),
    }
}

fn sizes() -> [Size; 3] {
    [Size::new(20.0, 20.0), Size::new(20.0, 20.0), Size::new(50.0, 20.0)]
}

#[test]
fn a_north_bar_runs_left_to_right_centred_below_the_top_margin() -> Result<(), String> {
    let l = layout(Side::North, area(), &sizes(), 4.0, 8.0);
    // 20 + 20 + 50 + 2 gaps of 4
    check(l.bar.size == Size::new(98.0, 20.0), &format!("bar size {:?}", l.bar.size))?;
    check(
        l.bar.origin == Point::new(151.0, 8.0),
        &format!("bar origin {:?}", l.bar.origin),
    )?;
    let xs: Vec<f64> = l.buttons.iter().map(|b| b.origin.x).collect();
    check(xs == vec![0.0, 24.0, 48.0], &format!("button xs {xs:?}"))
}

#[test]
fn a_south_bar_sits_above_the_bottom_margin() -> Result<(), String> {
    let l = layout(Side::South, area(), &sizes(), 4.0, 8.0);
    check(
        l.bar.origin == Point::new(151.0, 272.0),
        &format!("bar origin {:?}", l.bar.origin),
    )
}

#[test]
fn a_west_bar_stacks_top_to_bottom_and_widens_every_button_to_the_widest() -> Result<(), String> {
    let l = layout(Side::West, area(), &sizes(), 4.0, 8.0);
    check(l.bar.size == Size::new(50.0, 68.0), &format!("bar size {:?}", l.bar.size))?;
    check(
        l.bar.origin == Point::new(8.0, 116.0),
        &format!("bar origin {:?}", l.bar.origin),
    )?;
    let ys: Vec<f64> = l.buttons.iter().map(|b| b.origin.y).collect();
    check(ys == vec![0.0, 24.0, 48.0], &format!("button ys {ys:?}"))?;
    check(
        l.buttons.iter().all(|b| b.size.width == 50.0),
        "buttons were not widened to the widest",
    )
}

#[test]
fn an_east_bar_sits_left_of_the_right_margin() -> Result<(), String> {
    let l = layout(Side::East, area(), &sizes(), 4.0, 8.0);
    check(
        l.bar.origin == Point::new(342.0, 116.0),
        &format!("bar origin {:?}", l.bar.origin),
    )
}

#[test]
fn a_bar_larger_than_the_area_never_starts_before_the_area_origin() -> Result<(), String> {
    let tiny = Rect {
        origin: Point::new(10.0, 10.0),
        size: Size::new(30.0, 30.0),
    };
    let l = layout(Side::North, tiny, &sizes(), 4.0, 8.0);
    check(l.bar.origin.x == 10.0, &format!("bar x {}", l.bar.origin.x))
}

#[test]
fn a_non_zero_area_origin_offsets_the_bar() -> Result<(), String> {
    let shifted = Rect {
        origin: Point::new(-100.0, 50.0),
        size: Size::new(400.0, 300.0),
    };
    let l = layout(Side::North, shifted, &sizes(), 4.0, 8.0);
    check(
        l.bar.origin == Point::new(51.0, 58.0),
        &format!("bar origin {:?}", l.bar.origin),
    )
}

#[test]
fn view_box_parsing_accepts_spaces_and_commas_and_rejects_bad_input() -> Result<(), String> {
    let parsed = parse_view_box("0 0 640,480").ok_or("valid viewBox rejected")?;
    check(parsed.size == Size::new(640.0, 480.0), "wrong size")?;
    check(parse_view_box("0 0 640").is_none(), "three numbers accepted")?;
    check(parse_view_box("0 0 640 480 1").is_none(), "five numbers accepted")?;
    check(parse_view_box("0 0 0 480").is_none(), "zero width accepted")?;
    check(parse_view_box("0 0 x 480").is_none(), "non-number accepted")?;
    check(parse_view_box("0 0 inf 480").is_none(), "infinite width accepted")?;
    check(parse_view_box("").is_none(), "empty string accepted")
}
