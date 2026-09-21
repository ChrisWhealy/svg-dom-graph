use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Unless [`disarm`](Self::disarm) is called first, this removes `group` and every element tracked via
/// [`track`](Self::track) from the DOM.
///
/// `SvgRoot::rect`/`SvgRoot::text`/`SvgRoot::group` each attach their new element to the document immediately, not just
/// once a caller appends it into its intended parent. So a `?` failing between an element's creation and its
/// `group.append(...)` call would otherwise leave that element behind, as a stray sibling of `group` rather than a
/// child of it. [`track`](Self::track) covers exactly that window.
///
/// A `?` on any fallible step between construction and [`disarm`](Self::disarm) — creating an element, measuring its
/// bounding box, or setting an attribute — drops this guard while still armed. That unwinds a partially built node back
/// to nothing rendered, instead of leaving stray elements in the document.
/// [`SvgNode::remove`](svg_dom::SvgNode::remove) is idempotent, so removing an element already inside `group`'s own
/// (also being removed) subtree is harmless.
///
/// Mirrors `scene::drag`'s own `InstallGuard` rollback pattern, for DOM construction rather than listener installation.
pub(super) struct RenderGuard {
    group: SvgNode,
    loose: Vec<SvgNode>,
    armed: bool,
}

impl RenderGuard {
    /// Pre-sizes `loose` for `capacity` [`track`](Self::track) calls. Pass `0` when the caller has no useful bound
    /// to give; pass a real count whenever the caller already knows, or can cheaply upper-bound, how many elements
    /// it is about to construct — [`draw_content_box`](super::draw_content_box)'s per-cell loop is the motivating
    /// case, where growing `loose` one push at a time would otherwise reallocate repeatedly for a large data node.
    pub(super) fn with_capacity(group: SvgNode, capacity: usize) -> Self {
        Self {
            group,
            loose: Vec::with_capacity(capacity),
            armed: true,
        }
    }

    /// Tracks `node` for rollback. Call this right after creating `node`, before any other fallible step — in
    /// particular, before `group.append(&node)`, which is exactly the gap this guard exists to cover.
    pub(super) fn track(&mut self, node: SvgNode) {
        self.loose.push(node);
    }

    /// Stops tracking the most recently [`track`](Self::track)ed node because it is now safely appended into `group`,
    /// whose own future removal would already cascade to remove it, or because the caller has already removed it
    /// itself.
    ///
    /// Callers that create-then-immediately-resolve one node at a time ([`draw_content_box`](super::draw_content_box)'s
    /// per-cell loop is the motivating case) call this right after each node's own fate is settled, so `loose` never
    /// grows past the small number of nodes momentarily in flight at once, regardless of how many a whole node's own
    /// construction creates in total. This relies on the caller's own strict create-then-resolve discipline: this
    /// always drops whichever node [`track`](Self::track) most recently added, not a specific one named by the caller,
    /// so tracking a second node before resolving the first would silently stop tracking the wrong one.
    pub(super) fn release(&mut self) {
        self.loose.pop();
    }

    /// Rendering finished successfully — do not roll it back on drop.
    pub(super) fn disarm(mut self) {
        self.armed = false;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Drop for RenderGuard {
    fn drop(&mut self) {
        if self.armed {
            self.group.remove();
            for node in &self.loose {
                node.remove();
            }
        }
    }
}
