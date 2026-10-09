//! Adding a data node to a `Scene`, and measuring one without adding it.

use super::super::{NodeOptions, validate_data_content, validate_edge_anchors};
use super::{
    draw::{draw_content_box, measure_content_box},
    validate_node_name,
};
use crate::{
    error::Error,
    model::node::NodeId,
    scene::{DataNodeContent, Scene},
};
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a data node to the graph, and returns its id. Its visible content is `content`'s own grid of values (see
    /// [`DataNodeContent`]) rather than a plain text label.
    ///
    /// Unlike [`add_node`](Self::add_node), there is no `size` parameter. The box is always sized to fit `content`'s
    /// rendered grid exactly. See [`DataNodeContent`]'s own doc comment for the layout and formatting rules, and
    /// `draw_content_box`'s own doc comment for how the fit is computed.
    ///
    /// Equivalent to [`add_data_node_with`](Self::add_data_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values — there is no grid to draw.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0`.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite.
    pub fn add_data_node(&self, top_left: Point, content: DataNodeContent) -> Result<NodeId, Error> {
        self.add_data_node_with(top_left, content, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a data node to the graph, as [`add_data_node`](Self::add_data_node), but with `options` controlling how
    /// many connector fixing points this node's sides offer — see [`EdgeAnchors`](crate::scene::EdgeAnchors).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0` —
    /// `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite. Unlike
    /// [`add_node_with`](Self::add_node_with), there is no caller-supplied size to validate — the box is always sized
    /// to fit `content`.
    pub fn add_data_node_with(
        &self,
        top_left: Point,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        self.add_data_node_with_impl(top_left, None, content, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives a raw value a variable name. Adds a data node to the graph, exactly as
    /// [`add_data_node`](Self::add_data_node), but wrapped in a further outer box of its own labelled `name`. The same
    /// shape an "outer box labelled with a name with an inset value box beneath it" already drawn for an operator
    /// node's own result.
    ///
    /// Equivalent to [`add_named_data_node_with`](Self::add_named_data_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_named_data_node_with`](Self::add_named_data_node_with)'s own `# Errors` section.
    pub fn add_named_data_node(&self, top_left: Point, name: &str, content: DataNodeContent) -> Result<NodeId, Error> {
        self.add_named_data_node_with(top_left, name, content, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a named data node, as [`add_named_data_node`](Self::add_named_data_node), but with `options` controlling
    /// how many connector fixing points this node's sides offer — see [`EdgeAnchors`](crate::scene::EdgeAnchors).
    ///
    /// An incoming connector still anchors to this node's own *outer* (named) box, never to the inner content box
    /// `name` wraps. This is the same reasoning [`add_binary_operator_node`](Self::add_binary_operator_node)'s own
    /// module doc comment gives for why a connector must never land on an inset inner box.
    ///
    /// `name` is not stored in the graph's own model. Unlike [`add_node`](Self::add_node)'s own `label`, it exists only
    /// to draw this one label row. An operator's own label (`"ADD"`, `"XOR"`, …) is never stored either, in the same
    /// way. Query this node's own value(s) back through `content` itself, exactly as for an unnamed data node.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::EmptyNodeName`] if `name` is empty, or holds only whitespace. Also checked before drawing
    /// anything — see [`Error::EmptyNodeName`]'s own doc comment for why a blank name is rejected outright, rather than
    /// merely drawing an oddly-worded label.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0` —
    /// `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite. Unlike
    /// [`add_node_with`](Self::add_node_with), there is no caller-supplied size to validate — the box is always sized
    /// to fit `content` and `name` together.
    pub fn add_named_data_node_with(
        &self,
        top_left: Point,
        name: &str,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        self.add_data_node_with_impl(top_left, Some(name), content, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `Size` at which an unnamed data node showing `content` would render in *this* `Scene`. This is exactly what
    /// [`add_data_node`](Self::add_data_node) followed by [`Scene::node_rect`] on the result would report, without ever
    /// adding `content` to the graph to find out.
    ///
    /// The node is drawn into this `Scene`'s own `SvgRoot`, so the same styling context applies. The result is then
    /// measured, and the node is removed again before returning.
    ///
    /// Structurally, nothing about this call persists. No [`NodeId`] is returned, because nothing remains to address
    /// afterward. No graph node is created, no node handle is registered, and no edge or accessibility/navigation state
    /// is touched. The drawn content itself exists only for the instant between `draw_content_box` returning and this
    /// function removing it again. In practice it is never visible, selectable, or reachable by assistive technology.
    /// That stronger claim about transient browser behaviour is not itself something a test here establishes, only the
    /// structural absence above.
    ///
    /// This is most useful for sizing content that cannot be known until runtime. Character count is not equivalent to
    /// rendered width in general, so this always performs a real measurement rather than attempting a best guess.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values, or [`Error::InvalidGridLayout`] if its own
    /// [`GridLayout`](crate::scene::GridLayout) wraps `0` — the identical validation
    /// [`add_data_node`](Self::add_data_node) already applies, not a second copy of it.
    pub fn measure_data_node(&self, content: &DataNodeContent) -> Result<Size, Error> {
        self.measure_data_node_impl(None, content)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The named-node counterpart to [`measure_data_node`](Self::measure_data_node) — `name`'s own label row is
    /// included in the measured size. See that method's own doc comment for everything else.
    ///
    /// # Errors
    ///
    /// As [`measure_data_node`](Self::measure_data_node), plus [`Error::EmptyNodeName`] if `name` is empty or holds
    /// only whitespace — the same rejection [`add_named_data_node`](Self::add_named_data_node) already applies.
    pub fn measure_named_data_node(&self, name: &str, content: &DataNodeContent) -> Result<Size, Error> {
        self.measure_data_node_impl(Some(name), content)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shared implementation behind [`measure_data_node`](Self::measure_data_node) and
    /// [`measure_named_data_node`](Self::measure_named_data_node) — the measurement counterpart to
    /// [`add_data_node_with_impl`](Self::add_data_node_with_impl). It applies the same validation, then asks
    /// [`measure_content_box`] for the size drawing would give, without drawing any cell.
    fn measure_data_node_impl(&self, name: Option<&str>, content: &DataNodeContent) -> Result<Size, Error> {
        if let Some(name) = name {
            validate_node_name(name)?;
        }
        validate_data_content(content)?;

        measure_content_box(&self.inner.borrow().svg, name, content)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shared implementation behind [`add_data_node_with`](Self::add_data_node_with) and
    /// [`add_named_data_node_with`](Self::add_named_data_node_with) — every check, and the rendered box itself (modulo
    /// `name`'s own outer wrapper), is identical between a plain and a named data node.
    ///
    /// See either public wrapper's own doc comment for the full contract this enforces.
    fn add_data_node_with_impl(
        &self,
        top_left: Point,
        name: Option<&str>,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        if let Some(name) = name {
            validate_node_name(name)?;
        }
        validate_data_content(&content)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(0.0, 0.0),
            }));
        }

        let mut inner = self.inner.borrow_mut();
        // See `add_node_with`'s own matching comment for why `scratch` is taken out for the call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_content_box(&inner.svg, &mut scratch, top_left, name, &content, options.edge_anchors);
        inner.scratch = scratch;
        let (handles, rect) = result?;
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, content);
        inner.insert_node_handle(id, handles);
        Ok(id)
    }
}
