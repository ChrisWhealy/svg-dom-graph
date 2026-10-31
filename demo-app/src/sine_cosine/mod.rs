//! `panel-sine-cosine`: three scenes that show sine and cosine to be the height and the width of a point going round
//! the unit circle. See [`build_sine_cosine_demo`] for what the page does, and [`plots`] for what is drawn.

mod plots;

use demo_common::{
    force::animation::AnimationLoop,
    svg_buttons::{self, Button},
    util::{document, ensure_svg_in, required_element, stringify},
};
use plots::{
    SIDE, angle_of, circle_config, circle_items, cosine_config, cosine_items, key_angle, readout, sine_config,
    sine_items, turned,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{PlotConfig, PlotItem, Scene},
};
use wasm_bindgen::{JsCast, closure::Closure, convert::FromWasmAbi};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the configuration of the plots and what is drawn on them.
pub(crate) const FILES: &[crate::SourceFile] = &[crate::source_file!("sine_cosine/plots/mod.rs", "plots/mod.rs")];

/// The angle that C starts at, in degrees: on the right, where the circle meets the horizontal axis, with none of the
/// curves drawn yet. The longest gap between two frames that is counted as time passing, in milliseconds. A page
/// that was in the background for a while resumes from where it was and does not jump.
const FIRST_ANGLE: f64 = 0.0;
const LONGEST_FRAME: f64 = 100.0;

/// The button in the circle scene that stops C and starts it again: its action, where it is, and its colours. It is in
/// the bottom right corner, which the circle does not reach.
const PLAY: &str = "play";
const PLAY_AREA: Rect = Rect {
    origin: Point { x: 316.0, y: 360.0 },
    size: Size { width: 72.0, height: 28.0 },
};
const PLAY_COLOURS: (&str, &str) = ("#ffffff", "#64748b");

/// How near to C, in pixels, a pointer has to be to take hold of it, and the radius of the ring that shows C has the
/// keyboard focus.
const GRAB_RADIUS: f64 = 18.0;
const FOCUS_RADIUS: f64 = 13.0;

/// The ids of the elements that the page declares. Each stage is empty, and gets an `<svg>` of the same name.
const CIRCLE_STAGE: &str = "sine-cosine-circle-stage";
const SINE_STAGE: &str = "sine-cosine-sine-stage";
const COSINE_STAGE: &str = "sine-cosine-cosine-stage";
const READOUT: &str = "sine-cosine-readout";
const ERROR: &str = "sine-cosine-error";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One plot: the scene that holds it and its node, which a new frame draws on.
struct Plot {
    scene: Scene,
    node: NodeId,
}

impl Plot {
    /// Makes a scene in the empty `#{stage}`, and puts a plot node in it that fills the scene.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#{stage}`, or if the scene or the node could not be made.
    fn new(stage: &str, config: PlotConfig) -> Result<Self, String> {
        let svg_id = format!("{stage}-svg");
        ensure_svg_in(&document()?, stage, &svg_id, config.size)?;
        let scene = Scene::new(SvgRoot::attach(&svg_id).map_err(stringify)?).map_err(stringify)?;
        let node = scene.add_plot_node(Point::new(0.0, 0.0), config).map_err(stringify)?;
        Ok(Self { scene, node })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws `items` in place of what the plot held.
    fn draw(&self, items: &[PlotItem]) -> Result<(), String> {
        self.scene.set_plot_items(self.node, items).map_err(stringify)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the page needs after it is built: the three plots, where C is, and whether it is going round.
struct Demo {
    circle: Plot,
    sine: Plot,
    cosine: Plot,
    /// Where C is, in degrees from 0 up to but not including 360.
    angle: Cell<f64>,
    /// When the last frame came, or `None` when the next frame has no earlier one to measure from.
    last_frame: Cell<Option<f64>>,
    playing: Cell<bool>,
    /// Set once, just after the demo is made, since the loop's callback needs the demo.
    animation: OnceCell<Rc<AnimationLoop>>,
    /// The `<svg>` of the circle scene, and the layer of it that the button is in.
    svg: web_sys::Element,
    layer: web_sys::Element,
    /// The control that stands for C in the keyboard's tab order, drawn over C.
    handle: web_sys::Element,
    /// Whether C is held by the pointer.
    dragging: Cell<bool>,
    readout: web_sys::Element,
    error: web_sys::Element,
}

thread_local! {
    static DEMO: RefCell<Option<Rc<Demo>>> = const { RefCell::new(None) };
}

impl Demo {
    /// Draws all three plots and the numbers for C at `degrees`, and remembers that is where C is.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a plot could not be drawn.
    fn show(&self, degrees: f64) -> Result<(), String> {
        self.circle.draw(&circle_items(degrees))?;
        self.sine.draw(&sine_items(degrees))?;
        self.cosine.draw(&cosine_items(degrees))?;
        self.readout.set_text_content(Some(&readout(degrees)));
        self.angle.set(degrees);
        self.move_handle(degrees)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts the handle over C, and tells assistive technology the angle. The handle is not drawn, except for a ring
    /// round C while it has the keyboard focus.
    fn move_handle(&self, degrees: f64) -> Result<(), String> {
        let place = self
            .on_scene(plots::width(degrees), plots::height(degrees))
            .ok_or("C cannot be placed")?;
        let whole = format!("{:.0}", plots::whole(degrees));
        for (name, value) in [
            ("transform", format!("translate({} {})", place.x, place.y)),
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
    /// Moves C on by the time since the last frame, and asks for the next frame. A failure stops the demo.
    fn frame(&self, now: f64) {
        if !self.playing.get() {
            return;
        }
        let elapsed = self
            .last_frame
            .get()
            .map_or(0.0, |before| (now - before).clamp(0.0, LONGEST_FRAME));
        self.last_frame.set(Some(now));
        match self.show(turned(self.angle.get(), elapsed)) {
            Ok(()) => self.request_frame(),
            Err(problem) => self.fail(&problem),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn request_frame(&self) {
        if let Some(animation) = self.animation.get() {
            animation.request();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts or stops C going round, and writes on the button what it will do next.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the button could not be relabelled.
    fn set_playing(&self, playing: bool) -> Result<(), String> {
        self.playing.set(playing);
        self.last_frame.set(None);
        match (playing, self.animation.get()) {
            (true, _) => self.request_frame(),
            (false, Some(animation)) => animation.cancel(),
            (false, None) => {},
        }
        svg_buttons::set_label(&self.layer, PLAY, if playing { "Pause" } else { "Play" })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops or starts C, and says on the page if that went wrong.
    fn toggle(&self) {
        if let Err(problem) = self.set_playing(!self.playing.get()) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the pointer of `event` is in the circle scene, in the scene's own units. The scene is never zoomed or
    /// panned, so that is the position in the `<svg>`, scaled by however much the page has shrunk it.
    fn scene_point(&self, event: &web_sys::PointerEvent) -> Point {
        let rect = self.svg.get_bounding_client_rect();
        let scale = if rect.width() > 0.0 { SIDE / rect.width() } else { 1.0 };
        Point::new(
            (f64::from(event.client_x()) - rect.left()) * scale,
            (f64::from(event.client_y()) - rect.top()) * scale,
        )
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where `place` is in the circle scene, in the scene's own units, or `None` if it cannot be placed.
    fn on_scene(&self, x: f64, y: f64) -> Option<Point> {
        self.circle.scene.plot_point_to_scene(self.circle.node, x, y).ok().flatten()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves C for the key of `event`, if it is one that moves it. C stops going round by itself, as when it is held.
    fn pressed(&self, event: &web_sys::KeyboardEvent) {
        let Some(degrees) = key_angle(&event.key(), self.angle.get()) else {
            return;
        };
        event.prevent_default();
        if self.playing.get() {
            self.toggle();
        }
        if let Err(problem) = self.show(degrees) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether `point` is near enough to C to take hold of it.
    fn is_on_c(&self, point: Point) -> bool {
        let angle = self.angle.get();
        self.on_scene(plots::width(angle), plots::height(angle))
            .is_some_and(|c| (c.x - point.x).hypot(c.y - point.y) <= GRAB_RADIUS)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts C at the angle of `point` from the middle of the circle.
    fn drag_to(&self, point: Point) {
        let Some(middle) = self.on_scene(0.0, 0.0) else {
            return;
        };
        // The scene counts down, and an angle counts up.
        let Some(degrees) = angle_of(point.x - middle.x, middle.y - point.y) else {
            return;
        };
        if let Err(problem) = self.show(degrees) {
            self.fail(&problem);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shows the hand that says C can be taken hold of, while the pointer is near it or holding it.
    fn show_hand(&self, hand: &str) {
        let _ = self.svg.set_attribute("style", &format!("touch-action: none; cursor: {hand};"));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Takes hold of C if the pointer is on it. Holding C stops it going round by itself.
    fn grab(&self, event: &web_sys::PointerEvent) {
        if !self.is_on_c(self.scene_point(event)) {
            return;
        }
        event.prevent_default();
        let _ = self.svg.set_pointer_capture(event.pointer_id());
        self.dragging.set(true);
        self.show_hand("grabbing");
        if self.playing.get() {
            self.toggle();
        }
        self.drag_to(self.scene_point(event));
    }

    /// Moves C with the pointer while it is held, and otherwise shows whether the pointer is over it.
    fn moved(&self, event: &web_sys::PointerEvent) {
        let point = self.scene_point(event);
        if self.dragging.get() {
            self.drag_to(point);
        } else {
            self.show_hand(if self.is_on_c(point) { "grab" } else { "default" });
        }
    }

    /// Lets go of C.
    fn release(&self, event: &web_sys::PointerEvent) {
        if self.dragging.replace(false) {
            let _ = self.svg.release_pointer_capture(event.pointer_id());
            self.moved(event);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Stops C and says why on the page.
    fn fail(&self, problem: &str) {
        self.playing.set(false);
        if let Some(animation) = self.animation.get() {
            animation.cancel();
        }
        self.error
            .set_text_content(Some(&format!("The plots could not be drawn: {problem}")));
        let _ = self.error.remove_attribute("hidden");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the person has asked their system for less motion.
fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|query| query.matches())
}

/// Calls `handler` each time `target` gets `event`, which is an event of type `E`. The listener is never dropped:
/// `Closure::forget` leaks it deliberately, for the whole life of the page, the same span [`DEMO`] covers.
///
/// # Errors
///
/// Returns `Err` if the listener could not be attached.
fn listen<E: FromWasmAbi + 'static>(
    target: &web_sys::EventTarget,
    event: &str,
    handler: impl FnMut(E) + 'static,
) -> Result<(), String> {
    let closure = Closure::<dyn FnMut(E)>::new(handler);
    target
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the {event} listener: {e:?}"))?;
    closure.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the handle in the scene's `layer`: a group that is a slider for assistive technology, and can take the
/// keyboard focus, with a ring in it that is only seen while it has. It starts at the origin of the scene, and
/// [`Demo::show`] moves it over C.
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
            ("class", "sine-cosine-handle".to_owned()),
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the sine and cosine demo: the unit circle on the left, with a point C that goes round it anti-clockwise.
///
/// Sine is the height of C, and cosine is its width. The page shows that with three scenes, each of which has one plot
/// node:
///
/// 1. The circle, 400 pixels square, with the axes from -1.2 to 1.2. C casts dotted lines to the axes, where the width
///    W and the height H are. The button that stops C and starts it again is drawn in this scene too.
/// 2. The sine, to the right of the circle, 430 pixels wide. It has the circle's vertical scale, so the height of C in
///    the circle is at the same level here. The angle runs along the bottom.
/// 3. The cosine, below the circle, 430 pixels high. It has the circle's horizontal scale, so the width of C in the
///    circle is at the same place across here. The angle runs down the middle.
///
/// The circle drives the other two. Each frame, the angle moves on, and all three plots are drawn again from it with
/// [`Scene::set_plot_items`], so the grids and axes are never redrawn. C can be dragged round the circle, or moved with
/// the keys when it has the focus, which stops it going round by itself. Someone who has asked for less motion starts
/// with C still.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if a scene or a listener could not be made.
pub(crate) fn build_sine_cosine_demo() -> Result<(), String> {
    let document = document()?;
    let circle = Plot::new(CIRCLE_STAGE, circle_config())?;
    let svg_id = format!("{CIRCLE_STAGE}-svg");
    let layer = svg_buttons::layer(&document, &svg_id)?;
    let button = Button {
        label: "Pause",
        action: PLAY,
        colours: PLAY_COLOURS,
        pressed: None,
    };
    svg_buttons::add_button(&document, &layer, PLAY_AREA, &button)?;
    let handle = add_handle(&document, &layer)?;

    let demo = Rc::new(Demo {
        circle,
        sine: Plot::new(SINE_STAGE, sine_config())?,
        cosine: Plot::new(COSINE_STAGE, cosine_config())?,
        angle: Cell::new(FIRST_ANGLE),
        last_frame: Cell::new(None),
        playing: Cell::new(false),
        animation: OnceCell::new(),
        svg: required_element(&document, &svg_id)?,
        layer,
        handle,
        dragging: Cell::new(false),
        readout: required_element(&document, READOUT)?,
        error: required_element(&document, ERROR)?,
    });
    // The loop holds the demo weakly, since the demo holds the loop.
    let weak: Weak<Demo> = Rc::downgrade(&demo);
    let animation = AnimationLoop::new(move |now| {
        if let Some(demo) = weak.upgrade() {
            demo.frame(now);
        }
    })?;
    let _ = demo.animation.set(animation);

    demo.show(FIRST_ANGLE)?;
    demo.show_hand("default");

    let pressed = Rc::clone(&demo);
    svg_buttons::watch(&document, CIRCLE_STAGE, move |action| {
        if action == PLAY {
            pressed.toggle();
        }
    })?;
    for event in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
        let held = Rc::clone(&demo);
        listen(&demo.svg, event, move |e: web_sys::PointerEvent| match e.type_().as_str() {
            "pointerdown" => held.grab(&e),
            "pointermove" => held.moved(&e),
            _ => held.release(&e),
        })?;
    }

    let keyed = Rc::clone(&demo);
    listen(&demo.handle, "keydown", move |e: web_sys::KeyboardEvent| keyed.pressed(&e))?;

    demo.set_playing(!prefers_reduced_motion())?;
    DEMO.with_borrow_mut(|slot| *slot = Some(demo));
    Ok(())
}
