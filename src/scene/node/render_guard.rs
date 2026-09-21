use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most simultaneous loose nodes any real caller in this crate can ever have (where "loose" means tracked but not
/// yet resolved — see [`RenderGuard::track`]/[`RenderGuard::release`]).
///
/// [`draw_operator_box`](super::draw_operator_box) is the largest: its own label, value, outer, and value-row elements
/// are all tracked before any of them is released.
///
/// Every other caller in this module tracks at most two at once.
const MAX_LOOSE: usize = 4;

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
/// `loose` is fixed-size, not a `Vec`: every real caller tracks at most [`MAX_LOOSE`] nodes at once (see that
/// constant's own doc comment), so a heap-allocated, growable buffer would only ever hold a handful of elements at
/// most, for the cost of one allocation per node this crate ever constructs — successful or rolled back alike.
///
/// Mirrors `scene::drag`'s own `InstallGuard` rollback pattern, for DOM construction rather than listener installation.
pub(super) struct RenderGuard {
    group: SvgNode,
    loose: [Option<SvgNode>; MAX_LOOSE],
    len: usize,
    armed: bool,
}

impl RenderGuard {
    pub(super) fn new(group: SvgNode) -> Self {
        Self {
            group,
            loose: [None, None, None, None],
            len: 0,
            armed: true,
        }
    }

    /// Tracks `node` for rollback. Call this right after creating `node`, before any other fallible step — in
    /// particular, before `group.append(&node)`, which is exactly the gap this guard exists to cover.
    ///
    /// # Panics
    ///
    /// Panics if more than [`MAX_LOOSE`] nodes are tracked at once, without an intervening [`release`](Self::release)
    /// — every real caller in this crate stays within that bound (see its own doc comment), so this can only fire from
    /// a bug in this module itself, not from anything external.
    pub(super) fn track(&mut self, node: SvgNode) {
        let slot = self
            .loose
            .get_mut(self.len)
            .unwrap_or_else(|| panic!("RenderGuard::track: cannot track more than {MAX_LOOSE} nodes at once"));
        *slot = Some(node);
        self.len += 1;
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
        if let Some(i) = self.len.checked_sub(1) {
            self.loose[i] = None;
            self.len = i;
        }
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
            for node in self.loose[..self.len].iter().flatten() {
                node.remove();
            }
        }
    }
}
