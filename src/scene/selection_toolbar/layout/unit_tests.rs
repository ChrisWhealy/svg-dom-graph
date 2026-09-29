use super::*;

fn check<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

fn area() -> Rect {
    Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(400.0, 300.0),
    }
}

fn sizes() -> [Size; 3] {
    [Size::new(60.0, 24.0), Size::new(60.0, 24.0), Size::new(70.0, 24.0)]
}

#[test]
fn a_south_bar_runs_left_to_right_centred_above_the_bottom_margin() -> Result<(), String> {
    let l = layout(Side::South, area(), &sizes(), 8.0, 12.0);
    // 60 + 60 + 70 + 2 gaps of 8
    check(l.bar.size, Size::new(206.0, 24.0))?;
    check(l.bar.origin, Point::new(97.0, 264.0))?;
    let xs: Vec<f64> = l.buttons.iter().map(|b| b.origin.x).collect();
    check(xs, vec![0.0, 68.0, 136.0])
}

#[test]
fn a_north_bar_sits_below_the_top_margin() -> Result<(), String> {
    let l = layout(Side::North, area(), &sizes(), 8.0, 12.0);
    check(l.bar.origin, Point::new(97.0, 12.0))
}

#[test]
fn an_east_bar_stacks_top_to_bottom_and_widens_every_button_to_the_widest() -> Result<(), String> {
    let l = layout(Side::East, area(), &sizes(), 8.0, 12.0);
    check(l.bar.size, Size::new(70.0, 88.0))?;
    let widths: Vec<f64> = l.buttons.iter().map(|b| b.size.width).collect();
    check(widths, vec![70.0, 70.0, 70.0])
}

#[test]
fn the_bar_never_starts_before_the_areas_own_origin() -> Result<(), String> {
    let tiny = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(10.0, 10.0),
    };
    let l = layout(Side::West, tiny, &sizes(), 8.0, 12.0);
    check(l.bar.origin.x >= 0.0, true)
}
