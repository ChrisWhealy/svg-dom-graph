use crate::{
    model::{edge::EdgeId, node::NodeId},
    scene::Scene,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rolls back an operator node's own compound "create the node, then wire its auto-connected input edge(s)"
/// operation. This happens if a later step fails after the node itself was already drawn and registered.
///
/// `draw_operator_box`'s own [`RenderGuard`](super::render_guard::RenderGuard) already makes the node's *own* DOM
/// construction atomic. But `add_unary_operator_node_with`/`add_two_input_operator_node_with` don't stop there —
/// once the node is registered in the graph and `node_handles`, one or two further `Scene::add_edge` calls wire its
/// input(s). A failure in any of those would otherwise leave the node, and any edge that did succeed, behind. That
/// is exactly the "operator plus one input connection" state this guard exists to prevent.
///
/// Create one right after the node itself is registered. [`track_edge`](Self::track_edge) each `Scene::add_edge`
/// call's own id as it succeeds, and [`disarm`](Self::disarm) once every edge has been wired.
///
/// Dropped while still armed, this removes every tracked edge first — its rendered path, its `edge_handles` entry,
/// and its place in the graph. It then removes the node itself, so nothing is ever asked to remove a node an edge
/// still points at.
///
/// Mirrors [`RenderGuard`](super::render_guard::RenderGuard)'s own rollback pattern, but one level up. `RenderGuard`
/// only ever undoes DOM construction, since the node isn't registered anywhere yet by the time it runs.
///
/// By the time this guard exists, the node already is registered. So its own rollback also has to unwind the
/// graph model and the parallel `node_handles`/`edge_handles` bookkeeping `SceneInner` keeps beside it.
pub(super) struct OperatorConstructionGuard {
    scene: Scene,
    node_id: NodeId,
    edge_ids: [Option<EdgeId>; 2],
    armed: bool,
}

impl OperatorConstructionGuard {
    pub(super) fn new(scene: Scene, node_id: NodeId) -> Self {
        Self {
            scene,
            node_id,
            edge_ids: [None, None],
            armed: true,
        }
    }

    /// Tracks edge `id` for rollback. Call this right after each `Scene::add_edge`/`add_edge_with` call succeeds.
    ///
    /// A unary operator tracks one edge, a binary operator two — never more. `edge_ids` is fixed-size for exactly
    /// that reason, so a third call is a caller bug rather than something to accommodate.
    pub(super) fn track_edge(&mut self, id: EdgeId) {
        let slot = self
            .edge_ids
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("OperatorConstructionGuard::track_edge: no more than two edges are ever tracked");
        *slot = Some(id);
    }

    /// Every edge wired successfully — do not roll anything back on drop.
    pub(super) fn disarm(mut self) {
        self.armed = false;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Drop for OperatorConstructionGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let mut inner = self.scene.inner.borrow_mut();
        // Reverse creation order: `Graph::remove_edge`/`SceneInner::remove_edge_handle` only ever remove the most
        // recently added edge (see their own doc comments), so the second-tracked edge, if any, must go first.
        for edge_id in self.edge_ids.into_iter().rev().flatten() {
            if let Some(handle) = inner.remove_edge_handle(edge_id) {
                handle.path.remove();
            }
            inner.graph.remove_edge(edge_id);
        }
        if let Some(handles) = inner.remove_node_handle(self.node_id) {
            handles.group.remove();
        }
        inner.graph.remove_node(self.node_id);
    }
}
