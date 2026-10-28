//! An Argand diagram: the complex plane, drawn by a plot node, with a number's real and imaginary parts, its modulus
//! and its angle marked on it and labelled with their values.
//!
//! The plane, its grid, its axes and its tick labels are the library's own: [`Argand::config`] says what the plot node
//! looks like. What is on it is [`Argand::items`]: lines, dots and labels, placed by the value of the number. All of
//! that is arithmetic with no DOM in it, so it runs under a plain `cargo test`.

use std::f64::consts::PI;
use svg_dom::root::utils::Size;
use svg_dom_graph::{
    complex::Complex,
    scene::{Insets, LabelAnchor, PlotAxis, PlotConfig, PlotItem, PlotLabel, PlotLine, PlotPlane, PlotPoints},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const PLANE_STROKE: &str = "#c7cfdf";
const TEXT_FILL: &str = "#1a202c";
/// The real part and the imaginary part: the two legs of the right triangle.
const REAL_STROKE: &str = "#2f855a";
const IMAGINARY_STROKE: &str = "#805ad5";
/// The modulus, and the angle.
const MODULUS_STROKE: &str = "#2b6cb0";
const ANGLE_STROKE: &str = "#c05621";
const POINT_FILL: &str = "#c53030";
const ROOT_FILL: &str = "#2b6cb0";

const LABEL_SIZE: f64 = 12.0;
const POINT_RADIUS: f64 = 4.5;

/// The room between the edge of the node and the plotted area. It holds the arrowheads and the axis names.
pub(super) const PAD: f64 = 14.0;

/// The angle arc is this far from the origin at most, in user units, so that it does not swamp a short modulus.
const MAX_ARC_RADIUS: f64 = 34.0;

/// How far beyond its arc an angle's label sits.
const ANGLE_LABEL_GAP: f64 = 30.0;

/// An angle whose half is less than this many radians, about 29 degrees, has its label lifted off the real axis by
/// [`ANGLE_LABEL_LIFT`] user units.
const SMALL_BISECTOR: f64 = 0.5;
const ANGLE_LABEL_LIFT: f64 = 12.0;

/// How many straight segments draw an arc, and the unit circle.
const ARC_SEGMENTS: usize = 48;
const CIRCLE_SEGMENTS: usize = 128;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The distance between gridlines and tick marks for a plane that shows `-extent` to `extent`: whole units, or halves
/// for a small plane, or twos for a large one, so that there are never so many that their labels run together.
pub(super) fn tick_step(extent: f64) -> f64 {
    if extent <= 2.0 {
        0.5
    } else if extent <= 6.0 {
        1.0
    } else {
        2.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of the arc of a circle of `radius` about the origin, from `from` to `to` radians anticlockwise from the
/// real axis, as numbers of the plane. It is drawn as a line through them.
pub(super) fn arc_points(radius: f64, from: f64, to: f64, segments: usize) -> Vec<Complex> {
    (0..=segments)
        .map(|i| {
            let angle = from + (to - from) * i as f64 / segments as f64;
            Complex::new(radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `value` to at most `places` decimal places, with trailing zeros and a bare point removed, and never `-0`. So `3.0`
/// is `3`, `0.5` is `0.5`, `53.1301` to 2 places is `53.13`, and `-0.0001` to 2 places is `0`.
pub(super) fn trimmed(value: f64, places: usize) -> String {
    let text = format!("{value:.places$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    if text == "-0" { "0".to_owned() } else { text.to_owned() }
}

/// A number written `a + bi` or `a - bi`, each part to `places` decimal places and trimmed.
pub(super) fn rectangular_text(z: Complex, places: usize) -> String {
    let (re, im) = (trimmed(z.re, places), trimmed(z.im.abs(), places));
    let sign = if trimmed(z.im, places).starts_with('-') { '-' } else { '+' };
    format!("{re} {sign} {im}i")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far, in user units, a label beside a point that lies at `angle` from the origin is moved: pushed outwards along
/// that direction by `gap`, and anchored on the side that faces away from the point, so that the text never covers it.
pub(super) fn outward_offset(angle: f64, gap: f64) -> ((f64, f64), LabelAnchor) {
    let (cos, sin) = (angle.cos(), -angle.sin());
    let anchor = if cos > 0.35 {
        LabelAnchor::Start
    } else if cos < -0.35 {
        LabelAnchor::End
    } else {
        LabelAnchor::Middle
    };
    // A label above or below the point needs room for the text's own height as well.
    let lift = if cos.abs() <= 0.35 { LABEL_SIZE * 0.4 } else { 0.0 };
    ((gap * cos, gap * sin + lift * sin.signum()), anchor)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far below the middle of the adjacent leg of `z` its label goes: far enough below the axis to clear the numbers
/// written along it. It is above the axis, where the opposite leg is not, if the number is below the axis, so that the
/// label is never on the side the opposite leg is.
pub(super) fn adjacent_offset(z: Complex) -> f64 {
    if z.im >= 0.0 { 23.0 } else { -13.0 }
}

/// How far, and which way, the label of the opposite leg of `z` is moved from the middle of the leg: to the side away
/// from the origin, so that it is outside the triangle and clear of the modulus and the angle that are inside it.
pub(super) fn opposite_offset(z: Complex) -> (f64, LabelAnchor) {
    if z.re >= 0.0 { (10.0, LabelAnchor::Start) } else { (-10.0, LabelAnchor::End) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a label for the line from the origin at `angle` goes, and how far to turn it. The label is `along` user units
/// from the origin on the line, and `offset` to one side of it: the anticlockwise side if `side` is `1.0`, and the
/// clockwise side if it is `-1.0`. It is turned to run parallel to the line, and never upside down.
///
/// Running along the line, a label stays close to it however long its text is, and does not cross the numbers written
/// beside the axes, as a horizontal one beside a steep line would. The result is the point on the line, in user units
/// along it, then the move to the side in user units of the page, and the turn in degrees.
pub(super) fn line_label(angle: f64, offset: f64, side: f64) -> ((f64, f64), f64) {
    let (cos, sin) = (angle.cos(), angle.sin());
    // The anticlockwise normal to the line, on the page, where `y` points down, is `(-sin, -cos)`.
    let aside = (-offset * side * sin, -offset * side * cos);
    // A positive turn on the page is clockwise, and an angle on the plane is anticlockwise.
    let turn = -angle.to_degrees();
    (aside, if cos < 0.0 { turn + 180.0 } else { turn })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What to mark on a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Guides {
    /// Only the dot, and its label.
    None,
    /// The modulus, from the origin to the dot, and the angle, as an arc from the real axis, and the real part and the
    /// imaginary part too, as the two legs of the right triangle that the number makes with the real axis: the adjacent
    /// one along the real axis, in green, and the opposite one up or down to the dot, in purple.
    Both,
}

impl Guides {
    fn rectangular(self) -> bool {
        self == Self::Both
    }

    fn polar(self) -> bool {
        self == Self::Both
    }
}

/// One number on the diagram.
#[derive(Debug, Clone)]
pub(super) struct Mark {
    pub z: Complex,
    /// Written beside the dot.
    pub label: String,
    pub guides: Guides,
}

/// A diagram: how big its plotted area is, and what it shows.
#[derive(Debug, Clone)]
pub(super) struct Argand {
    /// The plotted area is a square with sides of this many user units.
    pub side: f64,
    /// The plane shows `-extent` to `extent` on both axes.
    pub extent: f64,
    /// Draw the unit circle, and join each mark to the origin by a thin spoke. For the roots of unity.
    pub unit_circle: bool,
    pub marks: Vec<Mark>,
    /// Read out by assistive technology in place of the drawing.
    pub description: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Argand {
    /// How many user units one unit of the plane is.
    pub(super) fn scale(&self) -> f64 {
        self.side / (2.0 * self.extent)
    }

    /// The plot node of this diagram: a complex plane that shows `-extent` to `extent` on both axes, with arrowheads on
    /// its axes, and a square plotted area `side` across. The margin round it is [`PAD`].
    pub(super) fn config(&self) -> PlotConfig {
        let axis = || {
            PlotAxis::linear(-self.extent, self.extent)
                .with_major_interval(tick_step(self.extent))
                .with_arrow(true)
        };
        let config = PlotConfig::new(Size::new(self.side + 2.0 * PAD, self.side + 2.0 * PAD), axis(), axis())
            .with_plane(PlotPlane::Complex)
            .with_insets(Insets::new(PAD, PAD, PAD, PAD));
        // A diagram that has not been given a description is named from its axes, until it is.
        if self.description.is_empty() {
            config
        } else {
            config.with_description(self.description.clone())
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What is drawn on the plot: the unit circle if there is one, and then each mark, in order.
    pub(super) fn items(&self) -> Vec<PlotItem> {
        let mut items = Vec::new();
        if self.unit_circle {
            let circle = arc_points(1.0, 0.0, 2.0 * PI, CIRCLE_SEGMENTS);
            items.push(PlotLine::new(circle, PLANE_STROKE).with_width(1.5).with_dash("5 4").into());
        }
        for mark in &self.marks {
            self.mark_items(mark, &mut items);
        }
        items
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// One number: its guides first, so that the dot is on top of them, then the dot and its label.
    fn mark_items(&self, mark: &Mark, items: &mut Vec<PlotItem>) {
        let z = mark.z;
        let scale = self.scale();
        let angle = z.im.atan2(z.re);
        let modulus = z.norm();
        let line = |points: [Complex; 2], colour: &str, width: f64| -> PlotItem {
            PlotLine::new(points, colour).with_width(width).into()
        };

        if self.unit_circle {
            items.push(line([Complex::ZERO, z], PLANE_STROKE, 1.0));
        }
        if mark.guides.rectangular() {
            // The right triangle that the number makes with the real axis, as a rectangular cell's picture draws it:
            // the adjacent leg along the real axis from the origin, in green, and then the opposite leg up or down to
            // the number, in purple. Each leg is the colour of the part it stands for, and carries that part's label.
            let corner = Complex::new(z.re, 0.0);
            items.push(line([Complex::ZERO, corner], REAL_STROKE, 3.0));
            items.push(line([corner, z], IMAGINARY_STROKE, 2.5));
            items.push(
                PlotLabel::new(Complex::new(z.re / 2.0, 0.0), format!("a = {}", trimmed(z.re, 2)))
                    .with_colour(REAL_STROKE)
                    .with_anchor(LabelAnchor::Middle)
                    .with_offset(0.0, adjacent_offset(z))
                    .into(),
            );
            let (beside, anchor) = opposite_offset(z);
            items.push(
                PlotLabel::new(Complex::new(z.re, z.im / 2.0), format!("b = {}", trimmed(z.im, 2)))
                    .with_colour(IMAGINARY_STROKE)
                    .with_anchor(anchor)
                    .with_offset(beside, 0.0)
                    .into(),
            );
        }
        if mark.guides.polar() && modulus > 0.0 {
            items.push(line([Complex::ZERO, z], MODULUS_STROKE, 2.0));
            // The label sits on the far side of the line from the angle arc, which is the side away from the real axis.
            let side = if angle >= 0.0 { 1.0 } else { -1.0 };
            let (aside, turn) = line_label(angle, 12.0, side);
            let along = Complex::from_polar(modulus * 0.65, angle);
            items.push(
                PlotLabel::new(along, format!("r = {}", trimmed(modulus, 2)))
                    .with_colour(MODULUS_STROKE)
                    .with_anchor(LabelAnchor::Middle)
                    .with_offset(aside.0, aside.1)
                    .with_rotation(turn)
                    .into(),
            );

            let radius = (modulus * scale * 0.4).min(MAX_ARC_RADIUS);
            items.push(
                PlotLine::new(arc_points(radius / scale, 0.0, angle, ARC_SEGMENTS), ANGLE_STROKE)
                    .with_width(2.0)
                    .into(),
            );
            // Well out from the arc, so that a label below the real axis clears the numbers that are written along it.
            // It starts at the bisector and runs away from the imaginary axis, so it never crosses the numbers on that.
            let bisector = angle / 2.0;
            // A small angle puts its label close to the real axis, where its name and its numbers are, so it is lifted
            // away.
            let lift = if bisector.abs() < SMALL_BISECTOR {
                -ANGLE_LABEL_LIFT * bisector.signum()
            } else {
                0.0
            };
            items.push(
                PlotLabel::new(
                    Complex::from_polar((radius + ANGLE_LABEL_GAP) / scale, bisector),
                    format!("\u{3b8} = {}\u{b0}", trimmed(angle.to_degrees(), 2)),
                )
                .with_colour(ANGLE_STROKE)
                .with_anchor(if bisector.cos() >= 0.0 { LabelAnchor::Start } else { LabelAnchor::End })
                .with_offset(0.0, lift)
                .into(),
            );
        }
        let fill = if self.unit_circle { ROOT_FILL } else { POINT_FILL };
        items.push(PlotPoints::new([z], fill).with_radius(POINT_RADIUS).into());
        let ((dx, dy), anchor) = outward_offset(angle, POINT_RADIUS + 7.0);
        items.push(
            PlotLabel::new(z, mark.label.clone())
                .with_colour(TEXT_FILL)
                .with_anchor(anchor)
                .with_offset(dx, dy)
                .into(),
        );
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
