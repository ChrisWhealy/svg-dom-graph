//! An Argand diagram: the complex plane, drawn as SVG, with a number's real and imaginary parts, its modulus and its
//! angle marked on it and labelled with their values.
//!
//! Everything that is arithmetic is in [`Plane`] and the small functions beside it, and touches no DOM, so it runs under
//! a plain `cargo test`. [`draw`] is the part that makes elements.

use std::f64::consts::PI;
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};
use svg_dom_graph::complex::Complex;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const GRID_STROKE: &str = "#e3e8f1";
const AXIS_STROKE: &str = "#4a5568";
const PLANE_FILL: &str = "#fbfcfe";
const PLANE_STROKE: &str = "#c7cfdf";
const TEXT_FILL: &str = "#1a202c";
const TICK_FILL: &str = "#5a6578";
/// The real part and the imaginary part: the two legs of the right triangle.
const REAL_STROKE: &str = "#2f855a";
const IMAGINARY_STROKE: &str = "#805ad5";
/// The modulus, and the angle.
const MODULUS_STROKE: &str = "#2b6cb0";
const ANGLE_STROKE: &str = "#c05621";
const POINT_FILL: &str = "#c53030";
const ROOT_FILL: &str = "#2b6cb0";

const FONT_FAMILY: &str = "ui-monospace, Menlo, Consolas, monospace";
const LABEL_SIZE: f64 = 12.0;
const TICK_SIZE: f64 = 10.0;
const POINT_RADIUS: f64 = 4.5;

/// How far an arrowhead's tip is past the end of its axis.
const ARROW_OVERHANG: f64 = 4.0;

/// The angle arc is this far from the origin at most, in user units, so that it does not swamp a short modulus.
const MAX_ARC_RADIUS: f64 = 34.0;

/// How far beyond its arc an angle's label sits.
const ANGLE_LABEL_GAP: f64 = 30.0;

/// An angle whose half is less than this many radians, about 29 degrees, has its label lifted off the real axis by
/// [`ANGLE_LABEL_LIFT`] user units.
const SMALL_BISECTOR: f64 = 0.5;
const ANGLE_LABEL_LIFT: f64 = 12.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How the complex plane maps onto a square of the page: where the origin is, and how many user units a unit of the
/// plane takes. The imaginary axis points up, as it is drawn in a book, and the page's own `y` points down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Plane {
    pub centre: Point,
    pub scale: f64,
}

impl Plane {
    /// The plane of a square with corner `top_left` and sides of `side`, that shows `-extent` to `extent` on each axis.
    pub(super) fn new(top_left: Point, side: f64, extent: f64) -> Self {
        Self {
            centre: Point::new(top_left.x + side / 2.0, top_left.y + side / 2.0),
            scale: side / (2.0 * extent),
        }
    }

    /// Where `z` is on the page.
    pub(super) fn to_screen(self, z: Complex) -> Point {
        Point::new(self.centre.x + z.re * self.scale, self.centre.y - z.im * self.scale)
    }

    /// The point `distance` user units from the origin, at `angle` radians anticlockwise from the real axis.
    pub(super) fn polar_point(self, distance: f64, angle: f64) -> Point {
        Point::new(self.centre.x + distance * angle.cos(), self.centre.y - distance * angle.sin())
    }
}

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
/// SVG path data for the arc of a circle of `radius` round `centre`, from `from` to `to` radians anticlockwise from the
/// real axis. An angle that grows runs anticlockwise on the page, which is the arc command's sweep flag of `0`, and one
/// that shrinks runs clockwise, which is `1`. The large-arc flag is set when the arc is more than half a turn.
pub(super) fn arc_path(centre: Point, radius: f64, from: f64, to: f64) -> String {
    let point = |angle: f64| (centre.x + radius * angle.cos(), centre.y - radius * angle.sin());
    let ((x0, y0), (x1, y1)) = (point(from), point(to));
    let large = i32::from((to - from).abs() > PI);
    let sweep = i32::from(to < from);
    format!("M {x0:.2} {y0:.2} A {radius:.2} {radius:.2} 0 {large} {sweep} {x1:.2} {y1:.2}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `value` to at most `places` decimal places, with trailing zeros and a bare point removed, and never `-0`. So `3.0` is
/// `3`, `0.5` is `0.5`, `53.1301` to 2 places is `53.13`, and `-0.0001` to 2 places is `0`.
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
/// Where a label sits when it is beside a point that lies at `angle` from the origin: pushed outwards along that
/// direction by `gap`, and anchored on the side that faces away from the point, so that the text never covers it.
pub(super) fn outward_label(point: Point, angle: f64, gap: f64) -> (Point, TextAnchor) {
    let (cos, sin) = (angle.cos(), -angle.sin());
    let anchor = if cos > 0.35 {
        TextAnchor::Start
    } else if cos < -0.35 {
        TextAnchor::End
    } else {
        TextAnchor::Middle
    };
    // A label above or below the point needs room for the text's own height as well.
    let lift = if cos.abs() <= 0.35 { LABEL_SIZE * 0.4 } else { 0.0 };
    (
        Point::new(point.x + gap * cos, point.y + gap * sin + lift * sin.signum()),
        anchor,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a label for the line from `centre` at `angle` goes, and how far to turn it. The label is `along` user units
/// from `centre` on the line, and `offset` to one side of it: the anticlockwise side if `side` is `1.0`, and the
/// clockwise side if it is `-1.0`. It is turned to run parallel to the line, and never upside down.
///
/// Running along the line, a label stays close to it however long its text is, and does not cross the numbers written
/// beside the axes, as a horizontal one beside a steep line would.
pub(super) fn line_label(centre: Point, angle: f64, along: f64, offset: f64, side: f64) -> (Point, f64) {
    let (cos, sin) = (angle.cos(), angle.sin());
    // The line's direction on the page is `(cos, -sin)`, and the anticlockwise normal to it is `(-sin, -cos)`.
    let at = Point::new(
        centre.x + along * cos - offset * side * sin,
        centre.y - along * sin - offset * side * cos,
    );
    // A positive turn on the page is clockwise, and an angle on the plane is anticlockwise.
    let turn = -angle.to_degrees();
    (at, if cos < 0.0 { turn + 180.0 } else { turn })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What to mark on a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Guides {
    /// Only the dot, and its label.
    None,
    /// The modulus, from the origin to the dot, and the angle, as an arc from the real axis, and the real part and the
    /// imaginary part too: two dashed lines from the dot to the axes.
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

/// A diagram: where it goes, how big it is, and what it shows.
#[derive(Debug, Clone)]
pub(super) struct Argand {
    pub top_left: Point,
    pub side: f64,
    /// The plane shows `-extent` to `extent` on both axes.
    pub extent: f64,
    /// The axes, and the numbers written along them, run from `-axis_half` to `axis_half`, which is less than `extent`
    /// to leave room past their ends for the arrowheads and names. A slider beside an axis is as long as the axis, and
    /// takes this as the greatest value it can give.
    pub axis_half: f64,
    /// Draw the unit circle, and join each mark to the origin by a thin spoke. For the roots of unity.
    pub unit_circle: bool,
    pub marks: Vec<Mark>,
    /// Read out by assistive technology in place of the drawing.
    pub description: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Argand {
    /// How the plane of this diagram maps onto the page.
    pub(super) fn plane(&self) -> Plane {
        Plane::new(self.top_left, self.side, self.extent)
    }

    /// How long each axis is on the page, from one end to the other. The sliders beside the axes are this long.
    pub(super) fn axis_length(&self) -> f64 {
        2.0 * self.axis_half * self.plane().scale
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything one diagram is drawn into: the document's `<svg>`, and the group that the diagram's elements are collected
/// in.
struct Canvas<'a> {
    svg: &'a SvgRoot,
    group: SvgNode,
}

impl Canvas<'_> {
    fn add(&self, node: &SvgNode) -> Result<(), String> {
        self.group.append(node).map_err(|e| e.to_string())
    }

    fn line(&self, from: Point, to: Point, stroke: &str, width: f64, dash: Option<&str>) -> Result<(), String> {
        let line = self.svg.line(from, to).map_err(|e| e.to_string())?;
        line.set_stroke(stroke).map_err(|e| e.to_string())?;
        line.set_stroke_width(width).map_err(|e| e.to_string())?;
        if let Some(dash) = dash {
            line.set_attr("stroke-dasharray", dash).map_err(|e| e.to_string())?;
        }
        self.add(&line)
    }

    fn path(&self, d: &str, stroke: &str, fill: &str, width: f64) -> Result<(), String> {
        let path = self.svg.path(d).map_err(|e| e.to_string())?;
        path.set_fill(fill).map_err(|e| e.to_string())?;
        path.set_stroke(stroke).map_err(|e| e.to_string())?;
        path.set_stroke_width(width).map_err(|e| e.to_string())?;
        self.add(&path)
    }

    /// Text at `at`. A white outline behind the letters, painted first, keeps them legible over a line.
    fn text(&self, at: Point, text: &str, anchor: TextAnchor, size: f64, fill: &str, halo: bool) -> Result<(), String> {
        let el = self.svg.text(at, text).map_err(|e| e.to_string())?;
        el.set_text_anchor(anchor).map_err(|e| e.to_string())?;
        el.set_dominant_baseline(DominantBaseline::Middle).map_err(|e| e.to_string())?;
        el.set_font_family(FONT_FAMILY).map_err(|e| e.to_string())?;
        el.set_font_size(size).map_err(|e| e.to_string())?;
        el.set_fill(fill).map_err(|e| e.to_string())?;
        if halo {
            el.set_stroke(PLANE_FILL).map_err(|e| e.to_string())?;
            el.set_stroke_width(3.5).map_err(|e| e.to_string())?;
            el.set_attr("paint-order", "stroke").map_err(|e| e.to_string())?;
            el.set_attr("stroke-linejoin", "round").map_err(|e| e.to_string())?;
        }
        self.add(&el)
    }

    /// Centred text at `at`, turned `degrees` clockwise about `at`, with the same halo.
    fn text_turned(&self, at: Point, text: &str, degrees: f64, fill: &str) -> Result<(), String> {
        let el = self.svg.text(at, text).map_err(|e| e.to_string())?;
        el.set_text_anchor(TextAnchor::Middle).map_err(|e| e.to_string())?;
        el.set_dominant_baseline(DominantBaseline::Middle).map_err(|e| e.to_string())?;
        el.set_font_family(FONT_FAMILY).map_err(|e| e.to_string())?;
        el.set_font_size(LABEL_SIZE).map_err(|e| e.to_string())?;
        el.set_fill(fill).map_err(|e| e.to_string())?;
        el.set_stroke(PLANE_FILL).map_err(|e| e.to_string())?;
        el.set_stroke_width(3.5).map_err(|e| e.to_string())?;
        el.set_attr("paint-order", "stroke").map_err(|e| e.to_string())?;
        el.set_attr("stroke-linejoin", "round").map_err(|e| e.to_string())?;
        el.set_attr("transform", &format!("rotate({degrees:.2} {:.2} {:.2})", at.x, at.y))
            .map_err(|e| e.to_string())?;
        self.add(&el)
    }

    fn dot(&self, at: Point, fill: &str) -> Result<(), String> {
        let dot = self.svg.circle(at, POINT_RADIUS).map_err(|e| e.to_string())?;
        dot.set_fill(fill).map_err(|e| e.to_string())?;
        dot.set_stroke("#ffffff").map_err(|e| e.to_string())?;
        dot.set_stroke_width(1.5).map_err(|e| e.to_string())?;
        self.add(&dot)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws `diagram` into `svg`, as one `<g role="img">` that is returned, for the caller to put where it belongs. It is
/// described as a whole for assistive technology, which then skips the shapes inside it.
///
/// # Errors
///
/// Returns `Err` if any element cannot be made.
pub(super) fn draw(svg: &SvgRoot, diagram: &Argand) -> Result<SvgNode, String> {
    let group = svg.group().map_err(|e| e.to_string())?;
    group.set_attr("role", "img").map_err(|e| e.to_string())?;
    group.set_attr("aria-label", &diagram.description).map_err(|e| e.to_string())?;
    let canvas = Canvas { svg, group: group.clone() };
    let plane = Plane::new(diagram.top_left, diagram.side, diagram.extent);

    draw_plane(&canvas, diagram, plane)?;
    if diagram.unit_circle {
        let circle = svg.circle(plane.centre, plane.scale).map_err(|e| e.to_string())?;
        circle.set_fill("none").map_err(|e| e.to_string())?;
        circle.set_stroke(PLANE_STROKE).map_err(|e| e.to_string())?;
        circle.set_stroke_width(1.5).map_err(|e| e.to_string())?;
        circle.set_attr("stroke-dasharray", "5 4").map_err(|e| e.to_string())?;
        canvas.add(&circle)?;
    }
    for mark in &diagram.marks {
        draw_mark(&canvas, diagram, plane, mark)?;
    }
    Ok(group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The background, the gridlines, the two axes with their arrowheads and names, and the numbers on them.
fn draw_plane(canvas: &Canvas<'_>, diagram: &Argand, plane: Plane) -> Result<(), String> {
    let Argand { top_left, side, extent, .. } = *diagram;
    let background = canvas.svg.rect(top_left, Size::new(side, side)).map_err(|e| e.to_string())?;
    background.set_fill(PLANE_FILL).map_err(|e| e.to_string())?;
    background.set_stroke(PLANE_STROKE).map_err(|e| e.to_string())?;
    background.set_stroke_width(1.0).map_err(|e| e.to_string())?;
    canvas.add(&background)?;

    let step = tick_step(extent);
    let ticks = (diagram.axis_half / step).floor() as i32;
    for n in -ticks..=ticks {
        let at = f64::from(n) * step;
        let (horizontal, vertical) = (
            plane.to_screen(Complex::new(at, 0.0)).x,
            plane.to_screen(Complex::new(0.0, at)).y,
        );
        if n != 0 {
            canvas.line(
                Point::new(horizontal, top_left.y),
                Point::new(horizontal, top_left.y + side),
                GRID_STROKE,
                1.0,
                None,
            )?;
            canvas.line(
                Point::new(top_left.x, vertical),
                Point::new(top_left.x + side, vertical),
                GRID_STROKE,
                1.0,
                None,
            )?;
        }
    }

    // Each axis runs from `-axis_half` to `axis_half`, and ends in an arrowhead just past that.
    let reach = diagram.axis_half * plane.scale;
    let (left, right) = (plane.centre.x - reach, plane.centre.x + reach);
    let (top, bottom) = (plane.centre.y - reach, plane.centre.y + reach);
    canvas.line(
        Point::new(left, plane.centre.y),
        Point::new(right, plane.centre.y),
        AXIS_STROKE,
        1.5,
        None,
    )?;
    canvas.line(
        Point::new(plane.centre.x, bottom),
        Point::new(plane.centre.x, top),
        AXIS_STROKE,
        1.5,
        None,
    )?;
    let head = |tip: Point, along: (f64, f64)| {
        // Two short strokes back from the tip, 25 degrees either side of the axis.
        let back = (-along.0, -along.1);
        let rotate = |angle: f64| {
            (
                back.0 * angle.cos() - back.1 * angle.sin(),
                back.0 * angle.sin() + back.1 * angle.cos(),
            )
        };
        let (a, b) = (rotate(0.45), rotate(-0.45));
        format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2}",
            tip.x + a.0 * 8.0,
            tip.y + a.1 * 8.0,
            tip.x,
            tip.y,
            tip.x + b.0 * 8.0,
            tip.y + b.1 * 8.0
        )
    };
    canvas.path(
        &head(Point::new(right + ARROW_OVERHANG, plane.centre.y), (1.0, 0.0)),
        AXIS_STROKE,
        "none",
        1.5,
    )?;
    canvas.path(
        &head(Point::new(plane.centre.x, top - ARROW_OVERHANG), (0.0, -1.0)),
        AXIS_STROKE,
        "none",
        1.5,
    )?;
    canvas.text(
        Point::new(right - 4.0, plane.centre.y - 12.0),
        "Re",
        TextAnchor::End,
        LABEL_SIZE,
        AXIS_STROKE,
        true,
    )?;
    canvas.text(
        Point::new(plane.centre.x + 8.0, top + 6.0),
        "Im",
        TextAnchor::Start,
        LABEL_SIZE,
        AXIS_STROKE,
        true,
    )?;

    for n in -ticks..=ticks {
        if n == 0 {
            continue;
        }
        let at = f64::from(n) * step;
        let text = trimmed(at, 1);
        let on_real = plane.to_screen(Complex::new(at, 0.0));
        let on_imaginary = plane.to_screen(Complex::new(0.0, at));
        canvas.text(
            Point::new(on_real.x, on_real.y + 11.0),
            &text,
            TextAnchor::Middle,
            TICK_SIZE,
            TICK_FILL,
            true,
        )?;
        canvas.text(
            Point::new(on_imaginary.x - 7.0, on_imaginary.y),
            &format!("{text}i"),
            TextAnchor::End,
            TICK_SIZE,
            TICK_FILL,
            true,
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One number: its guides first, so that the dot is on top of them, then the dot and its label.
fn draw_mark(canvas: &Canvas<'_>, diagram: &Argand, plane: Plane, mark: &Mark) -> Result<(), String> {
    let z = mark.z;
    let at = plane.to_screen(z);
    let angle = z.im.atan2(z.re);
    let modulus = z.norm();

    if diagram.unit_circle {
        canvas.line(plane.centre, at, PLANE_STROKE, 1.0, None)?;
    }
    if mark.guides.rectangular() {
        let (on_real, on_imaginary) = (
            plane.to_screen(Complex::new(z.re, 0.0)),
            plane.to_screen(Complex::new(0.0, z.im)),
        );
        canvas.line(at, on_real, REAL_STROKE, 1.5, Some("5 4"))?;
        canvas.line(at, on_imaginary, IMAGINARY_STROKE, 1.5, Some("5 4"))?;
        canvas.text(
            Point::new(on_real.x, on_real.y + if z.im >= 0.0 { 23.0 } else { -13.0 }),
            &format!("a = {}", trimmed(z.re, 2)),
            TextAnchor::Middle,
            LABEL_SIZE,
            REAL_STROKE,
            true,
        )?;
        let imaginary_anchor = if z.re >= 0.0 { TextAnchor::End } else { TextAnchor::Start };
        canvas.text(
            Point::new(on_imaginary.x + if z.re >= 0.0 { -8.0 } else { 8.0 }, on_imaginary.y - 14.0),
            &format!("b = {}", trimmed(z.im, 2)),
            imaginary_anchor,
            LABEL_SIZE,
            IMAGINARY_STROKE,
            true,
        )?;
    }
    if mark.guides.polar() && modulus > 0.0 {
        canvas.line(plane.centre, at, MODULUS_STROKE, 2.0, None)?;
        // The label sits on the far side of the line from the angle arc, which is the side away from the real axis.
        let side = if angle >= 0.0 { 1.0 } else { -1.0 };
        let (label_at, turn) = line_label(plane.centre, angle, modulus * plane.scale * 0.65, 12.0, side);
        canvas.text_turned(label_at, &format!("r = {}", trimmed(modulus, 2)), turn, MODULUS_STROKE)?;

        let radius = (modulus * plane.scale * 0.4).min(MAX_ARC_RADIUS);
        canvas.path(&arc_path(plane.centre, radius, 0.0, angle), ANGLE_STROKE, "none", 2.0)?;
        // Well out from the arc, so that a label below the real axis clears the numbers that are written along it. It
        // starts at the bisector and runs away from the imaginary axis, so it never crosses the numbers on that.
        let bisector = angle / 2.0;
        let mut label_at = plane.polar_point(radius + ANGLE_LABEL_GAP, bisector);
        // A small angle puts its label close to the real axis, where its name and its numbers are, so it is lifted away.
        if bisector.abs() < SMALL_BISECTOR {
            label_at.y -= ANGLE_LABEL_LIFT * bisector.signum();
        }
        let anchor = if bisector.cos() >= 0.0 { TextAnchor::Start } else { TextAnchor::End };
        canvas.text(
            label_at,
            &format!("\u{3b8} = {}\u{b0}", trimmed(angle.to_degrees(), 2)),
            anchor,
            LABEL_SIZE,
            ANGLE_STROKE,
            true,
        )?;
    }
    canvas.dot(at, if diagram.unit_circle { ROOT_FILL } else { POINT_FILL })?;
    let (label_at, anchor) = outward_label(at, angle, POINT_RADIUS + 7.0);
    canvas.text(label_at, &mark.label, anchor, LABEL_SIZE, TEXT_FILL, true)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
