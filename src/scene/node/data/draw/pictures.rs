//! The small Argand pictures drawn in a complex number's cell.

use super::super::super::{CELL_PADDING, render_guard::RenderGuard};
use super::layout::GridLayoutMetrics;
use crate::{
    colours::{PICTURE_AXES_STROKE, PICTURE_IMAGINARY_STROKE, PICTURE_REAL_STROKE, PICTURE_VECTOR_STROKE},
    error::Error,
    model::content::{ICON_GAP, ICON_SIZE, IconKind, axes_path, icon_paths, line_widths},
    scene::CellPicture,
};
use svg_dom::{SvgNode, SvgRoot, root::utils::Point};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the picture at the front of a cell that has `origin` and `size`, for the number `vector` that the cell holds,
/// as a group of the two faint axes and the two paths of the vector. It is decoration, hidden from assistive
/// technology, which is told the number by the cell's text, and it takes no pointer events. Its paths are returned, so
/// that [`update_picture`] can rewrite them when the number changes.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_picture(
    svg: &SvgRoot,
    group: &SvgNode,
    guard: &mut RenderGuard,
    origin: Point,
    layout: &GridLayoutMetrics,
    kind: IconKind,
    vector: Option<(f64, f64)>,
    max_modulus: f64,
) -> Result<CellPicture, Error> {
    // The paths are worked out in a box of `ICON_SIZE`, and the group scales that box to the side the layout gave it.
    // A line is given a width that the scaling takes back to a thin one, and does not grow with the picture.
    let side = layout.picture_room - ICON_GAP;
    let (line_width, axes_width) = line_widths(side / ICON_SIZE);
    let picture = svg.group()?;
    guard.track(picture.clone());
    picture.set_attr(
        "transform",
        &format!(
            "translate({:.2} {:.2}) scale({:.4})",
            origin.x + CELL_PADDING,
            origin.y + (layout.cell.height - side) / 2.0,
            side / ICON_SIZE
        ),
    )?;
    picture.set_attr("aria-hidden", "true")?;
    picture.set_attr("pointer-events", "none")?;

    let axes = svg.path(&axes_path())?;
    axes.set_fill("none")?;
    axes.set_stroke(PICTURE_AXES_STROKE)?;
    axes.set_stroke_width(axes_width)?;
    axes.set_attr("stroke-opacity", "0.55")?;
    picture.append(&axes)?;

    let paths = vector.map(|(re, im)| icon_paths(kind, re, im, max_modulus)).unwrap_or_default();
    let (first_stroke, second_stroke) = match kind {
        IconKind::Polar => (PICTURE_VECTOR_STROKE, PICTURE_VECTOR_STROKE),
        IconKind::Rectangular => (PICTURE_REAL_STROKE, PICTURE_IMAGINARY_STROKE),
    };
    let leg = |d: &str, stroke: &str| -> Result<SvgNode, Error> {
        let path = svg.path(d)?;
        path.set_fill("none")?;
        path.set_stroke(stroke)?;
        path.set_stroke_width(line_width)?;
        path.set_attr("stroke-linecap", "round")?;
        path.set_attr("stroke-linejoin", "round")?;
        picture.append(&path)?;
        Ok(path)
    };
    let (primary, secondary) = (leg(&paths.primary, first_stroke)?, leg(&paths.secondary, second_stroke)?);

    group.append(&picture)?;
    guard.release();
    Ok(CellPicture {
        primary,
        secondary,
        stale: false,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which pictures of a node of `count` pictures a replacement has to write, in order.
///
/// * Every picture, if `rewrite_all`, because the scale they are all drawn to has changed.
/// * Otherwise the pictures of the `changed` cells, and every picture for which `is_stale` is true: one that an earlier
///   replacement began to write and did not finish. Its number is already in the model, so nothing about it looks
///   changed, and only its stale mark says that it is wrong.
///
/// `changed` must be in ascending order, as the cells of a replacement are visited. The two are merged in one pass over
/// the pictures, so nothing is sorted and no list of stale marks is built. An index past `count` is dropped, and none
/// is listed twice.
pub(crate) fn pictures_to_write(
    count: usize,
    changed: &[usize],
    is_stale: impl Fn(usize) -> bool,
    rewrite_all: bool,
) -> Vec<usize> {
    if rewrite_all {
        return (0..count).collect();
    }
    debug_assert!(changed.windows(2).all(|pair| pair[0] <= pair[1]), "changed cells must ascend");
    let mut wanted = Vec::with_capacity(changed.len().min(count));
    let mut next_changed = 0;
    for i in 0..count {
        while next_changed < changed.len() && changed[next_changed] < i {
            next_changed += 1;
        }
        if changed.get(next_changed) == Some(&i) || is_stale(i) {
            wanted.push(i);
        }
    }
    wanted
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rewrites the two paths of `picture` for the number `vector`, drawn to the scale that `max_modulus` sets. A number
/// that cannot be drawn leaves them empty. Drawing a cell and rewriting its number both build the paths here, so the
/// two cannot disagree.
///
/// # Errors
///
/// Returns `Err` if an attribute cannot be set.
pub(crate) fn update_picture(
    picture: &CellPicture,
    kind: IconKind,
    vector: Option<(f64, f64)>,
    max_modulus: f64,
) -> Result<(), svg_dom::Error> {
    let paths = vector.map(|(re, im)| icon_paths(kind, re, im, max_modulus)).unwrap_or_default();
    picture.primary.set_attr("d", &paths.primary)?;
    picture.secondary.set_attr("d", &paths.secondary)
}
