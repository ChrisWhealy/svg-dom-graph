//! Where a selection toolbar's bar and its three buttons sit within a scene's visible area, as pure arithmetic.
//!
//! Kept free of any DOM dependency, so it stays testable with a plain `cargo test`. The arrangement itself is N buttons
//! along one edge of an area, centred, inset by a margin, `gap` apart. It is identical to
//! [`super::super::toolbar::layout`]'s own. This is a deliberate, separately-tested copy rather than a shared
//! dependency between the two toolbars. So either one's own geometry can change without touching the other's.

use super::Side;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a selection toolbar's bar and each of its buttons sit.
#[derive(Debug, PartialEq)]
pub(super) struct SelectionToolbarLayout {
    /// The bar's own outer rectangle, in the `<svg>`'s user space.
    pub bar: Rect,
    /// Each button's rectangle, in the same order as the sizes passed in — relative to `bar.origin`.
    pub buttons: Vec<Rect>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lays out buttons of the given `sizes` as one bar against `edge` of `area`, centred along that edge and inset from it
/// by `margin`.
///
/// A North or South bar runs its buttons left to right. An East or West bar stacks them top to bottom, and gives every
/// button the width of the widest so the column lines up. Buttons are `gap` apart.
///
/// The bar never starts before `area`'s own origin, even when `area` is too small to hold it.
pub(super) fn layout(edge: Side, area: Rect, sizes: &[Size], gap: f64, margin: f64) -> SelectionToolbarLayout {
    let horizontal = matches!(edge, Side::North | Side::South);
    let gaps = gap * sizes.len().saturating_sub(1) as f64;
    let max_width = sizes.iter().map(|s| s.width).fold(0.0, f64::max);
    let max_height = sizes.iter().map(|s| s.height).fold(0.0, f64::max);

    let mut buttons = Vec::with_capacity(sizes.len());
    let mut cursor = 0.0;
    for size in sizes {
        if horizontal {
            buttons.push(Rect {
                origin: Point::new(cursor, 0.0),
                size: Size::new(size.width, max_height),
            });
            cursor += size.width + gap;
        } else {
            buttons.push(Rect {
                origin: Point::new(0.0, cursor),
                size: Size::new(max_width, size.height),
            });
            cursor += size.height + gap;
        }
    }

    let bar_size = if horizontal {
        Size::new(sizes.iter().map(|s| s.width).sum::<f64>() + gaps, max_height)
    } else {
        Size::new(max_width, sizes.iter().map(|s| s.height).sum::<f64>() + gaps)
    };

    let (area_x, area_y) = (area.origin.x, area.origin.y);
    let (area_w, area_h) = (area.size.width, area.size.height);
    let centred_x = area_x + (area_w - bar_size.width) / 2.0;
    let centred_y = area_y + (area_h - bar_size.height) / 2.0;

    let origin = match edge {
        Side::North => Point::new(centred_x, area_y + margin),
        Side::South => Point::new(centred_x, area_y + area_h - margin - bar_size.height),
        Side::West => Point::new(area_x + margin, centred_y),
        Side::East => Point::new(area_x + area_w - margin - bar_size.width, centred_y),
    };

    SelectionToolbarLayout {
        bar: Rect {
            origin: Point::new(origin.x.max(area_x), origin.y.max(area_y)),
            size: bar_size,
        },
        buttons,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
