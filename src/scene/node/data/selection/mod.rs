//! Highlighting a data node's cells: `Scene::set_selection` and `Scene::set_secondary_selection`.

use super::style::{CellStyle, cell_style};
use crate::{
    error::Error,
    model::{
        content::ResolvedBand,
        node::{NodeContent, NodeId},
    },
    scene::{DataNodeContent, Scene, Selection},
};

impl Scene {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Highlights node `id`'s own cell(s) per `selection`, and recolours every affected cell immediately.
    ///
    /// `id` must be a [`DataNodeContent`] node. It may be drawn via [`add_data_node`](Self::add_data_node)/
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
        let new = Highlight::resolve(content, selection).ok_or(Error::InvalidSelection(id, selection))?;
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
        let old = Highlight::resolve(content, old_selection).unwrap_or_default();

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        let cell_stroke_width = handles.cell_stroke_width;
        let (cell_rects, cell_texts) = (&handles.cell_rects, &handles.cell_texts);
        let (secondary, unreached) = (&handles.secondary, &handles.unreached);
        let single_value = handles.cell_grid.is_some_and(|grid| grid.single_value);

        let mut result = Ok(());
        for_each_changed_cell(&old, &new, cell_rects.len(), |i| {
            if result.is_err() {
                return;
            }
            let Some(cell) = cell_rects.get(i) else { return };
            let is_secondary = secondary.binary_search(&i).is_ok();
            let is_unreached = unreached.binary_search(&i).is_ok();
            let new_style = new.style(i, is_secondary, is_unreached, base_colour, cell_stroke_width);
            let old_style = old.style(i, is_secondary, is_unreached, base_colour, cell_stroke_width);
            if new_style == old_style {
                return;
            }
            result = new_style.apply_from(old_style, single_value, cell, cell_texts.get(i));
        });
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
        let highlight = {
            let selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
            // `selection` was accepted by an earlier call against this same content, so it always resolves.
            Highlight::resolve(content, selection).unwrap_or_default()
        };

        // The same indices again, already sorted and without duplicates, change nothing. That is checked before the
        // copy below, which still normalises anything else.
        if inner.node_handle(id).ok_or(Error::UnknownNode(id))?.secondary == cells {
            return Ok(());
        }

        let mut new_secondary = cells.to_vec();
        new_secondary.sort_unstable();
        new_secondary.dedup();

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.secondary == new_secondary {
            return Ok(());
        }
        let cell_stroke_width = handles.cell_stroke_width;
        let single_value = handles.cell_grid.is_some_and(|grid| grid.single_value);

        // Only a cell that enters or leaves the set can change style, so walk the two sorted lists' own symmetric
        // difference rather than every cell in the grid.
        try_for_each_difference(
            &handles.secondary,
            &new_secondary,
            |i, now_in_set| -> Result<(), svg_dom::Error> {
                let Some(cell) = handles.cell_rects.get(i) else { return Ok(()) };
                let unreached = handles.unreached.binary_search(&i).is_ok();
                let style_with =
                    |secondary: bool| highlight.style(i, secondary, unreached, base_colour, cell_stroke_width);
                let new_style = style_with(now_in_set);
                let old_style = style_with(!now_in_set);
                if new_style != old_style {
                    new_style.apply_from(old_style, single_value, cell, handles.cell_texts.get(i))?;
                }
                Ok(())
            },
        )?;

        handles.secondary = new_secondary;
        handles.refresh_label()?;
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's own primary selection, resolved against its content's real grid shape: the row or column band, and the one
/// focused cell within it, if any. The two together decide every cell's own highlight.
#[derive(Default)]
pub(super) struct Highlight {
    pub(super) band: ResolvedBand,
    pub(super) focus: Option<usize>,
}

impl Highlight {
    /// Resolves `selection` against `content`. `None` if it names a row, column or cell the content does not have.
    pub(super) fn resolve(content: &DataNodeContent, selection: Selection) -> Option<Self> {
        let (band, focus) = content.resolve_selection(selection)?;
        Some(Self { band, focus })
    }

    /// Cell `i`'s style under this highlight, given whether it is also a secondary cell, or not yet computed. See
    /// [`cell_style`] for the precedence.
    pub(super) fn style(
        &self,
        i: usize,
        secondary: bool,
        unreached: bool,
        base_colour: &'static str,
        base_stroke_width: &'static str,
    ) -> CellStyle {
        cell_style(
            Some(i) == self.focus,
            self.band.contains(i),
            secondary,
            unreached,
            base_colour,
            base_stroke_width,
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `visit(i)` once for every cell index whose own category (focused, banded, or default) could differ between the
/// `old` highlight and the `new` one, in a grid of `len` cells. Never the whole grid. A cell outside this set is
/// provably unchanged: it is neither an old/new focus, nor in the symmetric difference of the two bands. So
/// [`cell_style`] resolves it to the same category either way. See `ResolvedBand::for_each_index`'s own doc comment.
///
/// A live "previous"/"next" control stepping through an array only ever touches a handful of cells per step, however
/// large the array. This is what keeps both the DOM writes and the computation deciding them that small.
///
/// The old and new focus come first, the new only if it differs. The two bands then contribute their true symmetric
/// difference, not each walked in full. A member of both bands is skipped in both traversals, since its own category
/// cannot have changed between them. A focus index is skipped there too, since it was visited explicitly already.
fn for_each_changed_cell(old: &Highlight, new: &Highlight, len: usize, mut visit: impl FnMut(usize)) {
    if let Some(i) = old.focus {
        visit(i);
    }
    if new.focus != old.focus {
        if let Some(i) = new.focus {
            visit(i);
        }
    }
    if old.band != new.band {
        let is_focus = |i: usize| Some(i) == old.focus || Some(i) == new.focus;
        old.band.for_each_index(len, |i| {
            if !new.band.contains(i) && !is_focus(i) {
                visit(i);
            }
        });
        new.band.for_each_index(len, |i| {
            if !old.band.contains(i) && !is_focus(i) {
                visit(i);
            }
        });
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `visit(index, now_in_set)` for every index in exactly one of `old` and `new`: `true` for one only in `new`, so
/// it enters the set, and `false` for one only in `old`, so it leaves. Both must be sorted and free of duplicates.
/// Stops at, and returns, the first error `visit` gives.
///
/// One merge walk over both lists, in index order. It builds no list of changes and searches nothing, so it costs
/// `old.len() + new.len()` steps.
pub(super) fn try_for_each_difference<E>(
    old: &[usize],
    new: &[usize],
    mut visit: impl FnMut(usize, bool) -> Result<(), E>,
) -> Result<(), E> {
    let (mut o, mut n) = (0, 0);
    while o < old.len() || n < new.len() {
        match (old.get(o), new.get(n)) {
            (Some(&a), Some(&b)) if a == b => {
                o += 1;
                n += 1;
            },
            (Some(&a), Some(&b)) if a < b => {
                visit(a, false)?;
                o += 1;
            },
            (Some(_), Some(&b)) => {
                visit(b, true)?;
                n += 1;
            },
            (Some(&a), None) => {
                visit(a, false)?;
                o += 1;
            },
            (None, Some(&b)) => {
                visit(b, true)?;
                n += 1;
            },
            (None, None) => break,
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
