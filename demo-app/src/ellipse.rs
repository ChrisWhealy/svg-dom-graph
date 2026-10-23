//! `panel-ellipse` / `#ellipse-diagram`: circles and ellipses of several shapes, joined to each other and to a
//! rectangle, and a clock ellipse whose connector leaves at the minute a slider chooses. See [`build_ellipse_demo`]'s own
//! doc comment for what it demonstrates.

use demo_common::util::{document, ensure_svg_in, required_element, required_input, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        ClockPosition, ConnectorOptions, ConnectorType, DragOptions, EllipseOptions, EllipseRadii, Scene, Side,
        ViewTransform,
    },
};
use wasm_bindgen::{JsCast, prelude::*};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("ellipse.rs");

/// The size of the scene, and the minute the slider starts at.
const SCENE_SIZE: Size = Size { width: 800.0, height: 460.0 };
const FIRST_MINUTE: u8 = 10;

/// Where the clock ellipse is, and how far from its centre the circle it points at sits.
const CLOCK_CENTRE: Point = Point { x: 600.0, y: 230.0 };
const CLOCK_RADII: (f64, f64) = (110.0, 55.0);
const SATELLITE_DISTANCE: f64 = 185.0;
const SATELLITE_RADIUS: f64 = 17.0;

/// The fill of the clock ellipse, which is the one the slider is about, and of the circle at the end of its connector.
const CLOCK_FILL: &str = "#fff3d6";
const SATELLITE_FILL: &str = "#d9c9f3";

/// One node of the left-hand part of the scene.
#[derive(Clone, Copy)]
enum Shape {
    /// A circle of this radius.
    Circle(f64),
    /// An ellipse with these two radii.
    Ellipse(f64, f64),
    /// A rectangle of this size, which is placed by its top-left corner and not its centre.
    Rectangle(f64, f64),
}

/// Every node that stays where it is put: its label, its shape, and where its centre starts. A rectangle's centre is the
/// centre of the box.
const NODES: [(&str, Shape, (f64, f64)); 5] = [
    ("A", Shape::Circle(22.0), (60.0, 70.0)),
    ("Distance", Shape::Circle(20.0), (190.0, 70.0)),
    ("Tall", Shape::Ellipse(30.0, 55.0), (330.0, 90.0)),
    ("Wide ellipse with a long label", Shape::Ellipse(60.0, 24.0), (190.0, 230.0)),
    ("Rectangle", Shape::Rectangle(110.0, 40.0), (190.0, 400.0)),
];

/// The edges between them, as indices into [`Demo::nodes`], whether each is an elbow (the rest are straight), and the
/// side a rectangle's end is forced to leave by, if any. Index 5 is the clock ellipse, so the last edge runs from the
/// rectangle to it. It leaves by the rectangle's east side. Left to itself, a box as wide as this one leaves by its top,
/// which would send the connector up through the wide ellipse.
const EDGES: [(usize, usize, bool, Option<Side>); 5] = [
    (0, 1, false, None),
    (1, 2, false, None),
    (2, 3, false, None),
    (3, 4, true, None),
    (4, 5, true, Some(Side::East)),
];

/// What the demo finds again when it draws the scene again: the scene, and the nodes in the order they were added.
struct Demo {
    scene: Scene,
    /// The nodes of [`NODES`] in order, then the clock ellipse, then the circle it points at.
    nodes: Vec<NodeId>,
}

thread_local! {
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the ellipse demo: five nodes on the left, and the clock ellipse on the right with a circle that its connector
/// points at.
///
/// Demonstrates [`Scene::add_circle_node`] and [`Scene::add_ellipse_node`]:
///
/// 1. A circle fits its label or grows to hold it, and stays a circle: `A` fits, and `Distance` makes its circle larger.
/// 2. An ellipse of any proportion, tall or wide, keeps its proportions when a long label makes it grow.
/// 3. A straight connector lands on the curved outline, on the line between the two centres. An elbow leaves an
///    ellipse by the nearest of north, east, south and west.
/// 4. An ellipse and a rectangle can be joined in either direction, as the elbows to and from `Rectangle` show.
/// 5. `#ellipse-minute` puts the clock ellipse's end of its connector at a minute from 0 to 59, which is a direction
///    from the centre and not a point on the curve's own parameter.
///
/// Every node can be dragged. The slider draws the scene again, which keeps where each node was dragged to, and the zoom
/// and pan.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#ellipse-diagram-stage` or a control, or if any library call fails.
pub(crate) fn build_ellipse_demo() -> Result<(), String> {
    let document = document()?;
    let demo = draw(&document, FIRST_MINUTE, None)?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    wire_minute_slider(document)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The direction of minute `minute` from the centre of an ellipse, as a unit vector in SVG coordinates, where `y` grows
/// southward. Minute 0 is north, and the minutes run clockwise.
fn direction(minute: u8) -> (f64, f64) {
    let theta = std::f64::consts::TAU * f64::from(minute) / 60.0;
    (theta.sin(), -theta.cos())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the node `id` of `previous` is now, as a centre and the radii or half-sizes that make its box: where the person
/// dragged it to, or where it started. A node's final size is read back, so drawing it again never makes it grow
/// further.
fn current(previous: &Demo, index: usize) -> Option<(Point, Size)> {
    let rect = previous.scene.node_rect(*previous.nodes.get(index)?).ok()?;
    let size = Size::new(rect.size.width / 2.0, rect.size.height / 2.0);
    Some((Point::new(rect.origin.x + size.width, rect.origin.y + size.height), size))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the whole scene with the clock connector at `minute`.
///
/// `previous` is the scene on the page, if there is one. Its zoom and pan are carried over, and so is where each node
/// that can be dragged is, so the slider does not undo a drag. The circle the clock connector points at is not kept: it
/// goes wherever `minute` says.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#ellipse-diagram-stage`, or any library call fails.
fn draw(document: &web_sys::Document, minute: u8, previous: Option<&Demo>) -> Result<Demo, String> {
    ensure_svg_in(document, "ellipse-diagram-stage", "ellipse-diagram", SCENE_SIZE)?;
    let view: Option<ViewTransform> = previous.map(|demo| demo.scene.view());
    required_element(document, "ellipse-diagram")?.set_inner_html("");
    resize_svg(document, "ellipse-diagram", SCENE_SIZE)?;
    let svg = SvgRoot::attach("ellipse-diagram").map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag = DragOptions::default();

    let mut nodes = Vec::with_capacity(NODES.len() + 2);
    for (index, &(label, shape, start)) in NODES.iter().enumerate() {
        let kept = previous.and_then(|demo| current(demo, index));
        let id = match shape {
            Shape::Rectangle(width, height) => {
                let centre = kept.map_or(Point::new(start.0, start.1), |(centre, _)| centre);
                scene
                    .add_node(
                        Point::new(centre.x - width / 2.0, centre.y - height / 2.0),
                        Size::new(width, height),
                        label,
                    )
                    .map_err(stringify)?
            },
            Shape::Circle(radius) => {
                // Drawn again, a circle is as large as it grew to, which is the same in both directions.
                let (centre, radius) =
                    kept.map_or((Point::new(start.0, start.1), radius), |(centre, half)| (centre, half.width));
                scene.add_circle_node(centre, radius, label).map_err(stringify)?
            },
            Shape::Ellipse(rx, ry) => {
                let (centre, radii) = kept
                    .map_or((Point::new(start.0, start.1), EllipseRadii::new(rx, ry)), |(centre, half)| {
                        (centre, EllipseRadii::new(half.width, half.height))
                    });
                scene.add_ellipse_node(centre, radii, label).map_err(stringify)?
            },
        };
        scene.make_draggable_with(id, drag).map_err(stringify)?;
        nodes.push(id);
    }

    // The clock ellipse, and the circle that its connector points at, in the direction of the chosen minute.
    let clock_centre = previous
        .and_then(|demo| current(demo, NODES.len()))
        .map_or(CLOCK_CENTRE, |(centre, _)| centre);
    let clock_radii = previous
        .and_then(|demo| current(demo, NODES.len()))
        .map_or(EllipseRadii::new(CLOCK_RADII.0, CLOCK_RADII.1), |(_, half)| {
            EllipseRadii::new(half.width, half.height)
        });
    let clock = scene
        .add_ellipse_node_with(
            clock_centre,
            clock_radii,
            "Clock",
            EllipseOptions::default().with_fill(Some(CLOCK_FILL)),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(clock, drag).map_err(stringify)?;
    nodes.push(clock);

    let (dx, dy) = direction(minute);
    let satellite_centre = Point::new(
        clock_centre.x + SATELLITE_DISTANCE * dx,
        clock_centre.y + SATELLITE_DISTANCE * dy,
    );
    let satellite = scene
        .add_circle_node_with(
            satellite_centre,
            SATELLITE_RADIUS,
            minute.to_string(),
            EllipseOptions::default().with_fill(Some(SATELLITE_FILL)),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(satellite, drag).map_err(stringify)?;
    nodes.push(satellite);

    for &(from, to, elbow, from_side) in &EDGES {
        let connector_type = if elbow {
            ConnectorType::Elbow { corner_radius: 8.0 }
        } else {
            ConnectorType::Straight
        };
        scene
            .add_edge_with(
                nodes[from],
                nodes[to],
                ConnectorOptions::default()
                    .with_connector_type(connector_type)
                    .with_from_side(from_side),
            )
            .map_err(stringify)?;
    }
    let position = ClockPosition::try_from(minute).map_err(stringify)?;
    scene
        .add_edge_with(
            clock,
            satellite,
            ConnectorOptions::default()
                .with_connector_type(ConnectorType::Straight)
                .with_from_clock(Some(position)),
        )
        .map_err(stringify)?;

    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }
    Ok(Demo { scene, nodes })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the control says about minute `minute`: the angle it is round from north.
fn bearing_text(minute: u8) -> String {
    let degrees = u32::from(minute) * 6;
    let name = match minute {
        0 => " (north)",
        15 => " (east)",
        30 => " (south)",
        45 => " (west)",
        _ => "",
    };
    format!("{degrees}\u{b0} clockwise from north{name}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires `#ellipse-minute` to draw the scene again at the chosen minute, and keeps the two outputs beside it in step.
///
/// The listener captures the page's `document`, and is never dropped: `Closure::forget` leaks it deliberately, for the
/// whole life of the page, the same span [`DEMO`] covers.
///
/// # Errors
///
/// Returns `Err` if the page is missing `#ellipse-minute`, `#ellipse-minute-value` or `#ellipse-minute-bearing`, if the
/// first is not an `<input>`, or if the listener cannot be attached.
fn wire_minute_slider(document: web_sys::Document) -> Result<(), String> {
    let slider = required_input(&document, "ellipse-minute")?;
    let value_output = required_element(&document, "ellipse-minute-value")?;
    let bearing_output = required_element(&document, "ellipse-minute-bearing")?;
    let input = slider.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        let minute: u8 = input.value().parse().unwrap_or(FIRST_MINUTE).min(59);
        value_output.set_text_content(Some(&minute.to_string()));
        bearing_output.set_text_content(Some(&bearing_text(minute)));
        let previous = DEMO.with_borrow_mut(Option::take);
        // On a failure the previous scene's handle is put back, so the page keeps working. Nothing in this demo's
        // geometry is invalid, so a failure here is not expected.
        match draw(&document, minute, previous.as_ref()) {
            Ok(demo) => DEMO.with_borrow_mut(|slot| *slot = Some(demo)),
            Err(_) => DEMO.with_borrow_mut(|slot| *slot = previous),
        }
    });
    slider
        .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the clock slider listener: {e:?}"))?;
    closure.forget();
    Ok(())
}
