//! Container nodes — a box, drawn exactly like a plain node's, that owns a nested `Scene` instead of a label's own
//! meaning-nothing-further text. See [`super::super::navigation`] for what "owns" means (strong down, weak up) and
//! for the `enter`/`exit` navigation this makes possible.

use super::{NodeOptions, plain::draw_box, validate_edge_anchors};
use crate::{
    error::Error,
    model::node::{NodeContent, NodeId},
    scene::{
        Scene,
        navigation::{ParentLink, detach_subtree, hide_root, is_ancestor_or_self, repoint_subtree, show_root},
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces container node `node`'s own nested child `Scene` with `new_child`, and returns the one it replaced.
    ///
    /// The returned `Scene` is detached, and comes back **visible and focused** — the same state any freshly
    /// constructed, standalone `Scene` starts in, not hidden. This crate's navigation model treats "focused" as
    /// meaning "the scene actually shown and receiving input"; a detached child that stayed focused but hidden
    /// would quietly break that equivalence. Making it visible again is safe precisely because `self` is required
    /// to be focused for this call to succeed at all (see "Errors" below): the old child was therefore already
    /// hidden going in, so revealing it again on the way out can never produce two visible `Scene`s within one
    /// navigation tree — only two separate, independent trees, each with exactly one visible, focused root, which
    /// is the invariant this crate already guarantees everywhere else. Whether the two `<svg>` roots then sit
    /// somewhere sensible on the page is a host layout concern, exactly as it already is for any standalone
    /// `Scene` a host constructs directly.
    ///
    /// The returned `Scene`'s own subtree no longer shares `self`'s own tree-wide navigation state either — it is
    /// given a fresh one of its own, the same two-stage bootstrap [`Scene::new`] uses, so it starts out as the sole
    /// focused root of its own, newly independent tree. Merely clearing its own `parent` link would not be enough
    /// on its own: the whole subtree would still share the *former* parent's navigation state, so a later
    /// `enter`/`exit`/`is_focused` call anywhere in it would still be answered by a navigation state that has
    /// nothing to do with this now-independent tree anymore.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFocused`] if `self` is not the scene tree's currently focused `Scene`. This rules out
    /// replacing a child while looking at one of its own descendants, which would otherwise leave focus with no
    /// defined destination — with this precondition, `self` stays focused throughout the whole call, the one
    /// property every other part of this method leans on. Checked first, so a rejected call touches nothing.
    ///
    /// Returns [`Error::UnknownNode`] if `node` does not name a node in this scene, or
    /// [`Error::NotAContainerNode`] if it names one that is not a container node.
    ///
    /// Returns [`Error::SelfNesting`] if `new_child` is `self`, or already an ancestor of `self` in the scene
    /// tree. Returns [`Error::AlreadyNested`] if `new_child` already has a live parent — which also, and
    /// deliberately, rejects replacing a container node's child with itself: the old child already counts as
    /// `new_child`'s own live parent at this point, since it has not been detached yet. Returns
    /// [`Error::ChildNotFocused`] if `new_child` is not currently the focused `Scene` of its own tree. All three
    /// are exactly the checks [`add_container_node_with`](Self::add_container_node_with) already makes for the
    /// same reasons, applied here to `new_child` in `new_child`'s place.
    ///
    /// Also returns a wrapped [`Error::Svg`] if hiding `new_child`'s own `<svg>` root fails. Every one of the
    /// checks above, and that hide, happens before any ownership or navigation state changes — so a rejected call
    /// leaves the original child attached to `node` exactly as it was, and `new_child` exactly as it was: still an
    /// independent, focused tree of its own. The only DOM write still to come after this point — making the old
    /// child visible again — cannot itself fail this call: by then the replacement has already fully succeeded,
    /// so that write is attempted on a best-effort basis, the same "already happened, so a failure to finish a
    /// secondary step is not reported as this call's own failure" reasoning this crate's own zoom-view bookkeeping
    /// already follows after a successful view change.
    pub fn replace_container_child(&self, node: NodeId, new_child: Scene) -> Result<Scene, Error> {
        if !self.is_focused() {
            return Err(Error::NotFocused);
        }

        let old_child_rc = {
            let inner = self.inner.borrow();
            let handles = inner.node_handle(node).ok_or(Error::UnknownNode(node))?;
            handles.child.clone().ok_or(Error::NotAContainerNode(node))?
        };

        if is_ancestor_or_self(&new_child.inner, &self.inner) {
            return Err(Error::SelfNesting);
        }
        let new_child_has_live_parent = new_child
            .inner
            .borrow()
            .parent
            .as_ref()
            .is_some_and(|link| link.scene.upgrade().is_some());
        if new_child_has_live_parent {
            return Err(Error::AlreadyNested);
        }
        if !new_child.is_focused() {
            return Err(Error::ChildNotFocused);
        }

        // `new_child` is folding into `self`'s own tree, so it is no longer the focused Scene of anything: hide
        // its own `<svg>` root now, while a failure still costs nothing — nothing below this point has touched
        // either scene's own model yet.
        hide_root(&new_child.inner.borrow().svg)?;

        // Commit: nothing past this point can fail. `new_child`'s own subtree adopts `self`'s own shared
        // navigation state and records `self`/`node` as the (parent Scene, container NodeId) pair that now owns
        // it — exactly `add_container_node_with`'s own commit, just onto an already-existing container node
        // instead of a freshly drawn one.
        let navigation = self.inner.borrow().navigation.clone();
        repoint_subtree(&new_child.inner, &navigation);
        new_child.inner.borrow_mut().parent = Some(ParentLink {
            scene: Rc::downgrade(&self.inner),
            node,
        });
        self.inner.borrow_mut().node_handle_mut(node).expect("checked above").child = Some(new_child.inner.clone());

        // The old child is detached: give it a fresh navigation state of its own, and reveal it again — see this
        // method's own doc comment ("Errors") for why that second write's own failure is not reported here.
        detach_subtree(&old_child_rc);
        let _ = show_root(&old_child_rc.borrow().svg);

        Ok(Scene { inner: old_child_rc })
    }
}
