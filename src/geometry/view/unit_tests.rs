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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Zoom-anchoring tests. A view maps a graph point `p` to `s * p + t`. Zooming about an anchor `a` must give
// `t' = a - (s' / s) * (a - t)`, where `s'` is the scale actually applied *after* clamping — not the scale requested.

/// The graph point currently displayed at viewport point `v`: the inverse of [`apply`].
fn graph_point_under(view: ViewTransform, v: Point) -> Point {
    Point::new((v.x - view.tx) / view.scale, (v.y - view.ty) / view.scale)
}

/// Whether zooming `start` by `factor` about `anchor` leaves the same graph point under `anchor`.
fn anchor_is_preserved(start: ViewTransform, factor: f64, anchor: Point) -> Result<ViewTransform, String> {
    let before = graph_point_under(start, anchor);
    let zoomed = start.zoomed_about(factor, anchor);
    let after = graph_point_under(zoomed, anchor);
    check(
        close(before.x, after.x) && close(before.y, after.y),
        &format!("anchor {anchor:?} drifted from graph point {before:?} to {after:?} (factor {factor})"),
    )?;
    Ok(zoomed)
}

/// The classic boundary bug: at 3.9x, a requested 1.25x would give 4.875x, but the scale is clamped to 4.0x. The
/// translation must use the applied factor (4.0 / 3.9), not the requested 1.25, or the point under the cursor jumps
/// exactly when the limit is reached.
#[test]
fn zooming_in_past_the_maximum_keeps_the_anchor_fixed_using_the_clamped_scale() -> Result<(), String> {
    let start = ViewTransform {
        scale: 3.9,
        tx: -140.0,
        ty: 55.0,
    };
    let anchor = Point::new(210.0, 90.0);
    check(
        start.scale * ZOOM_STEP > MAX_SCALE,
        "test setup: the requested scale must exceed the maximum",
    )?;

    let zoomed = anchor_is_preserved(start, ZOOM_STEP, anchor)?;
    check(zoomed.scale == MAX_SCALE, "the scale was not clamped to exactly MAX_SCALE")?;
    // The translation follows the closed form with the *applied* factor.
    let applied = MAX_SCALE / 3.9;
    check(
        close(zoomed.tx, anchor.x - applied * (anchor.x - start.tx)),
        "tx does not follow t' = a - (s'/s)(a - t)",
    )?;
    check(
        close(zoomed.ty, anchor.y - applied * (anchor.y - start.ty)),
        "ty does not follow t' = a - (s'/s)(a - t)",
    )
}

/// The minimum-scale counterpart: at 0.26x, a requested 1/1.25 would give 0.208x, clamped to 0.25x.
#[test]
fn zooming_out_past_the_minimum_keeps_the_anchor_fixed_using_the_clamped_scale() -> Result<(), String> {
    let start = ViewTransform {
        scale: 0.26,
        tx: 33.0,
        ty: -8.0,
    };
    let anchor = Point::new(-40.0, 120.0);
    check(
        start.scale / ZOOM_STEP < MIN_SCALE,
        "test setup: the requested scale must fall below the minimum",
    )?;

    let zoomed = anchor_is_preserved(start, 1.0 / ZOOM_STEP, anchor)?;
    check(zoomed.scale == MIN_SCALE, "the scale was not clamped to exactly MIN_SCALE")?;
    let applied = MIN_SCALE / 0.26;
    check(
        close(zoomed.tx, anchor.x - applied * (anchor.x - start.tx)),
        "tx does not follow t' = a - (s'/s)(a - t)",
    )
}

/// Already at a limit, so the applied factor is exactly 1.0: nothing may move at all.
#[test]
fn zooming_further_at_a_limit_changes_nothing() -> Result<(), String> {
    let anchor = Point::new(75.0, -25.0);
    let at_max = ViewTransform {
        scale: MAX_SCALE,
        tx: 12.0,
        ty: -34.0,
    };
    check(
        at_max.zoomed_about(ZOOM_STEP, anchor) == at_max,
        "zooming in at MAX_SCALE moved the view",
    )?;
    let at_min = ViewTransform {
        scale: MIN_SCALE,
        tx: -9.0,
        ty: 41.0,
    };
    check(
        at_min.zoomed_about(1.0 / ZOOM_STEP, anchor) == at_min,
        "zooming out at MIN_SCALE moved the view",
    )
}

/// Reaching a limit must land on it exactly, not merely near it.
#[test]
fn the_limits_are_reached_exactly() -> Result<(), String> {
    let anchor = Point::new(10.0, 10.0);
    let mut view = ViewTransform::IDENTITY;
    for _ in 0..40 {
        view = view.zoomed_about(ZOOM_STEP, anchor);
    }
    check(view.scale == 4.0, "did not settle at exactly 4.0")?;
    for _ in 0..80 {
        view = view.zoomed_about(1.0 / ZOOM_STEP, anchor);
    }
    check(view.scale == 0.25, "did not settle at exactly 0.25")?;
    // A single huge request also lands exactly on a limit.
    check(
        ViewTransform::IDENTITY.zoomed_about(1.0e9, anchor).scale == MAX_SCALE,
        "a huge factor did not clamp to MAX_SCALE",
    )?;
    check(
        ViewTransform::IDENTITY.zoomed_about(1.0e-9, anchor).scale == MIN_SCALE,
        "a tiny factor did not clamp to MIN_SCALE",
    )
}

/// Zooming about the origin scales the translation and nothing else moves: `t' = (s' / s) * t`.
#[test]
fn zooming_about_the_origin_only_scales_the_translation() -> Result<(), String> {
    let start = ViewTransform {
        scale: 2.0,
        tx: 40.0,
        ty: -16.0,
    };
    let zoomed = anchor_is_preserved(start, ZOOM_STEP, Point::new(0.0, 0.0))?;
    check(
        close(zoomed.tx, 50.0) && close(zoomed.ty, -20.0),
        &format!("expected (50, -20), got ({}, {})", zoomed.tx, zoomed.ty),
    )?;
    // From the identity, the origin anchor leaves the translation at zero.
    let from_identity = ViewTransform::IDENTITY.zoomed_about(ZOOM_STEP, Point::new(0.0, 0.0));
    check(
        from_identity.tx == 0.0 && from_identity.ty == 0.0,
        "an origin anchor from the identity moved the translation",
    )
}

/// Every quadrant, and both axes signs independently, at several starting scales.
#[test]
fn the_anchor_is_preserved_for_positive_negative_and_mixed_coordinates() -> Result<(), String> {
    let anchors = [
        Point::new(150.0, 90.0),
        Point::new(-150.0, -90.0),
        Point::new(150.0, -90.0),
        Point::new(-150.0, 90.0),
        Point::new(0.0, 200.0),
        Point::new(-300.5, 0.0),
    ];
    let starts = [
        ViewTransform::IDENTITY,
        ViewTransform {
            scale: 0.5,
            tx: 70.0,
            ty: -30.0,
        },
        ViewTransform {
            scale: 3.0,
            tx: -220.0,
            ty: 180.0,
        },
    ];
    for start in starts {
        for anchor in anchors {
            anchor_is_preserved(start, ZOOM_STEP, anchor)?;
            anchor_is_preserved(start, 1.0 / ZOOM_STEP, anchor)?;
            anchor_is_preserved(start, 1.7, anchor)?;
        }
    }
    Ok(())
}

/// Pan then zoom: the pan is part of the view being zoomed, so the anchor still holds, and the earlier pan is scaled
/// about the anchor rather than lost.
#[test]
fn zooming_after_a_pan_preserves_the_anchor() -> Result<(), String> {
    let anchor = Point::new(120.0, 60.0);
    let panned = ViewTransform::IDENTITY.translated(45.0, -30.0);
    let zoomed = anchor_is_preserved(panned, ZOOM_STEP, anchor)?;
    // The graph point that was under the anchor after the pan is still there.
    let expected = graph_point_under(panned, anchor);
    let now = graph_point_under(zoomed, anchor);
    check(
        close(expected.x, now.x) && close(expected.y, now.y),
        "the panned graph point was not kept under the anchor",
    )
}

/// Zoom then pan: a pan moves everything by exactly the drag amount on screen, at whatever scale it is now.
#[test]
fn panning_after_a_zoom_moves_the_view_by_exactly_the_pan_amount() -> Result<(), String> {
    let zoomed = ViewTransform::IDENTITY.zoomed_about(ZOOM_STEP, Point::new(200.0, 150.0));
    let panned = zoomed.translated(-17.0, 23.0);
    check(panned.scale == zoomed.scale, "a pan changed the scale")?;
    let p = Point::new(31.0, 47.0);
    let (before, after) = (apply(zoomed, p), apply(panned, p));
    check(
        close(after.x - before.x, -17.0) && close(after.y - before.y, 23.0),
        "a pan did not move the content by exactly its amount on screen",
    )
}

/// Zoom and pan interleaved in either order end at the same place only when they commute — which they do not, so
/// the order matters and each must apply against the view as it stands.
#[test]
fn zoom_and_pan_apply_in_sequence_against_the_current_view() -> Result<(), String> {
    let anchor = Point::new(100.0, 100.0);
    let zoom_then_pan = ViewTransform::IDENTITY.zoomed_about(2.0, anchor).translated(10.0, 0.0);
    let pan_then_zoom = ViewTransform::IDENTITY.translated(10.0, 0.0).zoomed_about(2.0, anchor);
    // zoom-then-pan: t = 100 - 2 * 100 = -100, then + 10 = -90. pan-then-zoom: t = 100 - 2 * (100 - 10) = -80.
    check(
        close(zoom_then_pan.tx, -90.0),
        &format!("zoom-then-pan tx was {}", zoom_then_pan.tx),
    )?;
    check(
        close(pan_then_zoom.tx, -80.0),
        &format!("pan-then-zoom tx was {}", pan_then_zoom.tx),
    )
}

/// The identity is what a reset restores, and it writes as the identity attribute.
#[test]
fn the_identity_is_the_default_and_writes_as_the_identity_transform() -> Result<(), String> {
    check(
        ViewTransform::default() == ViewTransform::IDENTITY,
        "the default is not the identity",
    )?;
    let mut out = String::new();
    ViewTransform::IDENTITY.write_attr(&mut out);
    check(out == "translate(0, 0) scale(1)", &out)?;

    // Whatever the view has become, the identity is fully independent of it.
    let arbitrary = ViewTransform::IDENTITY
        .zoomed_about(ZOOM_STEP, Point::new(37.0, 19.0))
        .translated(-80.0, 12.0)
        .zoomed_about(1.0 / ZOOM_STEP / ZOOM_STEP, Point::new(-5.0, 300.0));
    check(
        arbitrary != ViewTransform::IDENTITY,
        "test setup: the arbitrary view is still the identity",
    )?;
    let p = Point::new(12.0, 34.0);
    let restored = apply(ViewTransform::IDENTITY, p);
    check(restored.x == p.x && restored.y == p.y, "the identity moved a point")
}

/// Zooming in then out by the same step, about any anchor, returns to the start.
#[test]
fn zoom_in_then_out_about_an_arbitrary_anchor_returns_to_the_start() -> Result<(), String> {
    let start = ViewTransform {
        scale: 1.7,
        tx: -63.0,
        ty: 28.0,
    };
    let anchor = Point::new(-90.0, 140.0);
    let back = start.zoomed_about(ZOOM_STEP, anchor).zoomed_about(1.0 / ZOOM_STEP, anchor);
    check(
        close(back.scale, start.scale) && close(back.tx, start.tx) && close(back.ty, start.ty),
        &format!("expected {start:?}, got {back:?}"),
    )
}

/// A thousand in/out cycles, alternating anchors, must not accumulate significant error.
#[test]
fn repeated_zoom_cycles_do_not_accumulate_error() -> Result<(), String> {
    let start = ViewTransform {
        scale: 1.3,
        tx: 21.0,
        ty: -55.0,
    };
    let anchors = [Point::new(180.0, 95.0), Point::new(-40.0, 310.0), Point::new(0.0, 0.0)];
    let mut view = start;
    for i in 0..1000 {
        let anchor = anchors[i % anchors.len()];
        view = view.zoomed_about(ZOOM_STEP, anchor).zoomed_about(1.0 / ZOOM_STEP, anchor);
    }
    check(
        (view.scale - start.scale).abs() < 1.0e-9
            && (view.tx - start.tx).abs() < 1.0e-6
            && (view.ty - start.ty).abs() < 1.0e-6,
        &format!("drifted from {start:?} to {view:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan or zoom writes this attribute for every animation frame. Once its buffer is large enough it must be reused, not
/// reallocated, however the transform changes.
#[test]
fn writing_the_transform_attribute_reuses_its_buffer() -> Result<(), String> {
    let mut buffer = String::with_capacity(96);
    let (pointer, capacity) = (buffer.as_ptr(), buffer.capacity());

    let mut view = ViewTransform::IDENTITY;
    for step in 0..200 {
        let anchor = Point::new(f64::from(step) * 3.7 - 200.0, f64::from(step) * -1.3 + 90.0);
        view = view.zoomed_about(if step % 2 == 0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP }, anchor);
        view = view.translated(f64::from(step) * 0.37, -f64::from(step) * 0.11);
        view.write_attr(&mut buffer);
        check(buffer.starts_with("translate("), &buffer)?;
    }
    check(buffer.as_ptr() == pointer, "the buffer was moved to a new allocation")?;
    check(buffer.capacity() == capacity, "the buffer's capacity changed")
}
