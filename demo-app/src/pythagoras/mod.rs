//! `panel-pythagoras`: the unit circle with a right triangle in it and a square on each side of the triangle, which are
//! drawn again as point C is dragged round. See [`build_pythagoras_demo`] for what the page does, and [`figure`] for
//! what is drawn.

mod figure;

use crate::circle_control::{CircleControl, angles::whole};
use demo_common::util::{document, ensure_svg_in};
use figure::{Figure, Point as Unit, areas_text, figure, middle, sum_text};
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the triangle and the squares, and the circle that a person can move C
/// round, which the sine and cosine demo and the phase shift demo have too.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("pythagoras/figure/mod.rs", "figure/mod.rs"),
    crate::source_file!("circle_control/mod.rs", "../circle_control/mod.rs"),
    crate::source_file!("circle_control/angles/mod.rs", "../circle_control/angles/mod.rs"),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle that C starts at, in degrees.
const FIRST_ANGLE: f64 = 45.0;

/// The id of the empty stage that the page declares, and of the `<svg>` that is made in it.
const STAGE: &str = "pythagoras-stage";
const SVG: &str = "pythagoras-stage-svg";

/// The side of the scene, in pixels, where the middle of the circle is, and how long the radius is. The scene has room
/// for the greatest that the squares reach, which is a corner of the square on the hypotenuse, `sqrt(2)` radii from the
/// middle, and for the writing at the top.
const SIDE: f64 = 560.0;
const CENTRE: Point = Point { x: SIDE / 2.0, y: 310.0 };
const RADIUS: f64 = 140.0;

/// The colours of the squares: blue for the hypotenuse and green for the other two, each as a fill that lets what is
/// under it be seen, and an outline of the same colour that does not.
const BLUE: &str = "#2563eb";
const GREEN: &str = "#1f7a37";
const FILL_OPACITY: &str = "0.35";

/// The colours of the circle, the triangle, C and the writing.
const CIRCLE: &str = "#64748b";
const TRIANGLE: &str = "#1a202c";
const POINT: &str = "#d63c32";
const INK: &str = "#1a202c";

/// The width of the outline of a square, and of the triangle and the circle, and the radius of the dot for C, in
/// pixels.
const OUTLINE: &str = "1.5";
const TRIANGLE_WIDTH: &str = "2";
const DOT: &str = "6";

/// The length of a side of the mark for the right angle, in pixels, and how short a side of the triangle may be before
/// the mark is left out, since it would be bigger than the triangle.
const MARK: f64 = 12.0;
const SHORTEST: f64 = 1.8 * MARK;

/// The radius of the curve that shows the angle, and of the circle that its label is on, in pixels, and the smallest
/// angle that the curve and its label are drawn for, in degrees.
const ARC_RADIUS: f64 = 32.0;
const LABEL_RADIUS: f64 = 50.0;
const SMALLEST_ARC: f64 = 1.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements that are drawn again each time C moves.
struct Drawing {
    svg: web_sys::Element,
    hypotenuse: web_sys::Element,
    width: web_sys::Element,
    height: web_sys::Element,
    triangle: web_sys::Element,
    mark: web_sys::Element,
    /// The curve at the middle of the circle that shows the angle, and the \u{3b8} beside it.
    arc: web_sys::Element,
    theta: web_sys::Element,
    dot: web_sys::Element,
    /// The writing in each of the three squares, and the two lines at the top.
    hypotenuse_text: web_sys::Element,
    width_text: web_sys::Element,
    height_text: web_sys::Element,
    equation: web_sys::Element,
    numbers: web_sys::Element,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the control that moves C, and what is drawn.
struct Demo {
    circle: Rc<CircleControl>,
    drawing: Drawing,
}

impl Demo {
    /// Draws the triangle and the three squares for C at `degrees`, writes what they say, and puts the handle over C.
    ///
    /// # Errors
    ///
    /// Returns `Err` if an attribute could not be written.
    fn show(&self, degrees: f64) -> Result<(), String> {
        let figure = figure(degrees);
        let drawing = &self.drawing;
        set(&drawing.hypotenuse, "points", &polygon(&figure.hypotenuse))?;
        set(&drawing.width, "points", &polygon(&figure.width))?;
        set(&drawing.height, "points", &polygon(&figure.height))?;
        set(&drawing.triangle, "points", &polygon(&[figure.centre, figure.foot, figure.c]))?;
        set(&drawing.mark, "d", &mark_path(&figure))?;
        set(&drawing.arc, "d", &arc_path(degrees))?;
        let theta = angle_label_at(degrees);
        set(&drawing.theta, "x", &theta.x.to_string())?;
        set(&drawing.theta, "y", &theta.y.to_string())?;
        drawing
            .theta
            .set_text_content(Some(if shows_arc(degrees) { "\u{3b8}" } else { "" }));
        let at = scene(figure.c);
        set(&drawing.dot, "cx", &at.x.to_string())?;
        set(&drawing.dot, "cy", &at.y.to_string())?;

        let (width, height, hypotenuse) = areas_text(&figure);
        for (element, square, name, area) in [
            (&drawing.hypotenuse_text, &figure.hypotenuse, "c", hypotenuse),
            (&drawing.width_text, &figure.width, "a", width),
            (&drawing.height_text, &figure.height, "b", height),
        ] {
            let place = scene(middle(square));
            set(element, "x", &place.x.to_string())?;
            set(element, "y", &place.y.to_string())?;
            element.set_text_content(Some(&format!("{name}\u{b2} = {area}")));
        }
        drawing.equation.set_text_content(Some("a\u{b2} + b\u{b2} = c\u{b2}"));
        let numbers = sum_text(&figure);
        drawing.numbers.set_text_content(Some(&numbers));
        set(&drawing.svg, "aria-label", &label(degrees, &numbers))?;
        self.circle.place(degrees)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where `unit`, which is a place in the units of the circle, is in the scene.
fn scene(unit: Unit) -> Point {
    Point::new(CENTRE.x + RADIUS * unit.0, CENTRE.y - RADIUS * unit.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The corners of a polygon as its `points`.
fn polygon(corners: &[Unit]) -> String {
    corners
        .iter()
        .map(|&corner| {
            let at = scene(corner);
            format!("{},{}", at.x, at.y)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes an SVG element called `tag` with `attributes`.
fn make(document: &web_sys::Document, tag: &str, attributes: &[(&str, &str)]) -> Result<web_sys::Element, String> {
    let element = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), tag)
        .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
    for (name, value) in attributes {
        set(&element, name, value)?;
    }
    Ok(element)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets the attribute `name` of `element` to `value`.
fn set(element: &web_sys::Element, name: &str, value: &str) -> Result<(), String> {
    element
        .set_attribute(name, value)
        .map_err(|e| format!("could not set {name}: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes a text element `size` pixels high that says nothing yet, in `anchor`, with a halo so that it can be read over
/// what is under it.
fn text(
    document: &web_sys::Document,
    at: Point,
    anchor: &str,
    size: &str,
    class: &str,
) -> Result<web_sys::Element, String> {
    make(
        document,
        "text",
        &[
            ("class", class),
            ("x", &at.x.to_string()),
            ("y", &at.y.to_string()),
            ("text-anchor", anchor),
            ("font-size", size),
            ("fill", INK),
            ("stroke", "#ffffff"),
            ("stroke-width", "3"),
            ("paint-order", "stroke"),
            ("dominant-baseline", "middle"),
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A square of the colour `colour`, which is not drawn yet, for the class `class`.
fn square(document: &web_sys::Document, class: &str, colour: &str) -> Result<web_sys::Element, String> {
    make(
        document,
        "polygon",
        &[
            ("class", class),
            ("fill", colour),
            ("fill-opacity", FILL_OPACITY),
            ("stroke", colour),
            ("stroke-width", OUTLINE),
            ("stroke-linejoin", "round"),
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the circle and everything that is drawn again each time C moves, in the empty `<svg>`, behind the handle that
/// the circle control adds after them.
///
/// # Errors
///
/// Returns `Err` if the page has no `#pythagoras-stage`, or if an element could not be made or added.
fn draw_scene(document: &web_sys::Document) -> Result<Drawing, String> {
    ensure_svg_in(document, STAGE, SVG, Size::new(SIDE, SIDE))?;
    let svg = demo_common::util::required_element(document, SVG)?;
    set(&svg, "role", "img")?;
    // None of it takes the pointer, which the circle control listens for on the `<svg>`.
    let shapes = make(document, "g", &[("style", "pointer-events: none;")])?;
    let hypotenuse = square(document, "square-hypotenuse", BLUE)?;
    let width = square(document, "square-width", GREEN)?;
    let height = square(document, "square-height", GREEN)?;
    let circle = make(
        document,
        "circle",
        &[
            ("cx", &CENTRE.x.to_string()),
            ("cy", &CENTRE.y.to_string()),
            ("r", &RADIUS.to_string()),
            ("fill", "none"),
            ("stroke", CIRCLE),
            ("stroke-width", OUTLINE),
        ],
    )?;
    let triangle = make(
        document,
        "polygon",
        &[
            ("class", "triangle"),
            ("fill", "none"),
            ("stroke", TRIANGLE),
            ("stroke-width", TRIANGLE_WIDTH),
            ("stroke-linejoin", "round"),
        ],
    )?;
    let mark = make(
        document,
        "path",
        &[("fill", "none"), ("stroke", TRIANGLE), ("stroke-width", "1.5")],
    )?;
    let arc = make(
        document,
        "path",
        &[("fill", "none"), ("stroke", POINT), ("stroke-width", TRIANGLE_WIDTH)],
    )?;
    let theta = text(document, CENTRE, "middle", "18", "theta")?;
    set(&theta, "font-style", "italic")?;
    let dot = make(document, "circle", &[("r", DOT), ("fill", POINT)])?;
    let hypotenuse_text = text(document, CENTRE, "middle", "15", "text-hypotenuse")?;
    let width_text = text(document, CENTRE, "middle", "15", "text-width")?;
    let height_text = text(document, CENTRE, "middle", "15", "text-height")?;
    let equation = text(document, Point::new(SIDE / 2.0, 30.0), "middle", "18", "equation")?;
    let numbers = text(document, Point::new(SIDE / 2.0, 58.0), "middle", "18", "numbers")?;
    for child in [
        &hypotenuse, &width, &height, &circle, &triangle, &mark, &arc, &theta, &dot, &hypotenuse_text, &width_text,
        &height_text, &equation, &numbers,
    ] {
        shapes
            .append_child(child)
            .map_err(|e| format!("could not add a shape to the scene: {e:?}"))?;
    }
    svg.append_child(&shapes)
        .map_err(|e| format!("could not add the shapes to the scene: {e:?}"))?;
    Ok(Drawing {
        svg,
        hypotenuse,
        width,
        height,
        triangle,
        mark,
        arc,
        theta,
        dot,
        hypotenuse_text,
        width_text,
        height_text,
        equation,
        numbers,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path of the mark for the right angle at the foot of the triangle: a small square in the corner, or nothing if
/// one side of the triangle is too short for it.
fn mark_path(figure: &Figure) -> String {
    let (foot, centre, c) = (scene(figure.foot), scene(figure.centre), scene(figure.c));
    let (along, up) = ((centre.x - foot.x, centre.y - foot.y), (c.x - foot.x, c.y - foot.y));
    let (long_along, long_up) = (along.0.hypot(along.1), up.0.hypot(up.1));
    if long_along < SHORTEST || long_up < SHORTEST {
        return String::new();
    }
    let (a, b) = (
        (along.0 / long_along * MARK, along.1 / long_along * MARK),
        (up.0 / long_up * MARK, up.1 / long_up * MARK),
    );
    format!(
        "M {} {} L {} {} L {} {}",
        foot.x + a.0,
        foot.y + a.1,
        foot.x + a.0 + b.0,
        foot.y + a.1 + b.1,
        foot.x + b.0,
        foot.y + b.1
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle between the radius to C and the horizontal axis on the side that W is, which is always the angle at the
/// middle of the triangle, in degrees from 0 to 90, together with the direction of that side of the axis: 1 for right
/// and -1 for left.
fn triangle_angle(degrees: f64) -> (f64, f64) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let side = if cos < 0.0 { -1.0 } else { 1.0 };
    (sin.abs().atan2(cos.abs()).to_degrees(), side)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the angle at the middle of the triangle at `degrees` is big enough for its curve to be seen.
fn shows_arc(degrees: f64) -> bool {
    triangle_angle(degrees).0 >= SMALLEST_ARC
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path of the curve round the middle of the circle, from the horizontal axis to the radius to C by the short way,
/// which is the angle at the middle of the triangle in every quadrant, or nothing if the angle is too small to show.
fn arc_path(degrees: f64) -> String {
    if !shows_arc(degrees) {
        return String::new();
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (_, side) = triangle_angle(degrees);
    // The curve runs anticlockwise on the screen in quadrants one and three, and clockwise in two and four.
    let sweep = if side * sin < 0.0 { 1 } else { 0 };
    format!(
        "M {} {} A {ARC_RADIUS} {ARC_RADIUS} 0 0 {sweep} {} {}",
        CENTRE.x + side * ARC_RADIUS,
        CENTRE.y,
        CENTRE.x + ARC_RADIUS * cos,
        CENTRE.y - ARC_RADIUS * sin
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the \u{3b8} goes: beside the curve, half way between the axis and the radius to C.
fn angle_label_at(degrees: f64) -> Point {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (_, side) = triangle_angle(degrees);
    let (x, y) = (side + cos, sin);
    let long = x.hypot(y);
    Point::new(CENTRE.x + LABEL_RADIUS * x / long, CENTRE.y - LABEL_RADIUS * y / long)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What assistive technology reads for the scene.
fn label(degrees: f64, numbers: &str) -> String {
    format!(
        "A right triangle in the unit circle, with point C at {:.0} degrees, and a square on each side. The squares on \
         the two shorter sides have areas that add up to the area of the square on the hypotenuse: {numbers}.",
        whole(degrees)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Pythagoras demo: the unit circle, with a point C on it that can be dragged round.
///
/// The page shows it with one scene, 560 pixels square, with no plot, axes or grid in it. The scene has:
///
/// 1. The unit circle, with C on it, starting at 45 degrees. C is dragged round the circle with the pointer, or moved
///    with the keys when it has the focus.
/// 2. A curve at the middle of the circle, labelled \u{3b8}, that shows the angle whose cosine and sine are the sides.
/// 3. The right triangle that the centre, C and the point W under or over C on the horizontal axis make.
/// 4. A square on each of the three sides, away from the triangle. The square on the hypotenuse is a blue that lets
///    what is under it be seen, and the other two are green in the same way. The area of each is written in it.
///
/// Each move of C draws the triangle and the squares again, so the squares grow and shrink as C goes round, and the two
/// green ones always add up to the blue one, which is the theorem of Pythagoras. The hypotenuse is the radius, which is
/// 1, so the blue square is always 1, and the green ones are the cosine squared and the sine squared.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#pythagoras-stage`, or if the scene or a listener could not be made.
pub(crate) fn build_pythagoras_demo() -> Result<(), String> {
    let document = document()?;
    let drawing = draw_scene(&document)?;
    let demo = Rc::new(Demo {
        // The handle is added after the shapes, so that it is over them.
        circle: CircleControl::bare(SVG, SIDE, CENTRE, RADIUS, FIRST_ANGLE)?,
        drawing,
    });
    demo.show(FIRST_ANGLE)?;

    let moved = Rc::clone(&demo);
    demo.circle.wire(move |degrees| {
        // A failure to draw leaves the squares where they were, which is the last angle that could be drawn.
        let _ = moved.show(degrees);
    })?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
