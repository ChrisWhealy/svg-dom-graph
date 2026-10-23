//! `panel-springs`: the Dijkstra graph as a network of weighted springs.
//!
//! Each vertex is a circle of the Dijkstra demo, joined by the same nine edges with the same weights, here read as
//! stiffness. Drag one and its neighbours are pulled and pushed. Released, the network settles under damping and then
//! stops asking for frames.
//!
//! The pieces:
//!
//! - [`fixture`] — the graph, taken from the Dijkstra demo's own definitions, never repeated.
//! - [`core`] — what each command does to the physics, without a browser.
//! - [`controller`] — the one object that touches the scene and the panel, reached only through a command bridge.
//!
//! The numerical model is in `demo-common`'s `force::solver`, and the library only reports the drag: see
//! `Scene::make_draggable_with_observer`.

mod controller;
mod core;
mod fixture;

use crate::{
    dijkstra::{NAMES, VERTEX_RADIUS},
    source_file,
};
use controller::{Controller, ControllerCommand, PAUSE_ACTION, PanelView, RANDOMISE_ACTION, RESET_ACTION};
use core::Core;
use demo_common::{
    force::{animation::AnimationLoop, bridge::CommandBridge},
    svg_buttons::{self, Button},
    util::{MARGIN, document, ensure_svg_in, required_element, stringify},
};
use fixture::Fixture;
use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        CollisionPolicy, ConnectorOptions, ConnectorType, DragObserverHandle, DragOptions, EllipseOptions, Scene, Side,
        ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the graph it uses, what each command does, and the controller that draws.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("springs/fixture/mod.rs", "fixture/mod.rs"),
    source_file!("springs/core/mod.rs", "core/mod.rs"),
    source_file!("springs/controller.rs", "controller.rs"),
];

const STAGE_ID: &str = "springs-diagram-stage";
const DIAGRAM_ID: &str = "springs-diagram";

/// The room east of the graph for the zoom toolbar, as the Dijkstra demo leaves it.
const TOOLBAR_ROOM: f64 = 80.0;

/// The three buttons drawn in the scene, below the graph: their size, the gap between one and the next, and the gap
/// between the graph and the row.
const BUTTON_WIDTH: f64 = 170.0;
const BUTTON_HEIGHT: f64 = 36.0;
const BUTTON_GAP: f64 = 16.0;
const BUTTON_ROW_GAP: f64 = 16.0;

/// What each button is called, what it does, and its fill and outline. Reset is the amber of a change that moves
/// things, Pause is blue, and new weights are the calm green of the Dijkstra demo's.
const BUTTONS: [Button<'static>; 3] = [
    Button {
        label: "Reset",
        action: RESET_ACTION,
        colours: ("#fde3b0", "#a8650a"),
        pressed: None,
    },
    Button {
        label: "Pause",
        action: PAUSE_ACTION,
        colours: ("#dbe7f8", "#3563a8"),
        pressed: Some(false),
    },
    Button {
        label: "Randomise weights",
        action: RANDOMISE_ACTION,
        colours: ("#d4edd6", "#2f7a3a"),
        pressed: None,
    },
];

type Bridge = CommandBridge<ControllerCommand, Controller>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An observer registration: the library's handle, and the lease that queued events carry so that a stale one is known.
/// They are made together and destroyed together.
struct ObserverBinding {
    #[allow(dead_code)] // Held only so that dropping it ends the observation. See `DragObserverHandle`.
    handle: DragObserverHandle,
    #[allow(dead_code)] // Held only so that queued events of this registration go stale when it is dropped.
    lease: Rc<()>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An event listener that removes itself when dropped, so a closure is never left registered after it has gone.
struct Listener {
    target: web_sys::EventTarget,
    event: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl Listener {
    fn new(
        target: &web_sys::EventTarget,
        event: &'static str,
        handler: impl FnMut(&web_sys::Event) + 'static,
    ) -> Result<Self, String> {
        let mut handler = handler;
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| handler(&event));
        target
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach a {event} listener: {e:?}"))?;
        Ok(Self {
            target: target.clone(),
            event,
            closure,
        })
    }
}

impl Listener {
    /// Leaves the listener registered for good. For a listener that runs [`teardown`], which would otherwise drop the
    /// closure that is in the middle of being called: the browser reports that as a closure invoked after it was
    /// dropped.
    fn leave_registered(self) {
        std::mem::forget(self);
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.closure.as_ref().unchecked_ref());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the demo keeps alive for as long as the page is.
struct Runtime {
    bridge: Rc<Bridge>,
    controller: Rc<RefCell<Controller>>,
    bindings: Vec<ObserverBinding>,
    listeners: Vec<Listener>,
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the weighted springs demo: the six circles and nine labelled edges of the Dijkstra graph, each circle
/// draggable, with a controller that moves the free ones under spring forces.
///
/// 1. A circle is dragged by the library's own drag, bounded to the same graph region the physics uses. The drag is
///    observed, so the controller learns where the circle is without a second set of pointer listeners.
/// 2. The held circle is a kinematic proxy in the solver, which pursues the pointer at a capped speed, so a sudden jump
///    reaches the neighbours as a bounded sequence of small moves.
/// 3. Everything the controller is asked to do is queued in a bridge first. Nothing is lost if the controller is busy.
/// 4. A frame is requested only while the physics has something to do. At rest, none is pending.
/// 5. The three buttons are drawn in the scene, so they zoom and pan with it. Reset puts the circles back and gives the
///    springs new random weights and a random tension, so the network settles by itself. Pause pauses and resumes, and
///    Randomise weights changes the weights alone. Reset is refused while a pointer holds a circle or a released one is
///    still catching up.
///
/// A page the browser keeps in its back-forward cache is only suspended: `pagehide` with `persisted` set pauses the
/// physics as for a hidden page, and `pageshow` resumes it from a fresh clock. Any other `pagehide` means the page is
/// gone for good, and the demo is torn down: the bridge shuts down, the pending frame is cancelled, the observers are
/// dropped, the listeners are removed and then the controller goes.
///
/// # Errors
///
/// Returns `Err` if the panel is missing a part, if a library call fails, or if the graph is not one the solver
/// accepts.
pub(crate) fn build_springs_demo() -> Result<(), String> {
    let document = document()?;
    let fixture = Fixture::dijkstra();
    let core = Core::new(fixture.system().map_err(stringify)?);

    // The scene: the circles, the labelled edges, and the zoom toolbar.
    let region = fixture.region;
    let size = Size::new(
        region.origin.x + region.size.width + MARGIN + TOOLBAR_ROOM,
        region.origin.y + region.size.height + BUTTON_ROW_GAP + BUTTON_HEIGHT + MARGIN,
    );
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, size)?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    let mut nodes = Vec::with_capacity(NAMES.len());
    for (index, name) in NAMES.iter().enumerate() {
        let centre = fixture.centres[index];
        nodes.push(
            scene
                .add_circle_node_with(Point::new(centre.x, centre.y), VERTEX_RADIUS, *name, EllipseOptions::default())
                .map_err(stringify)?,
        );
    }
    let connector = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    let mut edges = Vec::with_capacity(fixture.edges.len());
    for &(from, to, weight) in &fixture.edges {
        let edge = scene.add_edge_with(nodes[from], nodes[to], connector).map_err(stringify)?;
        scene
            .set_edge_label(edge, Some(&(weight as i64).to_string()))
            .map_err(stringify)?;
        edges.push(edge);
    }
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    add_buttons(&document, region)?;
    let view = PanelView::find(&document, DIAGRAM_ID)?;

    // The controller, and the bridge that is the only way into it.
    let controller = Rc::new(RefCell::new(Controller::new(core, scene.clone(), nodes.clone(), edges, view)));
    let bridge: Rc<Bridge> = Rc::new(CommandBridge::new(&controller));

    let frame_bridge = Rc::downgrade(&bridge);
    let animation = AnimationLoop::new(move |timestamp_ms| {
        if let Some(bridge) = frame_bridge.upgrade() {
            bridge.dispatch(ControllerCommand::Frame { timestamp_ms });
        }
    })?;
    controller.borrow_mut().set_animation(animation);

    // Every circle is dragged by the library, bounded to the region the physics lives in, and observed. A circle may be
    // dropped on another: the physics keeps them apart, and the library must not push the circle back on its own.
    let drag = DragOptions::default()
        .with_collision(CollisionPolicy::Allow)
        .with_bounds(Some(region));
    let bindings = nodes
        .iter()
        .map(|node| observe(&scene, *node, drag, &bridge))
        .collect::<Result<Vec<_>, _>>()?;

    let listeners = wire_controls(&document, &bridge)?;
    let hidden = document.hidden();
    RUNTIME.with_borrow_mut(|slot| {
        *slot = Some(Runtime {
            bridge: bridge.clone(),
            controller,
            bindings,
            listeners,
        });
    });
    // Publishes the first state, and starts from hidden if the page is.
    bridge.dispatch(ControllerCommand::SetHidden(hidden));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the three buttons in a row under the graph `region`, in the scene's own layer, so they zoom and pan with it.
///
/// # Errors
///
/// Returns `Err` if the scene has no content layer or a button could not be made.
fn add_buttons(document: &web_sys::Document, region: Rect) -> Result<(), String> {
    let layer = svg_buttons::layer(document, DIAGRAM_ID)?;
    let top = region.origin.y + region.size.height + BUTTON_ROW_GAP;
    let mut left = region.origin.x;
    for button in &BUTTONS {
        svg_buttons::add_button(
            document,
            &layer,
            Rect {
                origin: Point::new(left, top),
                size: Size::new(BUTTON_WIDTH, BUTTON_HEIGHT),
            },
            button,
        )?;
        left += BUTTON_WIDTH + BUTTON_GAP;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes `node` draggable with an observer that queues each event in `bridge`, and returns the registration.
///
/// The callback holds the bridge and the lease only weakly, so the scene's listeners never keep either alive, and an
/// event that is still queued when the registration is dropped is recognised as stale.
fn observe(scene: &Scene, node: NodeId, drag: DragOptions, bridge: &Rc<Bridge>) -> Result<ObserverBinding, String> {
    let lease = Rc::new(());
    let weak_lease = Rc::downgrade(&lease);
    let weak_bridge = Rc::downgrade(bridge);
    let handle = scene
        .make_draggable_with_observer(node, drag, move |id, phase| {
            if let Some(bridge) = weak_bridge.upgrade() {
                // Queued, never run here: this may be inside a controller operation that is already in progress.
                bridge.dispatch(ControllerCommand::Drag {
                    id,
                    phase,
                    lease: weak_lease.clone(),
                });
            }
        })
        .map_err(stringify)?;
    Ok(ObserverBinding { handle, lease })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires the three buttons in the scene, the page's visibility, the preference for reduced motion, and the page being
/// left, to `bridge`, and returns the listeners so they can be removed again.
///
/// # Errors
///
/// Returns `Err` if a control is missing or a listener cannot be attached.
fn wire_controls(document: &web_sys::Document, bridge: &Rc<Bridge>) -> Result<Vec<Listener>, String> {
    let mut listeners = Vec::new();
    // The buttons are in the scene, and one pair of listeners on the stage finds them by their action. Those listeners
    // cannot be removed, so they hold the bridge weakly, and after the teardown they reach nothing.
    let weak = Rc::downgrade(bridge);
    svg_buttons::watch(document, STAGE_ID, move |action| {
        let command = match action {
            RESET_ACTION => ControllerCommand::Reset,
            PAUSE_ACTION => ControllerCommand::TogglePaused,
            RANDOMISE_ACTION => ControllerCommand::RandomizeWeights,
            _ => return,
        };
        if let Some(bridge) = weak.upgrade() {
            bridge.dispatch(command);
        }
    })?;

    // A hidden page is paused, and shown again it resumes from a fresh clock.
    let (weak, page) = (Rc::downgrade(bridge), document.clone());
    listeners.push(Listener::new(document, "visibilitychange", move |_| {
        if let Some(bridge) = weak.upgrade() {
            bridge.dispatch(ControllerCommand::SetHidden(page.hidden()));
        }
    })?);

    // Reduced motion settles at once instead of animating.
    if let Some(query) =
        web_sys::window().and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
    {
        if query.matches() {
            bridge.dispatch(ControllerCommand::SetReducedMotion(true));
        }
        let (weak, watched) = (Rc::downgrade(bridge), query.clone());
        listeners.push(Listener::new(&query, "change", move |_| {
            if let Some(bridge) = weak.upgrade() {
                bridge.dispatch(ControllerCommand::SetReducedMotion(watched.matches()));
            }
        })?);
    }

    // A page that is left for good tears the demo down in order. A page the browser keeps in its back-forward cache is
    // restored as it was, with its panel still marked as built, so nothing would ever build it again. It is only
    // suspended: the physics is paused as for a hidden page, and `pageshow` resumes it from a fresh clock.
    if let Some(window) = web_sys::window() {
        let weak = Rc::downgrade(bridge);
        // Not kept in `listeners`: tearing down drops them, and this one is running when it does. It stays registered,
        // and a `pagehide` after the teardown finds nothing to tear down.
        Listener::new(&window, "pagehide", move |event| {
            if is_persisted(event) {
                if let Some(bridge) = weak.upgrade() {
                    bridge.dispatch(ControllerCommand::SetHidden(true));
                }
            } else {
                teardown();
            }
        })?
        .leave_registered();
        let (weak, page) = (Rc::downgrade(bridge), document.clone());
        listeners.push(Listener::new(&window, "pageshow", move |event| {
            if is_persisted(event) {
                if let Some(bridge) = weak.upgrade() {
                    bridge.dispatch(ControllerCommand::SetHidden(page.hidden()));
                }
            }
        })?);
    }
    Ok(listeners)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `event` is a page transition that keeps the page alive: it is going into, or coming out of, the browser's
/// back-forward cache. Any other event, or one that cannot say, counts as a page that is gone for good.
fn is_persisted(event: &web_sys::Event) -> bool {
    event
        .dyn_ref::<web_sys::PageTransitionEvent>()
        .is_some_and(web_sys::PageTransitionEvent::persisted)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Tears the demo down, in this order: the bridge refuses new commands and forgets the queued ones, the pending frame
/// is cancelled, every observer registration and its lease is dropped, the listeners are removed, and last the
/// controller and everything it owns is released. A frame, a drag event or a click that is late after this reaches
/// nothing, and reactivates nothing.
pub(crate) fn teardown() {
    let Some(runtime) = RUNTIME.with_borrow_mut(Option::take) else {
        return;
    };
    let Runtime {
        bridge,
        controller,
        bindings,
        listeners,
    } = runtime;
    bridge.shut_down();
    if let Ok(controller) = controller.try_borrow() {
        controller.cancel_animation();
    }
    drop(bindings);
    drop(listeners);
    drop(controller);
    drop(bridge);
}
