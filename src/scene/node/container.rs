//! Container nodes — a box, drawn exactly like a plain node's, that owns a nested `Scene` instead of a label's own
//! meaning-nothing-further text. See [`super::super::navigation`] for what "owns" means (strong down, weak up) and
//! for the `enter`/`exit` navigation this makes possible.

use super::{NodeOptions, plain::draw_box, validate_edge_anchors};
use crate::{
    error::Error,
    model::node::{NodeContent, NodeId},
    scene::{
        Scene,
        navigation::{ParentLink, hide_root, is_ancestor_or_self, repoint_subtree, show_root},
    },
};
use std::rc::Rc;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a container node at `top_left`, owning `child` as its nested `Scene`.
    ///
    /// Equivalent to [`add_container_node_with`](Self::add_container_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_container_node_with`](Self::add_container_node_with).
    pub fn add_container_node(
        &self,
        top_left: Point,
        size: Size,
        label: impl Into<String>,
        child: Scene,
    ) -> Result<NodeId, Error> {
        self.add_container_node_with(top_left, size, label, child, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a container node at `top_left`, owning `child` as its nested `Scene`, and returns its id. `options`
    /// controls how many connector fixing points this node's sides offer, exactly as for
    /// [`add_node_with`](Self::add_node_with) — a container node is drawn exactly like a plain node's box, so it
    /// needs exactly a plain node's own construction information, plus the `child` `Scene` it owns.
    ///
    /// `child` is grafted in as-is — this crate never creates a nested `Scene`'s own `<svg>` itself, any more than
    /// [`Scene::new`] creates a top-level one. It must already be fully built: this call hides its whole `<svg>`
    /// root as one of its own steps, since `child` is no longer the focused `Scene` of anything once nested inside
    /// `self` — `self` stays focused, exactly as it was before this call.
    ///
    /// **This hides `child`'s `<svg>` root by writing its `visibility` as a plain SVG attribute, not through
    /// `style` — and [`enter`](Self::enter)/[`exit`](Self::exit) keep toggling it that way for as long as `child`
    /// stays nested. A CSS `visibility` declaration on that same `<svg>` root — an inline `style`, a stylesheet
    /// rule, or anything it inherits the property from — always wins over an attribute, permanently, no matter how
    /// many times `enter`/`exit` run afterward.** A single `your-selector { visibility: visible; }` rule reaching
    /// `child`'s own root is enough to defeat this crate's whole navigation model for it — the child would simply
    /// never actually hide, `enter`/`exit` update `focused` regardless. The host remains free to size, position
    /// (`position`/`top`/`left`, to stop a hidden root reserving page layout it does not need), and otherwise
    /// style that root however it likes; `visibility` alone is this crate's own, for as long as the `Scene` stays
    /// nested.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`, or
    /// [`Error::InvalidNodeGeometry`] under the same conditions [`add_node_with`](Self::add_node_with) already
    /// rejects `top_left`/`size` for. Checked first, so a rejected call leaves both scenes exactly as they were.
    ///
    /// Returns [`Error::AlreadyNested`] if `child` already has a live parent — a nested `Scene` has exactly one
    /// owner at a time. Returns [`Error::ChildNotFocused`] if `child` is not currently the focused `Scene` of its
    /// own tree: grafting only ever happens by an inactive tree's own root, never by one of its hidden descendants,
    /// so the tree's navigation state can never end up disagreeing with what is actually visible. Returns
    /// [`Error::SelfNesting`] if `child` is `self`, or already an ancestor of `self` in the scene tree — either
    /// would close a cycle through the strong `Rc` chain this ownership is built from. All three are checked before
    /// drawing anything or touching either scene's own model.
    ///
    /// Also returns a wrapped [`Error::Svg`] if drawing the container's own box, hiding `child`'s `<svg>` root, or
    /// attaching the drawn box fails. Every fallible step happens before `child` is actually folded into `self`'s
    /// own scene tree — the `Rc`/`Weak` updates that do that cannot themselves fail — so a failed call leaves
    /// `child` exactly as it was in every realistic case: still its own standalone, focused root, not merely "the
    /// parent's `Graph` is unchanged". The one exception: if attaching the drawn box fails after `child`'s own
    /// `<svg>` root was already hidden, this attempts to show it again, and that attempt is not itself guaranteed
    /// to succeed — see [`Scene::enter`]'s own doc comment for the same caveat, which applies here for the same
    /// reason.
    pub fn add_container_node_with(
        &self,
        top_left: Point,
        size: Size,
        label: impl Into<String>,
        child: Scene,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;

        let rect = Rect { origin: top_left, size };
        if !top_left.x.is_finite()
            || !top_left.y.is_finite()
            || !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return Err(Error::InvalidNodeGeometry(rect));
        }

        if is_ancestor_or_self(&child.inner, &self.inner) {
            return Err(Error::SelfNesting);
        }
        let child_has_live_parent = child
            .inner
            .borrow()
            .parent
            .as_ref()
            .is_some_and(|link| link.scene.upgrade().is_some());
        if child_has_live_parent {
            return Err(Error::AlreadyNested);
        }
        if !child.is_focused() {
            return Err(Error::ChildNotFocused);
        }

        let label = label.into();
        let mut inner = self.inner.borrow_mut();
        // Taken out for the call so `draw_box` can format into it without also needing `&inner.svg` to borrow
        // `inner` in two conflicting ways at once — see `SceneInner::scratch`'s own doc comment for why this,
        // rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_box(&inner.svg, &mut scratch, rect, &label, options.edge_anchors);
        inner.scratch = scratch;
        let mut handles = result?;

        // `child` is folding into a bigger tree, so it is no longer the focused Scene of anything: hide its own
        // `<svg>` root now, while a failure still only costs removing the container box just drawn — draw_box's own
        // `RenderGuard` already disarmed on success, so a failure from here on has to be unwound by hand.
        if let Err(err) = hide_root(&child.inner.borrow().svg) {
            handles.group.remove();
            return Err(err);
        }

        if let Err(err) = inner.attach(&handles.group) {
            let _ = show_root(&child.inner.borrow().svg);
            return Err(err);
        }

        handles.child = Some(child.inner.clone());
        let id = inner.graph.add_node(rect, NodeContent::Container(label));
        inner.insert_node_handle(id, handles);

        // Commit: nothing past this point can fail. `child`'s own subtree adopts `self`'s own shared navigation
        // state, and `child` records `self`/`id` as the (parent Scene, container NodeId) pair that now owns it.
        let navigation = inner.navigation.clone();
        drop(inner);
        repoint_subtree(&child.inner, &navigation);
        child.inner.borrow_mut().parent = Some(ParentLink {
            scene: Rc::downgrade(&self.inner),
            node: id,
        });

        Ok(id)
    }
}
