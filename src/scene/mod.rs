//! Renders a graph onto the DOM, and keeps each node's and edge's rendered SVG handles alongside it.
//!
//! The topology model (crate-private while this crate's API is still taking shape) owns the topology and is the single
//! source of truth for it. This module pairs each of its ids with a rendered handle, and keeps both in sync as
//! nodes move.
//!
//! This crate has no opinion about which HTML page hosts a [`Scene`], or what graph a caller builds with one. See the
//! sibling `demo-app` crate for a small worked example.

mod box_handles;
mod connector;
pub(crate) mod drag;
mod frame_request;
pub(crate) mod node;
mod scene_inner;
pub mod toolbar;
mod view_input;

pub use crate::geometry::side::Side;
pub use crate::model::content::{
    ArithmeticOperator, BinaryOperator, ByteOrder, DataFormat, DataNodeContent, GridLayout, NodeValues, Selection,
    UnaryOperator,
};
pub(crate) use box_handles::BoxHandles;
pub use connector::{ConnectorOptions, ConnectorType};
pub use drag::{DragOptions, collision_policy::CollisionPolicy};
pub use node::{EdgeAnchors, NodeOptions};
pub use toolbar::ToolbarOptions;
pub use view_input::InputMode;

use crate::{
    error::Error,
    geometry::{apply_matrix, view::ViewTransform},
    model::graph::Graph,
};
use scene_inner::SceneInner;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use svg_dom::{
    MarkerUnits, SvgMarker, SvgRoot,
    root::utils::{Matrix2D, Point, Rect},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts `client` (viewport CSS pixels, such as `PointerEvent::client_x`/`client_y`) into user-space coordinates,
/// via `inverse_ctm`.
fn client_to_user_space(client: Point, inverse_ctm: Matrix2D) -> Point {
    apply_matrix(inverse_ctm, client)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of a box's rectangle.
fn box_centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Assigns each `Scene` a distinct number, so its arrow marker gets an id no other `Scene` — and, so long as a caller's
/// own document doesn't deliberately collide with this crate's naming, no unrelated content either — is likely
/// to claim.
static NEXT_SCENE_ID: AtomicUsize = AtomicUsize::new(0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The squared distance between `a` and `b`.
///
/// The distance squared, not the actual distance. Every caller only compares the square of the distance, so all
/// comparisons can still function but without the expensive square root operation.
fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines a small filled-triangle arrowhead marker in `<defs>` and returns its handle.
///
/// `ref_x`/`ref_y` place the marker's anchor point (the tip of the triangle) at the very end of the line it attaches
/// to. `orient("auto")` then rotates the marker to follow that line's own direction.
///
/// `marker_id` must be unique within `svg`'s document. A hardcoded id such as `"arrow"` would collide the moment a
/// second `Scene` shares the same `<svg>`, or the caller's own document already defines an element with that id.
fn define_arrow_marker(svg: &SvgRoot, marker_id: &str) -> Result<SvgMarker, Error> {
    let defs = svg.defs()?;
    let marker = defs.marker(marker_id)?;

    marker.set_units(MarkerUnits::UserSpaceOnUse)?;
    marker.set_marker_width(10.0)?;
    marker.set_marker_height(7.0)?;
    marker.set_ref_x(9.0)?;
    marker.set_ref_y(3.5)?;
    marker.set_orient("auto")?;
    marker.polygon(&[Point::new(0.0, 0.0), Point::new(10.0, 3.5), Point::new(0.0, 7.0)])?;

    Ok(marker)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A cheap, cloneable handle to a rendered graph.
///
/// Internally an `Rc<RefCell<SceneInner>>` — this crate owns that sharing strategy, not the caller. A `Scene` can be
/// cloned freely (every clone refers to the same underlying graph and DOM state) and its methods take `&self`, not
/// `&mut self`, so a caller never has to wrap it in `Rc<RefCell<_>>` themselves just to call
/// [`make_draggable`](Self::make_draggable) or to share it with more than one closure.
///
/// # Keep at least one handle alive for as long as the scene should stay interactive
///
/// [`make_draggable`](Self::make_draggable)'s own listener closures deliberately hold only `Weak` references back to
/// this scene's shared state, not strong ones — a strong self-reference there would leak the whole scene (and every
/// node, edge, and DOM element it owns) forever, since nothing would ever be able to drop the last strong handle.
///
/// The consequence: once every `Scene` handle a caller holds is dropped, the scene's shared state is freed immediately,
/// and every listener silently stops responding — no panic, nothing in the console. This is easy to trip over in
/// exactly the shape a `#[wasm_bindgen(start)]` entry point naturally takes:
///
/// ```rust,no_run
/// # use svg_dom::{SvgRoot, root::utils::{Point, Size}};
/// # use svg_dom_graph::{Error, scene::Scene};
/// fn build() -> Result<(), Error> {
///     let svg = SvgRoot::attach("diagram")?;
///     let scene = Scene::new(svg)?;
///     let node = scene.add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Node")?;
///     scene.make_draggable(node)?;
///     Ok(())
///     // `scene` drops here, at the end of this function — which for a `#[wasm_bindgen(start)]` entry point
///     // happens at page load, long before the user ever gets a chance to click anything. Dragging silently
///     // does nothing.
/// }
/// ```
///
/// Keep a handle alive somewhere that outlives the function that built it — for example, in a `thread_local!` for the
/// page's whole lifetime, as `demo-app`'s own `SCENE` does.
#[derive(Clone)]
pub struct Scene {
    inner: Rc<RefCell<SceneInner>>,
}

impl Scene {
    /// Creates an empty scene, ready to hold nodes and edges within `svg`.
    ///
    /// Also defines the arrow marker every edge's connector uses, since every `Scene` needs exactly one, shared across
    /// all its edges.
    pub fn new(svg: SvgRoot) -> Result<Self, Error> {
        let marker_id = format!("svg-dom-graph-arrow-{}", NEXT_SCENE_ID.fetch_add(1, Ordering::Relaxed));
        let arrow = define_arrow_marker(&svg, &marker_id)?;
        let content = svg.group()?;
        content.set_attr("class", "svg-dom-graph-content")?;
        Ok(Self {
            inner: Rc::new(RefCell::new(SceneInner {
                svg,
                content,
                view: ViewTransform::default(),
                view_dirty: false,
                toolbar: None,
                pan_mode: InputMode::default(),
                wheel_zoom_mode: InputMode::default(),
                view_input: None,
                graph: Graph::new(),
                node_handles: Vec::new(),
                edge_handles: Vec::new(),
                arrow,
                scratch: String::new(),
            })),
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
