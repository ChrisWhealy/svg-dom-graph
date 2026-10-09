use super::SceneInner;
use crate::{
    model::{content::Selection, edge::EdgeId, node::NodeId},
    scene::node::EdgeAnchors,
};
use std::{cell::RefCell, rc::Rc};
use svg_dom::{SvgNode, root::utils::Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered elements that make up one box, kept so a drag handler can reposition them.
///
/// Every one of a box's own children — its outer rect, its label or grid cells — is drawn once, at creation, in local
/// coordinates relative to `(0, 0)`.
///
/// `group`'s own `transform="translate(...)"` is the only thing that ever changes afterward.
///
/// [`SceneInner::move_node`] repositions a box by rewriting this one transform. This cost stays the same regardless of
/// how many children a `group` might hold. So a data node with hundreds of value cells can be moved as cheaply as a
/// plain label. Moving a box needs no handle to any individual child beyond `group` — but recolouring one for
/// [`Scene::set_selection`](crate::scene::Scene::set_selection) does, hence `cell_rects` below.
pub(crate) struct BoxHandles {
    /// Event listeners attach here, so a click on any child starts a drag.
    pub(crate) group: SvgNode,
    /// This box's own outer `<rect>` — the one element every node kind draws, named label or not, data grid or not. For
    /// a named data node this is the wrapping box around the label row and the grid together, not the grid's own inner
    /// background. Every other kind has only the one rect, so it serves as both.
    ///
    /// `Scene::set_focus`'s only reader — a node has exactly one outer box to ring, regardless of how many
    /// [`cell_rects`](Self::cell_rects) it holds within it.
    pub(crate) outer_rect: SvgNode,
    /// Whether `Scene::make_draggable`/`Scene::make_draggable_with` has already been called for this node.
    ///
    /// `svg-dom`'s listener registration is append-only, so a second call would add a second, independent set of
    /// pointer listeners rather than replacing the first — see [`crate::Error::AlreadyDraggable`].
    pub(crate) draggable: bool,
    /// Whether `Scene::make_enterable` has already been called for this node.
    ///
    /// Same reasoning as [`draggable`](Self::draggable), for the same underlying cause. `svg-dom`'s listener
    /// registration is append-only, so a second call would add a second, independent click/keydown listener rather than
    /// replacing the first. See [`crate::Error::AlreadyEnterable`].
    pub(crate) enterable: bool,
    /// How many evenly spaced connector fixing points this node's own sides offer — see [`EdgeAnchors`].
    ///
    /// `redraw_edge` has no other way to learn a node's own anchor configuration once an incident edge needs a reroute.
    /// This value must live alongside the rendered handle, not just get used once at creation.
    pub(crate) edge_anchors: Option<EdgeAnchors>,
    /// `Some((inputs.0, inputs.1))` for a two-input operator node — a `BinaryOperator` or `ArithmeticOperator` one —
    /// its own two operand ids, in the order its own constructor received them. Not "left"/"right". The anti-crossing
    /// router freely reassigns which operand's connector lands on which visual side, so this order is a stable operand
    /// *identity*, never a stable position. See `node::operator::draw_port_marker`'s own doc comment for the
    /// non-commutative "L"/"R" marker that makes that identity visible to a reader too. `None` for every other node,
    /// unary operator nodes included, since only a two-input node's inputs can ever collide on the same side.
    ///
    /// `SceneInner::binary_operator_to_override` reads this on every redraw, so the two connectors split apart whenever
    /// they land on the same side, live — not just once, at creation.
    pub(crate) binary_operator_inputs: Option<(NodeId, NodeId)>,
    /// `Some((edge for inputs.0, edge for inputs.1))` for a two-input operator node — the ids of the two edges its own
    /// constructor auto-wired from `binary_operator_inputs.0`/`.1`, in the same order. `None` for every other node,
    /// exactly matching `binary_operator_inputs`.
    ///
    /// `SceneInner::redraw_binary_operator_inputs` reads this to redraw both edges together in one pass, rather than
    /// searching either operand's own incident edges for the one that also points at this operator.
    pub(crate) binary_operator_input_edges: Option<(EdgeId, EdgeId)>,
    /// Every [`crate::scene::DataNodeContent`] cell's own `<rect>`, flat, in the same order
    /// [`crate::scene::DataNodeContent::cells`]/`shape` already use.
    ///
    /// A single-value node or an operator node's own single-value result draws no separate inner cell, so its own lone
    /// entry here is the outer box's own `rect` itself. Empty for a plain label node, which has no cell to select at
    /// all.
    ///
    /// `Scene::set_selection` is the only reader — nothing else needs to reach an individual cell again once it is
    /// drawn.
    pub(crate) cell_rects: Vec<SvgNode>,
    /// Every cell's own `<text>`, flat, in the same order as [`cell_rects`](Self::cell_rects). `Scene::set_data_values`
    /// rewrites them, and `Scene::set_unreached_cells` dims them along with their boxes. Empty for a plain label node.
    /// An operator node has its one result text here too, though [`replaceable`](Self::replaceable) is `false` for it.
    pub(crate) cell_texts: Vec<SvgNode>,
    /// Whether `Scene::set_data_values` may replace this node's values. `true` for a node drawn from a
    /// `DataNodeContent`, `false` for a plain label node and for an operator node, whose result the caller works out
    /// once, when it adds the node.
    pub(crate) replaceable: bool,
    /// Every data-node cell's own box, flat, in the same order as [`cell_rects`](Self::cell_rects), in the node's own
    /// local coordinates. Add the node's own origin for scene coordinates. What `Scene::cell_rect` reports. Empty for
    /// a plain label node and for an operator node.
    pub(crate) cell_geometry: Vec<Rect>,
    /// Every entry in `cell_rects`' own stroke width, as drawn. It is `"1.5"` for a single-value node's own outer box,
    /// and `"1"` for a multi-value grid's inner cells or an operator's own result row. Unused (`""`) for a plain label
    /// node, which has no `cell_rects` to begin with.
    ///
    /// Already formatted, rather than a plain `f64`. Every stroke width this crate ever draws is one of a small fixed
    /// set (this default, or [`Scene::set_selection`](crate::scene::Scene::set_selection)'s own band/focus widths). So
    /// there is no reason to format one from scratch on a hot path. See `svg-dom`'s own `SvgNode::set_stroke_width` doc
    /// comment for why that convenience setter allocates a `String` on every call.
    ///
    /// `Scene::set_selection` restores this on every cell it does not band or focus. That way a selection's own thicker
    /// stroke never lingers once a cell is deselected — see that method's own doc comment for why it uses one.
    pub(crate) cell_stroke_width: &'static str,
    /// The current [`Selection`] `Scene::set_selection` last recoloured this node's own cells to, defaulting to
    /// [`Selection::None`] at creation.
    ///
    /// `Scene::set_selection` compares its own new `Selection` against this before touching anything. An identical
    /// selection is an immediate no-op. Even a genuinely different one only rewrites whichever cells actually changed
    /// category (focused/banded/default), not all `N` of them unconditionally.
    pub(crate) selection: Selection,
    /// The flat indices `Scene::set_secondary_selection` last marked as derived, sorted and without duplicates — empty
    /// until it is first called, and for every node kind with no cells. Independent of [`selection`](Self::selection):
    /// `Scene::set_selection` never changes it, and it never changes `selection`.
    pub(crate) secondary: Vec<usize>,
    /// The flat indices `Scene::set_unreached_cells` last marked as not yet computed, sorted and without duplicates —
    /// empty until it is first called, and for every node kind with no cells. Independent of both
    /// [`selection`](Self::selection) and [`secondary`](Self::secondary), and always overridden by either's own mark on
    /// a cell.
    pub(crate) unreached: Vec<usize>,
    /// This node's own live `aria-label` text, reused in place rather than rebuilt from scratch on every
    /// [`Scene::set_selection`](crate::scene::Scene::set_selection) call.
    ///
    /// Starts off as the node's base description alone; for instance `"u8 data grid, 7 values"`, with no selection
    /// appended. Empty for a plain label node, whose own visible text already serves as its accessible name.
    ///
    /// `Scene::set_selection` truncates this back to [`base_label_len`](Self::base_label_len), then appends the new
    /// selection's own [`Selection::describe_into`](crate::scene::Selection::describe_into) onto what remains. After
    /// the first call grows its capacity, a later selection change needs no further allocation.
    pub(crate) aria_label: String,
    /// `aria_label`'s own length at creation, before any selection was ever appended — the point `Scene::set_selection`
    /// truncates back to before appending a new selection's own description.
    pub(crate) base_label_len: usize,
    /// A short, stable name for this node, so a later node's own description can refer to it by name. A plain label
    /// node uses its own visible text; a named data node uses its own given `name`. An operator node uses its own label
    /// — `"NOT"`, `"XOR"`, `"ROTR 1"`, and so on; an unnamed data node falls back to its own type name instead.
    ///
    /// Set once at construction, this name is never rewritten afterward — unlike `aria_label`, which
    /// `Scene::set_selection` rewrites on every selection change. A node's own name stays fixed for its whole lifetime;
    /// `Scene`'s own public API offers no way to rename one once drawn.
    ///
    /// `current_ref_name` reads this to build a node's own name right now. `SceneInner::append_relationship` reads it
    /// too, seeding a plain label node's first relationship clause.
    pub(crate) ref_name: String,
    /// The nested `Scene` this node owns, if it is a container node — `Some` only for a node whose
    /// [`NodeContent`](crate::model::node::NodeContent) is `Container`. `None` for every other node kind.
    ///
    /// Kept here, not in `NodeContent::Container` itself, because a `Scene`/`SceneInner` is DOM/wasm state and the
    /// graph model is deliberately kept free of that — see `NodeContent`'s own doc comment. This is exactly the same
    /// reasoning `cell_rects`/`selection` above already follow for a data node's own DOM-side state.
    ///
    /// `Scene::enter` reads this to find which `Scene` to show. `Scene::add_container_node`/ `add_container_node_with`
    /// are the only place this is ever set.
    pub(super) child: Option<Rc<RefCell<SceneInner>>>,
}

impl BoxHandles {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Rebuilds this node's own `aria-label` and tooltip from its base description plus whatever `selection` and
    /// `secondary` currently say — `Scene::set_selection` and `Scene::set_secondary_selection` both end here.
    ///
    /// The secondary cells read as `", also highlighted: cells 3, 4"`, appended after the primary selection's own
    /// description. Colour and dash alone convey nothing to assistive technology. Cells not yet computed read as `",
    /// not yet computed: cells 16 to 63"`, after that, for the same reason. See [`unreached_clause`].
    pub(crate) fn refresh_label(&mut self) -> Result<(), svg_dom::Error> {
        use std::fmt::Write as _;
        self.aria_label.truncate(self.base_label_len);
        self.selection.describe_into(&mut self.aria_label);
        if !self.secondary.is_empty() {
            let noun = if self.secondary.len() == 1 { "cell" } else { "cells" };
            let _ = write!(self.aria_label, ", also highlighted: {noun} ");
            for (n, cell) in self.secondary.iter().enumerate() {
                if n > 0 {
                    self.aria_label.push_str(", ");
                }
                let _ = write!(self.aria_label, "{cell}");
            }
        }
        unreached_clause_into(&self.unreached, self.cell_rects.len(), &mut self.aria_label);
        self.group.set_attr("aria-label", &self.aria_label)?;
        // Keeps the browser's own mouse-hover tooltip reading exactly the same text as `aria-label` — see
        // `draw_content_box`'s own doc comment on why `<title>` is set to that same text at construction.
        self.group.set_title(&self.aria_label)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces a single-value node's own value text, `old`, with `new` in its `aria-label` and tooltip, then rebuilds
    /// them. The value follows `"{type_name} = "` in the base description, for example `"u32 = 0A 0B 0C 0D"`. Any
    /// relationship clauses after it, and the selection description, are kept.
    ///
    /// The base description's length is adjusted by the difference, since a decimal value can change length. A label
    /// that does not hold `old` after `"{type_name} = "` is left as it is.
    pub(crate) fn replace_label_value(&mut self, type_name: &str, old: &str, new: &str) -> Result<(), svg_dom::Error> {
        let needle = format!("{type_name} = {old}");
        if let Some(position) = self.aria_label[..self.base_label_len].find(&needle) {
            let start = position + type_name.len() + " = ".len();
            self.aria_label.replace_range(start..start + old.len(), new);
            self.base_label_len = self.base_label_len - old.len() + new.len();
        }
        self.refresh_label()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This node's own name right now, layering whatever `selection` currently highlights onto `ref_name`.
    ///
    /// `ref_name` alone names a node's whole self — "A" for an array `Scene::set_selection` re-bands each step, never
    /// rebuilt. This method adds back the element selection currently picks out, so a later description can call it
    /// "A(0)" on one step, "A(1)" on the next. `ref_name` itself never changes — it stays exactly as fixed at
    /// construction.
    ///
    /// `Selection::None` returns `ref_name` unchanged. `Cell(i)` reads as `"{ref_name}({i})"` — one flat index, the
    /// same notation a 1-D array's own step already uses. A `Row`/`Column` with its own focus index reads as
    /// `"{ref_name}({row}, {col})"` instead — matrix notation, matching `A(row, col)` elsewhere in this crate's own
    /// demos. A `Row`/`Column` with no focus names its whole group instead — `"{ref_name}(row {row})"` or
    /// `"{ref_name}(column {col})"`.
    ///
    /// `Scene::add_edge_with` calls this for both of a new edge's own endpoints, via `SceneInner::append_relationship`.
    /// That is the "A" or "A(0)" a peer node's own new relationship clause names it by.
    pub(crate) fn current_ref_name(&self) -> String {
        match self.selection {
            Selection::None => self.ref_name.clone(),
            Selection::Cell(i) => format!("{}({i})", self.ref_name),
            Selection::Row { row, col: None } => format!("{}(row {row})", self.ref_name),
            Selection::Row { row, col: Some(col) } => format!("{}({row}, {col})", self.ref_name),
            Selection::Column { col, row: None } => format!("{}(column {col})", self.ref_name),
            Selection::Column { col, row: Some(row) } => format!("{}({row}, {col})", self.ref_name),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Appends to `out` how `cells`, sorted and without duplicates, read in a node's accessible name when they are not yet
/// computed, for a node with `total` cells. Appends nothing when there are none.
///
/// A node of a single cell reads as just `", not yet computed"`. Otherwise `", not yet computed: cells 16 to 63"`. A run
/// of three or more consecutive cells is written as a range, so a schedule that is nearly all unreached does not read
/// out dozens of numbers. Shorter runs are listed. Everything is written straight into `out`, so a label rebuilt on
/// every selection change allocates only if `out` must grow.
pub(crate) fn unreached_clause_into(cells: &[usize], total: usize, out: &mut String) {
    use std::fmt::Write as _;
    if cells.is_empty() {
        return;
    }
    if total == 1 {
        out.push_str(", not yet computed");
        return;
    }
    let noun = if cells.len() == 1 { "cell" } else { "cells" };
    let _ = write!(out, ", not yet computed: {noun} ");
    let mut first = true;
    let mut separate = |out: &mut String| {
        if !std::mem::take(&mut first) {
            out.push_str(", ");
        }
    };
    let mut start = 0;
    while start < cells.len() {
        let mut end = start;
        while end + 1 < cells.len() && cells[end + 1] == cells[end] + 1 {
            end += 1;
        }
        if end - start >= 2 {
            separate(out);
            let _ = write!(out, "{} to {}", cells[start], cells[end]);
        } else {
            for cell in &cells[start..=end] {
                separate(out);
                let _ = write!(out, "{cell}");
            }
        }
        start = end + 1;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`unreached_clause_into`] as an owned `String`, for the tests that only read the finished text.
#[cfg(test)]
pub(crate) fn unreached_clause(cells: &[usize], total: usize) -> String {
    let mut out = String::new();
    unreached_clause_into(cells, total, &mut out);
    out
}

#[cfg(test)]
mod unit_tests;
