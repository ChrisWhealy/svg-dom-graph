//! Scene-tree navigation: which nested `Scene` is currently focused, and how a container node's own child gets
//! grafted into — or falls out of — its parent's tree.
//!
//! "Focused" is not inferred from DOM visibility; it is tracked explicitly, in one [`NavigationState`] shared by
//! every `Scene` in a tree via `SceneInner::navigation`. Without that, a caller holding an outer `Scene` handle
//! after navigating deeper could call [`Scene::enter`]/[`Scene::exit`] on it anyway, and nothing would stop two
//! Scenes ending up visible at once. [`Scene::enter`]/[`Scene::exit`] both check that `self` is the tree's
//! currently focused `Scene` before doing anything else, and update this same shared state once the visibility
//! swap that follows has actually succeeded.
//!
//! [`hide_root`]/[`show_root`] hide the whole `<svg>` root, not just its content layer — the toolbar bar is a DOM
//! sibling of the content layer (see [`super::toolbar`]'s own module doc comment), so hiding only the content layer
//! would leave a hidden Scene's own toolbar still visible. They use the `visibility` CSS property rather than
//! `display: none`, so a Scene built while hidden still measures correctly: `shrink_label_to_fit`'s `getBBox()` and
//! `SceneInner::visible_area`'s `getBoundingClientRect()` fallback both read zero for anything built inside a
//! `display: none` subtree in most browsers, but not a `visibility: hidden` one.

use super::{Scene, SceneInner};
use crate::{error::Error, model::node::NodeId};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use svg_dom::SvgRoot;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The (parent `Scene`, container `NodeId`) pair that owns a nested `Scene` as its child. `SceneInner::parent`'s
/// own value once a `Scene` has been grafted in by [`Scene::add_container_node`]/`add_container_node_with`.
///
/// `scene` is `Weak`, mirroring every other back-reference in this crate to a `Scene` one does not own — see
/// `toolbar::build_button`'s own doc comment. A strong reference here, alongside the strong `BoxHandles::child` the
/// parent already holds, would leak the whole subtree: neither side could ever be the one to drop last.
///
/// Once `scene` no longer upgrades (the parent's last strong handle is gone), this child is simply detached: not an
/// error, not a state anything here has to clean up. [`Scene::parent`]/[`Scene::is_nested`] read straight through
/// to a dead `Weak` and report `None`/`false` accordingly; [`Scene::exit`] treats it exactly like a true root.
#[derive(Clone)]
pub(super) struct ParentLink {
    pub(super) scene: Weak<RefCell<SceneInner>>,
    pub(super) node: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shared by every `Scene` in one scene tree: which one of them is currently focused — the one actually shown and
/// receiving input, exactly one at a time.
///
/// Created once, when a `Scene` that starts a new tree is made ([`Scene::new`](super::Scene::new)), and adopted by
/// every `Scene` [`Scene::add_container_node`]/`add_container_node_with` ever grafts underneath it — see
/// [`repoint_subtree`]'s own doc comment.
pub(super) struct NavigationState {
    pub(super) focused: Weak<RefCell<SceneInner>>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets `svg`'s own root `<svg>` element's `visibility`, hidden or visible.
///
/// The whole root, deliberately — not [`SceneInner::content`](super::SceneInner), whose own layer the toolbar bar
/// is a sibling of, never a child of (see this module's own doc comment).
///
/// Writes `visibility` as a plain SVG presentation attribute (`setAttribute`), not through `style`. That is
/// deliberate, but it has a consequence a host must respect: a presentation attribute is the *lowest*-priority
/// source of a CSS property there is. Any `visibility` declaration that also applies to this same `<svg>` root —
/// an inline `style` attribute, or a stylesheet rule, whether written directly on it or inherited — wins over
/// whatever this function writes, permanently, regardless of how many times [`enter`](Scene::enter)/
/// [`exit`](Scene::exit) run afterward. **A host embedding a nested `Scene`'s own `<svg>` must not set
/// `visibility` on it, or on anything it inherits that property from, by any CSS means at all** — sizing,
/// positioning (including `position`/`top`/`left` to keep a hidden root from reserving page layout it doesn't
/// need), and every other property remain entirely the host's own choice; only `visibility` itself is reserved.
fn set_root_visibility(svg: &SvgRoot, hidden: bool) -> Result<(), Error> {
    svg.root
        .set_attribute("visibility", if hidden { "hidden" } else { "visible" })
        .map_err(|err| Error::Svg(svg_dom::Error::Dom(format!("{err:?}"))))
}

/// Hides `svg`'s own root `<svg>` element. See [`set_root_visibility`].
pub(super) fn hide_root(svg: &SvgRoot) -> Result<(), Error> {
    set_root_visibility(svg, true)
}

/// Shows `svg`'s own root `<svg>` element. See [`set_root_visibility`].
pub(super) fn show_root(svg: &SvgRoot) -> Result<(), Error> {
    set_root_visibility(svg, false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `candidate` is `of` itself, or already an ancestor of it — walking `of`'s own `parent` chain through
/// live [`ParentLink`]s only. A dead link (an ancestor whose own last strong handle has been dropped) ends the
/// walk there: nothing beyond it is still part of the tree `of` belongs to.
///
/// [`Scene::add_container_node`]/`add_container_node_with` call this with `candidate` set to the offered child and
/// `of` set to `self`, to reject grafting a node onto its own descendant. Allowing that would close a cycle through
/// the strong `BoxHandles::child`/`Rc` chain each successful `add_container_node` call adds one more link to —
/// exactly the leak the strong-down/weak-up ownership split exists to prevent.
pub(super) fn is_ancestor_or_self(candidate: &Rc<RefCell<SceneInner>>, of: &Rc<RefCell<SceneInner>>) -> bool {
    if Rc::ptr_eq(candidate, of) {
        return true;
    }
    let mut current = of.borrow().parent.clone();
    while let Some(link) = current {
        let Some(ancestor) = link.scene.upgrade() else { break };
        if Rc::ptr_eq(&ancestor, candidate) {
            return true;
        }
        current = ancestor.borrow().parent.clone();
    }
    false
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Repoints every `SceneInner` in `root`'s own subtree — `root` itself, then recursively through every container
/// node's own `BoxHandles::child` — to share `navigation` instead of whatever it held before.
///
/// [`Scene::add_container_node`]/`add_container_node_with` call this once, last, after every fallible step has
/// already succeeded — see their own doc comments for why the ordering matters.
///
/// Safe to call on a tree of any depth without ever holding two overlapping borrows of the same `SceneInner`: each
/// level's own container children are cloned out of one immutable borrow, which then ends, before this recurses
/// into any of them. Borrowing a parent and, while still borrowed, reaching through it to mutate a child would
/// compile — `RefCell`'s borrow checking is a runtime property, not one `rustc` enforces — and then panic the
/// first time a real tree nested more than one level deep.
pub(super) fn repoint_subtree(root: &Rc<RefCell<SceneInner>>, navigation: &Rc<RefCell<NavigationState>>) {
    let children: Vec<Rc<RefCell<SceneInner>>> = root
        .borrow()
        .node_handles
        .iter()
        .filter_map(|handles| handles.child.clone())
        .collect();
    root.borrow_mut().navigation = navigation.clone();
    for child in &children {
        repoint_subtree(child, navigation);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Enters `node`'s own nested child `Scene`: hides `self`'s whole `<svg>` root, shows the child's, and returns
    /// a handle to it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFocused`] if `self` is not the scene tree's currently focused `Scene`. Checked first, so
    /// a rejected call touches nothing.
    ///
    /// Returns [`Error::UnknownNode`] if `node` does not name a node in this scene, or [`Error::NotAContainerNode`]
    /// if it names one that is not a container node. Also checked before anything is hidden or shown.
    ///
    /// Returns a wrapped [`Error::Svg`] if hiding `self` or showing the child fails. This is transactional: if
    /// showing the child fails after `self` was already hidden, `self` is shown again before returning, so a
    /// failed `enter` never leaves two Scenes hidden, or the wrong one visible. The tree's focused `Scene` is only
    /// ever updated once both DOM writes have already succeeded.
    pub fn enter(&self, node: NodeId) -> Result<Scene, Error> {
        if !self.is_focused() {
            return Err(Error::NotFocused);
        }

        let child_rc = {
            let inner = self.inner.borrow();
            let handles = inner.node_handle(node).ok_or(Error::UnknownNode(node))?;
            handles.child.clone().ok_or(Error::NotAContainerNode(node))?
        };

        hide_root(&self.inner.borrow().svg)?;
        if let Err(err) = show_root(&child_rc.borrow().svg) {
            // `self` must end this call exactly as visible as it started, whichever branch is taken.
            let _ = show_root(&self.inner.borrow().svg);
            return Err(err);
        }

        let navigation = self.inner.borrow().navigation.clone();
        navigation.borrow_mut().focused = Rc::downgrade(&child_rc);
        Ok(Scene { inner: child_rc })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Exits to the parent `Scene`, if any: hides `self`'s whole `<svg>` root and shows the parent's.
    ///
    /// Does nothing — returns `Ok(None)` — if `self` is the root of its own tree, or has become detached from one
    /// (its own parent's last strong handle has been dropped). Both read the same way here: there is no live parent
    /// to exit to.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFocused`] if `self` is not the scene tree's currently focused `Scene`. Checked first, so
    /// a rejected call touches nothing.
    ///
    /// Returns a wrapped [`Error::Svg`] under the same transactional guarantee [`enter`](Self::enter) documents.
    pub fn exit(&self) -> Result<Option<Scene>, Error> {
        if !self.is_focused() {
            return Err(Error::NotFocused);
        }

        let Some(parent_rc) = self.inner.borrow().parent.as_ref().and_then(|link| link.scene.upgrade()) else {
            return Ok(None);
        };

        hide_root(&self.inner.borrow().svg)?;
        if let Err(err) = show_root(&parent_rc.borrow().svg) {
            let _ = show_root(&self.inner.borrow().svg);
            return Err(err);
        }

        let navigation = self.inner.borrow().navigation.clone();
        navigation.borrow_mut().focused = Rc::downgrade(&parent_rc);
        Ok(Some(Scene { inner: parent_rc }))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `(Scene, NodeId)` that owns this `Scene` as a container node's child, if any.
    ///
    /// `None` for a true root, and also for a `Scene` that has become detached from its own tree — its former
    /// parent's last strong handle has been dropped. Both read the same way: there is no live parent to report.
    pub fn parent(&self) -> Option<(Scene, NodeId)> {
        let link = self.inner.borrow().parent.clone()?;
        let scene_rc = link.scene.upgrade()?;
        Some((Scene { inner: scene_rc }, link.node))
    }

    /// Whether this `Scene` is any container node's child. Derived from [`parent`](Self::parent), rather than
    /// tracked separately, so it can never disagree with it: `false` for a true root, and also once a nested
    /// `Scene`'s own former parent has been dropped.
    pub fn is_nested(&self) -> bool {
        self.parent().is_some()
    }

    /// Whether this `Scene` is the scene tree's currently focused one — the one actually shown and receiving
    /// input.
    pub fn is_focused(&self) -> bool {
        let inner = self.inner.borrow();
        let navigation = inner.navigation.borrow();
        navigation
            .focused
            .upgrade()
            .is_some_and(|focused| Rc::ptr_eq(&focused, &self.inner))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes container node `id` itself clickable: a pointer click, or Enter/Space while it has keyboard focus,
    /// calls [`enter(id)`](Self::enter) — the caller never has to wire that up by hand.
    ///
    /// Deliberately separate from [`add_container_node`](Self::add_container_node)/
    /// [`add_container_node_with`](Self::add_container_node_with): a container node stays exactly as passive as any
    /// other node, with no built-in way to activate it, unless a caller asks for this. `enter`/`exit` are
    /// navigation primitives, not input handlers — see [`Error::NotFocused`]'s own doc comment — and this is the
    /// one place that deliberately bridges the two, opt in.
    ///
    /// Gives `id`'s own `<g>` `role="button"`, `tabindex="0"`, and a pointer cursor — the same activation pattern
    /// `toolbar::build_button` already uses for its own buttons. An activation attempt that fails
    /// — `self` is not focused, or `id`'s own child has since become otherwise unenterable — is silently ignored:
    /// there is nowhere for a click listener to report an error to, the same reasoning that button's own click
    /// handler already follows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene, or
    /// [`Error::NotAContainerNode`] if it names one that is not a container node. Checked before attaching
    /// anything.
    ///
    /// Returns [`Error::AlreadyEnterable`] if `id` is already enterable — calling this a second time for the same
    /// node does not replace the first installation, so this is rejected outright rather than silently doubling up
    /// its listeners, the same reasoning [`Error::AlreadyDraggable`] already documents for
    /// [`make_draggable`](Self::make_draggable)/[`make_draggable_with`](Self::make_draggable_with).
    ///
    /// If `set_attr` or either listener registration this method makes fails partway through — expected to be
    /// extremely rare, since it means the underlying `addEventListener`/`setAttribute` DOM call itself failed —
    /// `id` is left exactly as it was before the call: not marked enterable, and with none of this method's own
    /// listeners left attached.
    pub fn make_enterable(&self, id: NodeId) -> Result<(), Error> {
        let group = {
            let inner = self.inner.borrow();
            let handles = inner.node_handle(id).ok_or(Error::UnknownNode(id))?;
            if handles.child.is_none() {
                return Err(Error::NotAContainerNode(id));
            }
            if handles.enterable {
                return Err(Error::AlreadyEnterable(id));
            }
            handles.group.clone()
        };

        let result: Result<(), svg_dom::Error> = (|| {
            group.set_attr("role", "button")?;
            group.set_attr("tabindex", "0")?;
            group.set_attr("style", "cursor: pointer;")?;

            let weak = Rc::downgrade(&self.inner);
            group.on_click(move |_| {
                if let Some(inner) = weak.upgrade() {
                    let _ = (Scene { inner }).enter(id);
                }
            })?;

            let weak = Rc::downgrade(&self.inner);
            group.on_keydown(move |event| {
                if event.key() == "Enter" || event.key() == " " {
                    event.prevent_default();
                    if let Some(inner) = weak.upgrade() {
                        let _ = (Scene { inner }).enter(id);
                    }
                }
            })
        })();
        if let Err(err) = result {
            group.remove_listeners("click");
            group.remove_listeners("keydown");
            return Err(err.into());
        }

        // Cannot fail: `id` already resolved to a real node handle above, in this same `self.inner`.
        self.inner.borrow_mut().node_handle_mut(id).expect("checked above").enterable = true;
        Ok(())
    }
}
