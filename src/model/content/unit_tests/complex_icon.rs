//! The pictures at the front of a polar cell and a rectangular cell: the path data of the arrow, and of the two legs, for
//! numbers in every direction and of every size, and the numbers that cannot be drawn.

use super::super::*;
use super::support::check_eq;
use crate::test_support::check;

const MIDDLE: f64 = ICON_SIZE / 2.0;

/// Every number in the path data, in order, whatever letter it follows.
fn numbers(path: &str) -> Vec<f64> {
    path.split_whitespace().filter_map(|word| word.parse().ok()).collect()
}

fn polar(re: f64, im: f64, max_modulus: f64) -> IconPaths {
    icon_paths(IconKind::Polar, re, im, max_modulus)
}

fn rectangular(re: f64, im: f64, max_modulus: f64) -> IconPaths {
    icon_paths(IconKind::Rectangular, re, im, max_modulus)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The arrow
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_longest_arrow_reaches_the_edge_of_the_range_in_its_own_direction() -> Result<(), String> {
    // 3 + 4i has a modulus of 5, so with 5 as the largest it is at full reach: 9 units, at 3-4-5 of the way across and up.
    let paths = polar(3.0, 4.0, 5.0);

    check(paths.primary.starts_with("M 11.00 11.00 L 16.40 3.80"), &paths.primary)?;
    check_eq(paths.secondary, String::new())
}

#[test]
fn an_arrow_is_as_long_as_the_modulus_is_a_share_of_the_largest() -> Result<(), String> {
    let tip = |re: f64, max: f64| {
        let nums = numbers(&polar(re, 0.0, max).primary);
        nums[2] - MIDDLE
    };

    check((tip(5.0, 5.0) - 9.0).abs() < 1e-9, "the largest reaches 9")?;
    check((tip(2.5, 5.0) - 4.5).abs() < 1e-9, "half as long is half as far")?;
    check((tip(2.0, 5.0) - 3.6).abs() < 1e-9, "two fifths as long is two fifths as far")
}

#[test]
fn an_arrow_points_the_way_the_number_does() -> Result<(), String> {
    // The page's `y` points down, so up the plane is a smaller `y`.
    let tip = |re: f64, im: f64| {
        let nums = numbers(&polar(re, im, 1.0).primary);
        (nums[2] - MIDDLE, nums[3] - MIDDLE)
    };

    let (x, y) = tip(1.0, 0.0);
    check(x > 8.9 && y.abs() < 1e-9, "east")?;
    let (x, y) = tip(0.0, 1.0);
    check(x.abs() < 1e-9 && y < -8.9, "north is up the page")?;
    let (x, y) = tip(-1.0, 0.0);
    check(x < -8.9 && y.abs() < 1e-9, "west")?;
    let (x, y) = tip(0.0, -1.0);
    check(x.abs() < 1e-9 && y > 8.9, "south is down the page")
}

#[test]
fn a_small_number_is_still_an_arrow_you_can_see() -> Result<(), String> {
    // A millionth of the largest would be a pixel of nothing, so it is drawn no shorter than a couple of units.
    let nums = numbers(&polar(1e-6, 0.0, 1.0).primary);
    let length = nums[2] - MIDDLE;

    check((length - 2.5).abs() < 1e-9, &format!("length {length}"))
}

#[test]
fn zero_is_a_dot() -> Result<(), String> {
    check_eq(polar(0.0, 0.0, 5.0).primary, "M 11.00 11.00 h 0".to_owned())?;
    check_eq(polar(-0.0, -0.0, 5.0).primary, "M 11.00 11.00 h 0".to_owned())
}

#[test]
fn an_arrow_has_a_head_of_two_strokes_back_from_the_tip() -> Result<(), String> {
    let path = polar(1.0, 0.0, 1.0).primary;
    // A shaft, and then a stroke out to one side, to the tip, and out to the other.
    check_eq(path.matches('M').count(), 2)?;
    check_eq(path.matches('L').count(), 3)?;
    let nums = numbers(&path);
    let (tip_x, tip_y) = (nums[2], nums[3]);
    let (left, right) = ((nums[4], nums[5]), (nums[8], nums[9]));
    // Both ends of the head are behind the tip and either side of the shaft.
    check(left.0 < tip_x && right.0 < tip_x, "the head sweeps back")?;
    check((left.1 - tip_y) * (right.1 - tip_y) < 0.0, "one stroke each side of the shaft")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The legs
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_legs_are_the_adjacent_along_the_real_axis_and_then_the_opposite_up_to_the_number() -> Result<(), String> {
    let paths = rectangular(3.0, 4.0, 5.0);

    check_eq(paths.primary, "M 11.00 11.00 H 16.40".to_owned())?;
    check_eq(paths.secondary, "M 16.40 11.00 V 3.80".to_owned())
}

#[test]
fn a_negative_part_runs_the_other_way() -> Result<(), String> {
    let paths = rectangular(-3.0, -4.0, 5.0);

    check_eq(paths.primary, "M 11.00 11.00 H 5.60".to_owned())?;
    check_eq(paths.secondary, "M 5.60 11.00 V 18.20".to_owned())
}

#[test]
fn a_number_on_an_axis_has_one_leg_with_no_length() -> Result<(), String> {
    let on_real = rectangular(5.0, 0.0, 5.0);
    check_eq(on_real.secondary, "M 20.00 11.00 V 11.00".to_owned())?;
    let on_imaginary = rectangular(0.0, 5.0, 5.0);
    check_eq(on_imaginary.primary, "M 11.00 11.00 H 11.00".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Both
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_picture_stays_inside_its_box() -> Result<(), String> {
    for kind in [IconKind::Polar, IconKind::Rectangular] {
        for turn in 0..72 {
            let angle = f64::from(turn) * std::f64::consts::TAU / 72.0;
            for modulus in [1e-9, 0.01, 0.3, 1.0] {
                let paths = icon_paths(kind, modulus * angle.cos(), modulus * angle.sin(), 1.0);
                for value in numbers(&paths.primary).into_iter().chain(numbers(&paths.secondary)) {
                    check(
                        (0.0..=ICON_SIZE).contains(&value),
                        &format!("{kind:?} at {turn}/72 of a turn and modulus {modulus} reaches {value}"),
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_number_that_cannot_be_drawn_gives_no_path() -> Result<(), String> {
    for kind in [IconKind::Polar, IconKind::Rectangular] {
        for (re, im, max) in [
            (f64::NAN, 1.0, 1.0),
            (1.0, f64::INFINITY, 1.0),
            (f64::NEG_INFINITY, 0.0, 1.0),
            (1.0, 1.0, 0.0),
            (1.0, 1.0, -1.0),
            (1.0, 1.0, f64::NAN),
            (1.0, 1.0, f64::INFINITY),
        ] {
            check_eq(icon_paths(kind, re, im, max), IconPaths::default())?;
        }
    }
    Ok(())
}

#[test]
fn the_axes_cross_in_the_middle_of_the_box() -> Result<(), String> {
    check_eq(axes_path(), "M 1 11 H 21 M 11 1 V 21".to_owned())
}

#[test]
fn the_format_says_which_picture_a_cell_has() -> Result<(), String> {
    let format = |form| DataFormat::ComplexNumber(form, ComplexStyle::default());

    check_eq(IconKind::of(format(ComplexNumberFormat::Polar)), Some(IconKind::Polar))?;
    check_eq(
        IconKind::of(format(ComplexNumberFormat::Rectangular)),
        Some(IconKind::Rectangular),
    )?;
    check_eq(IconKind::of(format(ComplexNumberFormat::Matrix)), None)?;
    check_eq(IconKind::of(DataFormat::Decimal), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The weight of the lines
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_lines_come_out_the_same_width_whatever_size_the_picture_is_scaled_to() -> Result<(), String> {
    for scale in [0.25, 0.5, 1.0, 2.4, 8.0] {
        let (vector, axes) = line_widths(scale);

        // After the node scales the picture, the line is as thick as it was meant to be.
        check(
            (vector * scale - 1.1).abs() < 1e-9,
            &format!("the vector at {scale}: {}", vector * scale),
        )?;
        check(
            (axes * scale - 0.6).abs() < 1e-9,
            &format!("the axes at {scale}: {}", axes * scale),
        )?;
    }
    Ok(())
}

#[test]
fn the_axes_are_thinner_than_the_vector_and_both_are_thin_beside_the_arrow() -> Result<(), String> {
    let (vector, axes) = line_widths(1.0);

    check(axes < vector, "the faint axes are the thinner")?;
    // The arrow reaches nine units, so a line a fifth of that would be heavy.
    check(
        vector < 9.0 / 5.0,
        &format!("a line of {vector} is too heavy beside a 9 unit arrow"),
    )
}

#[test]
fn a_scale_that_is_not_a_positive_number_gives_a_finite_width() -> Result<(), String> {
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let (vector, axes) = line_widths(scale);
        check(
            vector.is_finite() && axes.is_finite() && vector > 0.0,
            &format!("scale {scale}: {vector}, {axes}"),
        )?;
    }
    Ok(())
}
