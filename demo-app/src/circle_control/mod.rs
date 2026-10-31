//! The circle that the sine, cosine, phase shift and Pythagoras demos share: a unit circle with point C on it, which
//! the pointer can drag round and the keyboard can move. See [`CircleControl`], and [`angles`] for the numbers behind
//! it.
//!
//! The control only works out where C should be. What a demo draws for that angle is up to the demo, which is told by
//! the function that it gives to [`CircleControl::wire`].

pub(crate) mod angles;

use crate::plot_scene::{Plot, listen};
use angles::{angle_of, height, key_angle, whole, width};
use demo_common::{
    svg_buttons,
    util::{document, required_element},
};
use std::{cell::Cell, rc::Rc};
use svg_dom::root::utils::Point;
use svg_dom_graph::scene::{PlotConfig, PlotItem};

/// How near to C, in pixels, a pointer has to be to take hold of it, and the radius of the ring that shows C has the
/// keyboard focus.
const GRAB_RADIUS: f64 = 18.0;
const FOCUS_RADIUS: f64 = 13.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the unit circle is in the scene, which is what turns the width and the height of C into a place.
enum Placing {
    /// A plot node, which knows where each of its values is.
    Plot(Plot),
    /// A circle drawn by the demo itself, with no plot: its middle, and its radius, in the units of the scene.
    Circle { centre: Point, radius: f64 },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The circle scene, and what lets a person move C round it.
///
/// C can be taken hold of with the pointer and dragged, and it can be reached with the Tab key and moved with the
/// arrow, page, Home and End keys. A handle that is not drawn stands over C for the keyboard and for assistive
/// technology. It is a slider from 0 to 359, and moves with C.
pub(crate) struct CircleControl {
    placing: Placing,
    /// The `<svg>` of the scene, and the layer of it that a demo can add its own controls to.
    pub(crate) svg: web_sys::Element,
    pub(crate) layer: web_sys::Element,
    /// The side of the scene, in the units of the scene.
    side: f64,
    /// The control that stands for C in the keyboard's tab order, drawn over C.
    handle: web_sys::Element,
    /// Where C is, in degrees from 0 up to but not including 360.
    angle: Cell<f64>,
    /// Whether C is held by the pointer.
    dragging: Cell<bool>,
}

impl CircleControl {
    /// Makes the circle scene in the empty `#{stage}`, with the handle over C at the angle `first`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#{stage}`, or if the scene or the handle could not be made.
    pub(crate) fn new(stage: &str, config: PlotConfig, first: f64) -> Result<Rc<Self>, String> {
        let document = document()?;
        let side = config.size.width;
        let plot = Plot::new(stage, config)?;
        let svg_id = format!("{stage}-svg");
        let layer = svg_buttons::layer(&document, &svg_id)?;
        let control = Rc::new(Self {
            placing: Placing::Plot(plot),
            svg: required_element(&document, &svg_id)?,
            handle: add_handle(&document, &layer)?,
            layer,
            side,
            angle: Cell::new(first),
            dragging: Cell::new(false),
        });
        control.show_hand("default");
        Ok(control)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the circle control for a scene that the demo has drawn itself, with no plot in it: the `<svg>`
    /// `#{svg_id}`, which is `side` units square, with a circle of radius `radius` round the point `centre`. The handle
    /// is put in the `<svg>` after whatever is there, so it is over it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#{svg_id}`, or if the handle could not be made.
    pub(crate) fn bare(svg_id: &str, side: f64, centre: Point, radius: f64, first: f64) -> Result<Rc<Self>, String> {
        let document = document()?;
        let svg = required_element(&document, svg_id)?;
        let control = Rc::new(Self {
            placing: Placing::Circle { centre, radius },
            handle: add_handle(&document, &svg)?,
            layer: svg.clone(),
            svg,
            side,
            angle: Cell::new(first),
            dragging: Cell::new(false),
        });
        control.show_hand("default");
        Ok(control)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws `items` on the plot of a circle that has one, in place of what was drawn there before.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the circle has no plot, or if the plot could not be drawn.
    pub(crate) fn draw(&self, items: &[PlotItem]) -> Result<(), String> {
        match &self.placing {
            Placing::Plot(plot) => plot.draw(items),
            Placing::Circle { .. } => Err("this circle has no plot to draw on".to_owned()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where C is, in degrees.
    pub(crate) fn angle(&self) -> f64 {
        self.angle.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Remembers that C is at `degrees`, and puts the handle over it, with the angle for assistive technology. The
    /// demo draws C itself, as part of what it draws on the circle plot.
    ///
    /// # Errors
    ///
    /// Returns `Err` if C cannot be placed on the plot, or the handle cannot be written to.
    pub(crate) fn place(&self, degrees: f64) -> Result<(), String> {
        self.angle.set(degrees);
        let at = self.on_scene(width(degrees), height(degrees)).ok_or("C cannot be placed")?;
        let whole = format!("{:.0}", whole(degrees));
        for (name, value) in [
            ("transform", format!("translate({} {})", at.x, at.y)),
            ("aria-valuenow", whole.clone()),
            ("aria-valuetext", format!("{whole} degrees")),
        ] {
            self.handle
                .set_attribute(name, &value)
                .map_err(|e| format!("could not set {name} on the handle: {e:?}"))?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the pointer and the keyboard work. `on_angle` is called with the angle that a person has moved C to, each
    /// time they do, and the demo is to draw it and call [`place`](Self::place).
    ///
    /// # Errors
    ///
    /// Returns `Err` if a listener could not be attached.
    pub(crate) fn wire(self: &Rc<Self>, on_angle: impl Fn(f64) + 'static) -> Result<(), String> {
        let on_angle = Rc::new(on_angle);
        for event in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
            let (control, on_angle) = (Rc::clone(self), Rc::clone(&on_angle));
            listen(&self.svg, event, move |e: web_sys::PointerEvent| {
                let moved = match e.type_().as_str() {
                    "pointerdown" => control.grab(&e),
                    "pointermove" => control.moved(&e),
                    _ => control.release(&e),
                };
                if let Some(degrees) = moved {
                    on_angle(degrees);
                }
            })?;
        }
        let control = Rc::clone(self);
        listen(&self.handle, "keydown", move |e: web_sys::KeyboardEvent| {
            if let Some(degrees) = control.pressed(&e) {
                on_angle(degrees);
            }
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the pointer of `event` is in the circle scene, in the scene's own units. The scene is never zoomed or
    /// panned, so that is the position in the `<svg>`, scaled by however much the page has shrunk it.
    fn scene_point(&self, event: &web_sys::PointerEvent) -> Point {
        let rect = self.svg.get_bounding_client_rect();
        let scale = if rect.width() > 0.0 { self.side / rect.width() } else { 1.0 };
        Point::new(
            (f64::from(event.client_x()) - rect.left()) * scale,
            (f64::from(event.client_y()) - rect.top()) * scale,
        )
    }

    /// Where the point `(x, y)` of the plot is in the circle scene, in the scene's own units, or `None` if it cannot
    /// be placed.
    fn on_scene(&self, x: f64, y: f64) -> Option<Point> {
        match &self.placing {
            Placing::Plot(plot) => plot.scene.plot_point_to_scene(plot.node, x, y).ok().flatten(),
            // The scene counts down, and a height counts up.
            Placing::Circle { centre, radius } => Some(Point::new(centre.x + radius * x, centre.y - radius * y)),
        }
    }

    /// Whether `point` is near enough to C to take hold of it.
    fn is_on_c(&self, point: Point) -> bool {
        let angle = self.angle.get();
        self.on_scene(width(angle), height(angle))
            .is_some_and(|c| (c.x - point.x).hypot(c.y - point.y) <= GRAB_RADIUS)
    }

    /// The angle of `point` from the middle of the circle, or `None` at the middle.
    fn angle_at(&self, point: Point) -> Option<f64> {
        let middle = self.on_scene(0.0, 0.0)?;
        // The scene counts down, and an angle counts up.
        angle_of(point.x - middle.x, middle.y - point.y)
    }

    /// Shows the hand that says C can be taken hold of, while the pointer is near it or holding it.
    fn show_hand(&self, hand: &str) {
        let _ = self.svg.set_attribute("style", &format!("touch-action: none; cursor: {hand};"));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Takes hold of C if the pointer is on it, and gives the angle that C is then at.
    fn grab(&self, event: &web_sys::PointerEvent) -> Option<f64> {
        let point = self.scene_point(event);
        if !self.is_on_c(point) {
            return None;
        }
        event.prevent_default();
        let _ = self.svg.set_pointer_capture(event.pointer_id());
        self.dragging.set(true);
        self.show_hand("grabbing");
        self.angle_at(point)
    }

    /// Gives the angle of the pointer while C is held, and otherwise shows whether the pointer is over C.
    fn moved(&self, event: &web_sys::PointerEvent) -> Option<f64> {
        let point = self.scene_point(event);
        if self.dragging.get() {
            self.angle_at(point)
        } else {
            self.show_hand(if self.is_on_c(point) { "grab" } else { "default" });
            None
        }
    }

    /// Lets go of C.
    fn release(&self, event: &web_sys::PointerEvent) -> Option<f64> {
        if self.dragging.replace(false) {
            let _ = self.svg.release_pointer_capture(event.pointer_id());
            self.moved(event);
        }
        None
    }

    /// Gives the angle that C moves to for the key of `event`, if it is one that moves it.
    fn pressed(&self, event: &web_sys::KeyboardEvent) -> Option<f64> {
        let degrees = key_angle(&event.key(), self.angle.get())?;
        event.prevent_default();
        Some(degrees)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the handle in the scene's `layer`: a group that is a slider for assistive technology, and can take the
/// keyboard focus, with a ring in it that is only seen while it has. It starts at the origin of the scene, and
/// [`CircleControl::place`] moves it over C.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
fn add_handle(document: &web_sys::Document, layer: &web_sys::Element) -> Result<web_sys::Element, String> {
    let make = |tag: &str, attributes: &[(&str, String)]| -> Result<web_sys::Element, String> {
        let element = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), tag)
            .map_err(|e| format!("could not make a <{tag}>: {e:?}"))?;
        for (name, value) in attributes {
            element
                .set_attribute(name, value)
                .map_err(|e| format!("could not set {name} on a <{tag}>: {e:?}"))?;
        }
        Ok(element)
    };
    let handle = make(
        "g",
        &[
            ("class", "circle-handle".to_owned()),
            ("role", "slider".to_owned()),
            ("tabindex", "0".to_owned()),
            ("aria-label", "Point C, the angle round the circle".to_owned()),
            ("aria-valuemin", "0".to_owned()),
            ("aria-valuemax", "359".to_owned()),
        ],
    )?;
    let ring = make("circle", &[("r", FOCUS_RADIUS.to_string()), ("fill", "transparent".to_owned())])?;
    for (parent, child) in [(&handle, &ring), (layer, &handle)] {
        parent
            .append_child(child)
            .map_err(|e| format!("could not add the handle: {e:?}"))?;
    }
    Ok(handle)
}
