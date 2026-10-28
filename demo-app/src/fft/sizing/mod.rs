//! How a complex number is drawn in the FFT panel's nodes, and how a node's size is chosen.
//!
//! A node never changes size after it is drawn. So each is drawn for the widest number it will ever show, and is then
//! given the number it should show now. The library measures to find the widest number, and nothing counts characters.
//! A minus sign or an `i` can come from a font whose width is not that of the digits.

use super::layout::widest_by;
use demo_common::{complex::Complex, util::stringify};
use std::collections::HashSet;
use svg_dom_graph::scene::{ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent, NodeValues, Scene};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many lines of text high the pictures in the nodes are. Smaller than the Butterfly panel's, since an overview of
/// sixteen lanes has many nodes.
const PICTURE_SCALE: f64 = 0.8;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A part of a number that is zero to the three decimals a node shows, written as a plain zero. Without this, the
/// conjugate of a root such as 1 would be drawn as `1.000 - 0.000i`, and a rounding error of `-1e-17` the same.
fn tidy_part(x: f64) -> f64 {
    if x.abs() < 5e-4 { 0.0 } else { x }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The number as a node shows it: see [`tidy_part`].
pub(super) fn tidy(z: Complex) -> Complex {
    Complex::new(tidy_part(z.re), tidy_part(z.im))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content of a node that holds one complex number.
pub(super) fn content(z: Complex) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::Complex64(vec![tidy(z)]),
        DataFormat::ComplexNumber(
            ComplexNumberFormat::Rectangular,
            ComplexStyle::default().with_icon_scale(PICTURE_SCALE),
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values of a node that holds one complex number.
pub(super) fn only(z: Complex) -> NodeValues {
    NodeValues::Complex64(vec![tidy(z)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a number is written in a node: three decimals for each part. Two numbers with the same text are the same width.
fn written(z: Complex) -> (String, String) {
    (format!("{:.3}", z.re), format!("{:.3}", z.im))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The number among `values` that a node of this `name` is widest for, measured by the library. Each distinct text is
/// measured once.
///
/// # Errors
///
/// Returns `Err` if a measurement fails.
pub(super) fn widest_value(
    scene: &Scene,
    name: &str,
    values: impl IntoIterator<Item = Complex>,
) -> Result<Complex, String> {
    let mut seen = HashSet::new();
    let distinct = values.into_iter().map(tidy).filter(|&z| seen.insert(written(z)));
    let widest = widest_by(distinct, |&z| {
        scene
            .measure_named_data_node(name, &content(z))
            .map(|size| size.width)
            .map_err(stringify)
    })?;
    Ok(widest.unwrap_or(Complex::ZERO))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
