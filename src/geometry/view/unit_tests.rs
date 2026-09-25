use super::*;
use crate::test_support::check;

/// Maps a content-space `p` to viewport space through `view`.
fn apply(view: ViewTransform, p: Point) -> Point {
    Point::new(p.x * view.scale + view.tx, p.y * view.scale + view.ty)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn zooming_keeps_the_pivot_fixed_on_screen() -> Result<(), String> {
    let start = ViewTransform {
        scale: 1.5,
        tx: 30.0,
        ty: -12.0,
    };
    let pivot = Point::new(100.0, 80.0);
    // The content point currently under the pivot.
    let under = Point::new((pivot.x - start.tx) / start.scale, (pivot.y - start.ty) / start.scale);

    let zoomed = start.zoomed_about(ZOOM_STEP, pivot);
    let now = apply(zoomed, under);
    check(close(now.x, pivot.x) && close(now.y, pivot.y), "pivot drifted")?;
    check(close(zoomed.scale, 1.5 * ZOOM_STEP), "scale did not multiply by the factor")
}

#[test]
fn zoom_in_then_out_returns_to_the_identity() -> Result<(), String> {
    let pivot = Point::new(64.0, 40.0);
    let back = ViewTransform::IDENTITY
        .zoomed_about(ZOOM_STEP, pivot)
        .zoomed_about(1.0 / ZOOM_STEP, pivot);
    check(
        close(back.scale, 1.0) && close(back.tx, 0.0) && close(back.ty, 0.0),
        "did not return to the identity",
    )
}

#[test]
fn scale_is_clamped_and_the_pivot_still_holds_at_the_limit() -> Result<(), String> {
    let pivot = Point::new(50.0, 50.0);
    let mut view = ViewTransform::IDENTITY;
    for _ in 0..50 {
        view = view.zoomed_about(ZOOM_STEP, pivot);
    }
    check(view.scale == MAX_SCALE, "zoom-in was not clamped to MAX_SCALE")?;
    check(
        !view.can_zoom_in() && view.can_zoom_out(),
        "can_zoom_* wrong at the upper limit",
    )?;
    // The content point that started under the pivot (50, 50) must still be there.
    let now = apply(view, pivot);
    check(close(now.x, pivot.x) && close(now.y, pivot.y), "pivot drifted at the limit")?;

    for _ in 0..100 {
        view = view.zoomed_about(1.0 / ZOOM_STEP, pivot);
    }
    check(view.scale == MIN_SCALE, "zoom-out was not clamped to MIN_SCALE")?;
    check(
        view.can_zoom_in() && !view.can_zoom_out(),
        "can_zoom_* wrong at the lower limit",
    )
}

#[test]
fn a_non_finite_or_non_positive_factor_changes_nothing() -> Result<(), String> {
    let view = ViewTransform::IDENTITY;
    let pivot = Point::new(1.0, 1.0);
    for factor in [f64::NAN, f64::INFINITY, 0.0, -2.0] {
        check(view.zoomed_about(factor, pivot) == view, "an invalid factor altered the view")?;
    }
    Ok(())
}

#[test]
fn write_attr_replaces_previous_buffer_content() -> Result<(), String> {
    let mut out = String::from("stale");
    ViewTransform { scale: 2.0, tx: 5.0, ty: -3.5 }.write_attr(&mut out);
    check(out == "translate(5, -3.5) scale(2)", &format!("unexpected attribute: {out}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn translating_moves_content_by_the_same_amount_on_screen_at_any_scale() -> Result<(), String> {
    let view = ViewTransform { scale: 2.0, tx: 10.0, ty: 20.0 };
    let moved = view.translated(5.0, -7.0);
    let before = apply(view, Point::new(3.0, 4.0));
    let after = apply(moved, Point::new(3.0, 4.0));
    check(
        close(after.x - before.x, 5.0) && close(after.y - before.y, -7.0),
        "content did not move by (5, -7)",
    )?;
    check(moved.scale == 2.0, "translating changed the scale")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn one_wheel_notch_is_one_zoom_step_and_scrolling_up_zooms_in() -> Result<(), String> {
    check(close(wheel_zoom_factor(-100.0, 0), ZOOM_STEP), "a notch up is not one step in")?;
    check(
        close(wheel_zoom_factor(100.0, 0), 1.0 / ZOOM_STEP),
        "a notch down is not one step out",
    )?;
    check(close(wheel_zoom_factor(0.0, 0), 1.0), "no movement changed the zoom")?;
    check(
        close(wheel_zoom_factor(-10.0, 0), ZOOM_STEP.powf(0.1)),
        "small deltas do not zoom in proportion",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn line_and_page_deltas_are_scaled_and_a_runaway_delta_is_limited() -> Result<(), String> {
    // Firefox reports one wheel notch as 3 lines.
    check(close(wheel_zoom_factor(-2.5, 1), ZOOM_STEP), "2.5 lines is not one notch")?;
    check(
        close(wheel_zoom_factor(-1.0, 2), ZOOM_STEP.powf(4.0)),
        "a page delta was not limited",
    )?;
    check(
        close(wheel_zoom_factor(-1.0e9, 0), ZOOM_STEP.powf(4.0)),
        "a huge delta was not limited",
    )?;
    check(
        close(wheel_zoom_factor(1.0e9, 0), ZOOM_STEP.powf(-4.0)),
        "a huge delta was not limited",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_non_finite_wheel_delta_changes_nothing() -> Result<(), String> {
    check(wheel_zoom_factor(f64::NAN, 0) == 1.0, "NaN changed the zoom")?;
    check(wheel_zoom_factor(f64::INFINITY, 0) == 1.0, "infinity changed the zoom")
}
