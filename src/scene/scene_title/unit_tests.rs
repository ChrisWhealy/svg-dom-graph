use super::{layout::layout, *};
use crate::test_support::check;
use svg_dom::root::utils::{Rect, Size};

fn area() -> Rect {
    Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 300.0),
    }
}

fn size() -> Size {
    Size::new(80.0, 24.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_north_title_sits_centred_below_the_top_margin() -> Result<(), String> {
    let rect = layout(Side::North, area(), size(), 8.0);
    check(rect.size == size(), &format!("size {:?}", rect.size))?;
    check(rect.origin == Point::new(160.0, 8.0), &format!("origin {:?}", rect.origin))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_south_title_sits_centred_above_the_bottom_margin() -> Result<(), String> {
    let rect = layout(Side::South, area(), size(), 8.0);
    check(rect.origin == Point::new(160.0, 268.0), &format!("origin {:?}", rect.origin))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_west_title_sits_centred_left_of_the_left_margin() -> Result<(), String> {
    let rect = layout(Side::West, area(), size(), 8.0);
    check(rect.origin == Point::new(8.0, 138.0), &format!("origin {:?}", rect.origin))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn an_east_title_sits_centred_right_of_the_right_margin() -> Result<(), String> {
    let rect = layout(Side::East, area(), size(), 8.0);
    check(rect.origin == Point::new(312.0, 138.0), &format!("origin {:?}", rect.origin))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_title_larger_than_the_area_never_starts_before_the_area_origin() -> Result<(), String> {
    let tiny = Rect {
        origin: Point::new(10.0, 10.0),
        size: Size::new(30.0, 30.0),
    };
    let rect = layout(Side::North, tiny, size(), 8.0);
    check(rect.origin.x == 10.0, &format!("origin.x {}", rect.origin.x))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_non_zero_area_origin_offsets_the_title() -> Result<(), String> {
    let shifted = Rect {
        origin: Point::new(-100.0, 50.0),
        size: Size::new(400.0, 300.0),
    };
    let rect = layout(Side::North, shifted, size(), 8.0);
    check(rect.origin == Point::new(60.0, 58.0), &format!("origin {:?}", rect.origin))
}
