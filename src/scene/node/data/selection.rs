//! Highlighting a data node's cells: `Scene::set_selection` and `Scene::set_secondary_selection`.

use super::style::cell_style;
use crate::{
    error::Error,
    model::{
        content::ResolvedBand,
        node::{NodeContent, NodeId},
    },
    scene::{Scene, Selection},
};

impl Scene {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Highlights node `id`'s own cell(s) per `selection`, and recolours every affected cell immediately.
    ///
    /// `id` must be a [`DataNodeContent`](crate::scene::DataNodeContent) node. It may be drawn via [`add_data_node`](Self::add_data_node)/
    /// [`add_data_node_with`](Self::add_data_node_with), or be an operator node's own single-value result (see
    /// [`add_unary_operator_node`](Self::add_unary_operator_node)/
    /// [`add_binary_operator_node`](Self::add_binary_operator_node)). A plain label node has no cells to highlight.
    ///
    /// This is the only way to change a node's own selection after it is first drawn. A live "previous"/"next" control
    /// stepping through an array as it is processed only ever touches a handful of cells per step, however large the
    /// array. This shape is the hot path being optimised, in both the DOM writes it performs and the Rust computation
    /// that decides them.
    ///
    /// [`Selection::None`] clears back to every cell's own default `NodeValues::type_colour`. So there is no need to
    /// clear before setting a new selection.
    ///
    /// An identical `selection` to `id`'s own current one is an immediate no-op — no cell is touched, and no
    /// `aria-label` write happens. Otherwise, only the cells whose own colour/stroke category (focused, banded, or
    /// default) changes between the old and new selection are examined, let alone written to. Those are the old and new
    /// focus cells, plus each band's own members, via `ResolvedBand::for_each_index`. A cell in neither band, and not a
    /// focus either way, is never visited: its category cannot have changed. A live "previous"/"next" control stepping
    /// through an array as it is processed only ever touches a handful of cells per step, however large the array. This
    /// shape is the hot path being optimised, in both the DOM writes it performs and the Rust computation that decides
    /// them.
    ///
    /// Also gives the focused cell, and, less strongly, a banded row/column, a thicker stroke than its own default
    /// border. It also rebuilds the node's own `aria-label` to describe the current selection as text. Neither depends
    /// on colour alone — the same reasoning `NodeValues::type_colour`'s own `<title>`/`aria-label` pairing already
    /// follows.
    ///
    /// This updates the node's own accessible name, making the current selection available to assistive technology. It
    /// does not create a live-region announcement, so a screen reader whose virtual cursor sits elsewhere may not
    /// notice the change until the user navigates back to this node. An application needing an immediate announcement
    /// should provide its own status/live region; this method does not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`] if `id` names a plain label node. Also returns it if `selection` names a
    /// cell/row/column index out of range for `id`'s own actual value count or grid shape. Checked before recolouring
    /// any cell, so a rejected call leaves every cell's own colour exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if recolouring a cell fails partway through. A failure here can leave some
    /// cells already recoloured and others not — the same documented property
    /// [`set_edge_anchors`](Self::set_edge_anchors) already carries for its own incident redraws.
    pub fn set_selection(&self, id: NodeId, selection: Selection) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_) | NodeContent::Container(_) => return Err(Error::InvalidSelection(id, selection)),
        };
        let (new_band, new_focus) = content
            .resolve_selection(selection)
            .ok_or(Error::InvalidSelection(id, selection))?;
        let base_colour = content.type_colour();

        let old_selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
        if old_selection == selection {
            // No cell needs recolouring. But a selection toolbar just installed against this node has never had its own
            // button states synced at all. Its own commit is exactly a same-as-current `set_selection` call whenever
            // the node's `Selection` already happened to be `Selection::None`. See `sync_selection_toolbar_state`'s own
            // doc comment. Skipping this call here would leave every button with no `aria-disabled`/`opacity` written,
            // not just a stale one.
            let _ = inner.sync_selection_toolbar_state();
            return Ok(());
        }
        // `old_selection` was itself accepted by an earlier, successful `set_selection` call against this same,
        // unchanged content (or is the default `Selection::None`, always valid), so it always resolves here too.
        let (old_band, old_focus) = content.resolve_selection(old_selection).unwrap_or((ResolvedBand::None, None));

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        let cell_stroke_width = handles.cell_stroke_width;
        let cell_rects = &handles.cell_rects;
        let secondary = &handles.secondary;
        let len = cell_rects.len();

        // Every index whose own category (focused, banded, or default) could possibly differ between the old selection
        // and the new one, never the whole grid. Each is visited at most once. A cell outside this set is provably
        // unchanged. It is neither an old/new focus, nor in the symmetric difference of the two bands. So `cell_style`
        // resolves it to the same category either way. See `ResolvedBand::for_each_index`'s own doc comment.
        let mut result = Ok(());
        let mut restyle = |i: usize| {
            if result.is_err() {
                return;
            }
            let Some(cell) = cell_rects.get(i) else { return };
            let is_secondary = secondary.binary_search(&i).is_ok();
            let old_style = cell_style(
                Some(i) == old_focus,
                old_band.contains(i),
                is_secondary,
                base_colour,
                cell_stroke_width,
            );
            let new_style = cell_style(
                Some(i) == new_focus,
                new_band.contains(i),
                is_secondary,
                base_colour,
                cell_stroke_width,
            );
            if new_style == old_style {
                return;
            }
            result = new_style.apply(cell);
        };
        if let Some(i) = old_focus {
            restyle(i);
        }
        // Only if it differs from `old_focus` — otherwise this index was already visited above, and a second visit here
        // would just repeat the same comparison.
        if new_focus != old_focus {
            if let Some(i) = new_focus {
                restyle(i);
            }
        }
        if old_band != new_band {
            // True symmetric difference, not each band walked in full. A member of both bands (their intersection) is
            // skipped in both traversals below, since its own category cannot have changed between them either. A focus
            // index is skipped here too, since it was already visited, explicitly, above.
            let already_visited = |i: usize| new_band.contains(i) || Some(i) == old_focus || Some(i) == new_focus;
            old_band.for_each_index(len, |i| {
                if !already_visited(i) {
                    restyle(i);
                }
            });
            let already_visited = |i: usize| old_band.contains(i) || Some(i) == old_focus || Some(i) == new_focus;
            new_band.for_each_index(len, |i| {
                if !already_visited(i) {
                    restyle(i);
                }
            });
        }
        result?;

        handles.selection = selection;
        handles.refresh_label()?;

        // The selection has already changed, so a failure to keep a selection toolbar's own button states in sync with
        // it is not reported as this call's own failure. This follows the same reasoning as `SceneInner::flush_view`'s
        // own `let _ = self.sync_toolbar_state();`. The next selection change puts it right.
        let _ = inner.sync_selection_toolbar_state();

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks the cells at flat indices `cells` of node `id` as *secondary*. They are derived from the current
    /// selection, and part of the same step, without being the one cell (or row/column)
    /// [`set_selection`](Self::set_selection) names.
    ///
    /// A walk often reads more than the cell it is standing on. SHA3's `Chi` step, standing on `A[x, y]`, also reads
    /// `A[x + 1, y]` and `A[x + 2, y]`; `Pi` writes that same step's value to a cell elsewhere. `cells` names those
    /// derived cells, on this node or on any other data node, so a reader can see everything one step touches.
    ///
    /// # Independent of `Selection`
    ///
    /// This never reads or changes `id`'s own [`Selection`], and [`set_selection`](Self::set_selection) never changes
    /// the secondary cells. A selection toolbar's own stepping is therefore unaffected: it only ever reads the primary
    /// `Selection`. The two are set separately — typically both on every step.
    ///
    /// # Rendering
    ///
    /// A secondary cell has its own teal fill and a dashed outline. Both, since colour alone is not a reliable channel
    /// — the same reasoning [`set_selection`](Self::set_selection) follows for its own thicker borders. Where a cell is
    /// also primary-selected, the primary highlight wins. Where it is also inside a selected row or column, the
    /// secondary one wins.
    ///
    /// The node's own `aria-label` and tooltip gain `", also highlighted: cells 3, 4"`.
    ///
    /// # Replacement
    ///
    /// `cells` replaces whatever was secondary before. An empty slice clears it. Order and duplicates do not matter.
    /// There is no limit on how many cells can be secondary beyond the node's own value count. An identical set to the
    /// current one is an immediate no-op, and otherwise only the cells that enter or leave the set are recoloured.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`], carrying the offending index as a [`Selection::Cell`], if `id` names a
    /// plain label or container node. It also returns it if any index in `cells` is out of range for `id`'s own value
    /// count. Checked before recolouring any cell, so a rejected call leaves every cell exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if recolouring a cell fails partway through, with the same "can leave some
    /// cells already recoloured" property [`set_selection`](Self::set_selection) documents.
    pub fn set_secondary_selection(&self, id: NodeId, cells: &[usize]) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_) | NodeContent::Container(_) => {
                return Err(Error::InvalidSelection(id, Selection::None));
            },
        };
        let len = content.len();
        if let Some(&bad) = cells.iter().find(|&&i| i >= len) {
            return Err(Error::InvalidSelection(id, Selection::Cell(bad)));
        }
        let base_colour = content.type_colour();
        let (band, focus) = {
            let selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
            // `selection` was accepted by an earlier call against this same content, so it always resolves.
            content.resolve_selection(selection).unwrap_or((ResolvedBand::None, None))
        };

        let mut new_secondary = cells.to_vec();
        new_secondary.sort_unstable();
        new_secondary.dedup();

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.secondary == new_secondary {
            return Ok(());
        }
        let cell_stroke_width = handles.cell_stroke_width;

        // Only a cell that enters or leaves the set can change style, so walk the two sorted lists' own symmetric
        // difference rather than every cell in the grid.
        let old = &handles.secondary;
        let mut changed = Vec::with_capacity(old.len() + new_secondary.len());
        changed.extend(old.iter().filter(|i| new_secondary.binary_search(i).is_err()));
        changed.extend(new_secondary.iter().filter(|i| old.binary_search(i).is_err()));

        let mut result = Ok(());
        for i in changed {
            let Some(cell) = handles.cell_rects.get(i) else { continue };
            let style_with = |secondary: bool| {
                cell_style(Some(i) == focus, band.contains(i), secondary, base_colour, cell_stroke_width)
            };
            let new_style = style_with(new_secondary.binary_search(&i).is_ok());
            if new_style != style_with(old.binary_search(&i).is_ok()) {
                result = new_style.apply(cell);
                if result.is_err() {
                    break;
                }
            }
        }
        result?;

        handles.secondary = new_secondary;
        handles.refresh_label()?;
        Ok(())
    }
}
