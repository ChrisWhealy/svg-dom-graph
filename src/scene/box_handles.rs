use crate::scene::node::EdgeAnchors;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered elements that make up one box, kept so a drag handler can reposition them.
///
/// Every one of a box's own children — its outer rect, its label or grid cells — is drawn once, at creation, in
/// local coordinates relative to `(0, 0)`. `group`'s own `transform="translate(...)"` is the only thing that ever
/// changes afterward. [`SceneInner::move_node`] repositions a box by rewriting this one transform. This cost
/// stays the same regardless of how many children `group` holds. So a data node with hundreds of value cells
/// moves exactly as cheaply as a plain label. `BoxHandles` itself needs no handle to any individual child;
/// `group` is enough.
pub struct BoxHandles {
    /// Event listeners attach here, so a click on any child starts a drag.
    pub group: SvgNode,
    /// Whether `Scene::make_draggable`/`Scene::make_draggable_with` has already been called for this node.
    ///
    /// `svg-dom`'s listener registration is append-only, so a second call would add a second, independent set of
    /// pointer listeners rather than replacing the first — see [`crate::Error::AlreadyDraggable`].
    pub draggable: bool,
    /// How many evenly spaced connector fixing points this node's own sides offer — see [`EdgeAnchors`].
    ///
    /// `redraw_edge` has no other way to learn a node's own anchor configuration once an incident edge needs a reroute.
    /// This value must live alongside the rendered handle, not just get used once at creation.
    pub edge_anchors: Option<EdgeAnchors>,
}
