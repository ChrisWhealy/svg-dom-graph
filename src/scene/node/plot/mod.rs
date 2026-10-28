//! Plot nodes: a graph of values, drawn as a node. A plot has a plane, two axes that are linear or logarithmic, ticks,
//! a grid and a background. Lines, dots and labels are drawn on it by value, and a click on it reports the values at
//! the place that was clicked.
//!
//! A plot is a node like any other. A connector can be anchored to it, it takes the focus ring, and it has an
//! accessible name. It is not made draggable, since a plot is something to read and to click. Like every other node, it
//! does no arithmetic. The caller works out what is drawn.
//!
//! What is fixed is drawn once, by [`draw`]: the box, the grid, the axes, the ticks and the titles. What is plotted is
//! in a group of its own, which [`Scene::set_plot_items`] replaces whole. Redrawing a plot as a slider moves is then
//! the cost of its items, and nothing else.

mod draw;
mod layer;
mod paint;

use super::{NodeOptions, render_guard::RenderGuard};
use crate::{
    error::Error,
    geometry::{apply_matrix, invert_matrix},
    model::{
        node::{NodeContent, NodeId},
        plot::{PlotClick, PlotConfig, PlotConfigError, PlotItem, PlotLayout, default_description},
    },
    scene::{BoxHandles, DataDecor, Scene, Selection},
};
use paint::Painter;
use std::rc::Rc;
use svg_dom::{
    SvgNode,
    root::utils::{Point, Rect, Size},
};

/// What a plot node keeps, beyond what every node does, so that it can be drawn on and clicked.
pub(in crate::scene) struct PlotHandles {
    /// Where everything of the plot is, which is all that is needed to place a value and to find one.
    layout: PlotLayout,
    /// The group that holds what is plotted. It is replaced whole by `set_plot_items`.
    layer: SvgNode,
    /// The plotted area's rectangle, which takes the pointer cursor once there is a click handler.
    area_rect: SvgNode,
    /// The background, which is the colour of every halo.
    background: String,
    /// The description at the front of the node's accessible name. Connectors append what they connect after it.
    description: String,
    /// Whether `on_plot_click` has been called for this node.
    has_click: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The style of the plotted area once it can be clicked.
const CLICKABLE_STYLE: &str = "cursor: crosshair";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a plot node's group, with its transform at `top_left`. A [`Painter`] removes whatever was drawn if any step
/// fails.
fn draw_plot_box(
    svg: &svg_dom::SvgRoot,
    scratch: &mut String,
    top_left: Point,
    config: &PlotConfig,
    layout: &PlotLayout,
    description: &str,
) -> Result<(BoxHandles, PlotHandles, Rect), Error> {
    let group = svg.group()?;
    let mut painter = Painter::new(svg, group.clone());
    let outer = draw::draw_frame(&mut painter, config, layout)?;
    // The group that holds what is plotted comes last, so that it is over the grid and the axes, and under nothing.
    let layer = svg.group()?;
    let guard = RenderGuard::new(layer.clone());
    group.append(&layer)?;
    guard.disarm();
    let area_rect = area_rect_of(&group)?;
    let group = painter.finish();

    group.set_attr("role", "img")?;
    group.set_attr("aria-label", description)?;
    group.set_title(description)?;
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

    let handles = BoxHandles {
        group,
        draggable: false,
        enterable: false,
        edge_anchors: None,
        binary_operator_inputs: None,
        operator_input_side: None,
        binary_operator_input_edges: None,
        outer_rect: outer,
        cell_rects: Vec::new(),
        cell_texts: Vec::new(),
        replaceable: false,
        cell_grid: None,
        decor: DataDecor::default(),
        cell_stroke_width: "",
        selection: Selection::None,
        secondary: Vec::new(),
        unreached: Vec::new(),
        aria_label: description.to_owned(),
        base_label_len: description.len(),
        label_stale: false,
        edge_anchors_stale: false,
        ref_name: description.to_owned(),
        child: None,
        plot: None,
    };
    let plot = PlotHandles {
        layout: *layout,
        layer,
        area_rect,
        background: config.background.clone(),
        description: description.to_owned(),
        has_click: false,
    };
    Ok((
        handles,
        plot,
        Rect {
            origin: top_left,
            size: config.size,
        },
    ))
}

/// The rectangle of the plotted area: the second `<rect>` of the group, after the outer box.
fn area_rect_of(group: &SvgNode) -> Result<SvgNode, Error> {
    group
        .children()
        .into_iter()
        .filter(|child| child.as_element().tag_name().eq_ignore_ascii_case("rect"))
        .nth(1)
        .ok_or_else(|| svg_dom::Error::Dom("plot: the plotted area was not drawn".into()).into())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The plot handles of node `id`, or the error for a node that is not a plot.
fn plot_of(inner: &crate::scene::scene_inner::SceneInner, id: NodeId) -> Result<&PlotHandles, Error> {
    inner.graph.node(id).ok_or(Error::UnknownNode(id))?;
    inner
        .node_handle(id)
        .and_then(|handles| handles.plot.as_ref())
        .ok_or(Error::NotAPlot(id))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a plot node at `top_left`, and returns its id. Nothing is plotted on it yet:
    /// [`set_plot_items`](Self::set_plot_items) draws lines, dots and labels on it.
    ///
    /// The node is as big as `config.size`. It draws a background, a plotted area, a grid, two axes with ticks and
    /// labels, titles and a legend, as `config` says. See [`PlotConfig`] for what it can say.
    ///
    /// # Accessibility
    ///
    /// The node is an image, named by `config.description`. A plot that has none is named from its axes: "Real plane
    /// plot: horizontal axis linear from 0 to 1, vertical axis …". What is plotted is not in the name, so a caller that
    /// changes it should say so with [`set_plot_description`](Self::set_plot_description), and say it elsewhere too, in
    /// text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPlotConfig`] if `config` cannot be drawn: an axis that does not span a range, a
    /// logarithmic axis from zero or below, an interval that makes too many ticks, a background that is not a hex
    /// colour, and so on. Returns [`Error::InvalidNodeGeometry`] if `top_left` is not finite. Every check is made
    /// before anything is drawn, so a rejected call leaves the scene exactly as it was.
    pub fn add_plot_node(&self, top_left: Point, config: PlotConfig) -> Result<NodeId, Error> {
        self.add_plot_node_with(top_left, config, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a plot node, as [`add_plot_node`](Self::add_plot_node), but with `options` controlling how many connector
    /// fixing points its sides offer — see [`EdgeAnchors`](crate::scene::EdgeAnchors). Its fill is the plot's own
    /// background, so `options.fill` is not read.
    ///
    /// # Errors
    ///
    /// As [`add_plot_node`](Self::add_plot_node), plus [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is
    /// `Some(EdgeAnchors(0))`.
    pub fn add_plot_node_with(
        &self,
        top_left: Point,
        config: PlotConfig,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        super::validate_edge_anchors(options.edge_anchors)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: config.size,
            }));
        }
        let layout = PlotLayout::new(&config).map_err(Error::InvalidPlotConfig)?;
        let description = config.description.clone().unwrap_or_else(|| default_description(&config));

        let mut inner = self.inner.borrow_mut();
        let mut scratch = std::mem::take(&mut inner.scratch);
        let drawn = draw_plot_box(&inner.svg, &mut scratch, top_left, &config, &layout, &description);
        inner.scratch = scratch;
        let (mut handles, plot, rect) = drawn?;
        handles.edge_anchors = options.edge_anchors;
        handles.plot = Some(plot);
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, NodeContent::Plot(description));
        inner.insert_node_handle(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws `items` on plot node `id`, in place of whatever was drawn on it before. They are drawn in order, so a
    /// later one is over an earlier one.
    ///
    /// Only what is plotted changes. The grid, the axes and the labels stay as they are. So this is the call to make as
    /// often as a slider moves.
    ///
    /// A line is cut where it leaves the plotted area, and a dot or a label outside it is not drawn. A point that
    /// cannot be placed, because it is not finite, or is zero or less on a logarithmic axis, is skipped, and breaks a
    /// line there.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene, or [`Error::NotAPlot`] if it names a
    /// node of any other kind. Returns a wrapped [`Error::Svg`] if a write fails. The new items are drawn out of the
    /// way first and only then put in place of the old, so a failure leaves what was plotted before exactly as it was.
    pub fn set_plot_items(&self, id: NodeId, items: &[PlotItem]) -> Result<(), Error> {
        let (fresh, group, old) = {
            let inner = self.inner.borrow();
            let plot = plot_of(&inner, id)?;
            let group = inner.node_handle(id).ok_or(Error::NotAPlot(id))?.group.clone();
            // Nothing of the scene is changed while this is drawn, so the borrow can be held.
            let mut painter = Painter::new(&inner.svg, inner.svg.group()?);
            layer::draw_items(&mut painter, &plot.layout, &plot.background, items)?;
            (painter.finish(), group, plot.layer.clone())
        };
        // The new group is put in the node's own group, and only then is the old layer removed.
        if let Err(e) = group.append(&fresh) {
            fresh.remove();
            return Err(e.into());
        }
        old.remove();
        let mut inner = self.inner.borrow_mut();
        if let Some(plot) = inner.node_handle_mut(id).and_then(|handles| handles.plot.as_mut()) {
            plot.layer = fresh;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces what assistive technology reads for plot node `id` with `description`, which the browser also shows as
    /// its tooltip. Relationships that connectors gave the node are kept after it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPlotConfig`] if `description` is empty or only whitespace. Returns
    /// [`Error::UnknownNode`] if `id` does not name a node in this scene, or [`Error::NotAPlot`] if it names a node of
    /// any other kind.
    pub fn set_plot_description(&self, id: NodeId, description: &str) -> Result<(), Error> {
        if description.trim().is_empty() {
            return Err(Error::InvalidPlotConfig(PlotConfigError::EmptyDescription));
        }
        let mut inner = self.inner.borrow_mut();
        plot_of(&inner, id)?;
        let handles = inner.node_handle_mut(id).ok_or(Error::NotAPlot(id))?;
        let old = handles.plot.as_ref().map(|plot| plot.description.clone()).unwrap_or_default();
        // What connectors appended follows the old description, and is kept after the new one.
        let relationships = handles.aria_label.strip_prefix(&old).unwrap_or_default().to_owned();
        let appended = handles.base_label_len.saturating_sub(old.len());
        handles.aria_label.clear();
        handles.aria_label.push_str(description);
        handles.aria_label.push_str(&relationships);
        handles.base_label_len = description.len() + appended;
        handles.group.set_attr("aria-label", &handles.aria_label)?;
        handles.group.set_title(&handles.aria_label)?;
        if let Some(plot) = handles.plot.as_mut() {
            plot.description = description.to_owned();
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The plotted area of plot node `id`: its rectangle in the `<svg>`'s own user space, without the margins. It is
    /// where the grid is, and where a click is reported.
    ///
    /// A host that lays something out against a plot, such as a slider that is as long as an axis, reads it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene, or [`Error::NotAPlot`] if it names a
    /// node of any other kind.
    pub fn plot_area(&self, id: NodeId) -> Result<Rect, Error> {
        let inner = self.inner.borrow();
        let plot = plot_of(&inner, id)?;
        let origin = inner.node_rect(id)?.origin;
        let area = plot.layout.area;
        Ok(Rect {
            origin: Point::new(origin.x + area.left, origin.y + area.top),
            size: Size::new(area.right - area.left, area.bottom - area.top),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the point `(x, y)` of plot node `id` is, in the `<svg>`'s own user space. `None` if it cannot be placed,
    /// because a value is not finite, or is zero or less on a logarithmic axis. A point beyond the axes is placed
    /// beyond the plotted area.
    ///
    /// # Errors
    ///
    /// As [`plot_area`](Self::plot_area).
    pub fn plot_point_to_scene(&self, id: NodeId, x: f64, y: f64) -> Result<Option<Point>, Error> {
        let inner = self.inner.borrow();
        let plot = plot_of(&inner, id)?;
        let origin = inner.node_rect(id)?.origin;
        Ok(plot.layout.point(x, y).map(|(px, py)| Point::new(origin.x + px, origin.y + py)))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `on_click` each time plot node `id` is clicked inside its plotted area, with this scene, the node's id and
    /// the values of the axes at the place that was clicked. A click in the margin round the plotted area is not
    /// reported.
    ///
    /// The plotted area takes the crosshair cursor, so that it can be seen to be clickable.
    ///
    /// A click is a mouse or touch gesture. It cannot be made from the keyboard, so a caller that reacts to it should
    /// offer another way to the same result: a slider, or a text box.
    ///
    /// # Callback lifecycle
    ///
    /// `on_click` is called with nothing of this scene borrowed, so it may call back into it, even to draw the plot
    /// again.
    ///
    /// # Ownership
    ///
    /// `on_click` is held by the node's own event listener, which holds only a `Weak` reference to this scene. So the
    /// scene is not kept alive by it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene, or [`Error::NotAPlot`] if it names a
    /// node of any other kind. Returns [`Error::AlreadyHasClickHandler`] if this was called before for `id`: a listener
    /// cannot be removed, so a second would not replace the first.
    pub fn on_plot_click(
        &self,
        id: NodeId,
        on_click: impl Fn(&Scene, NodeId, PlotClick) + 'static,
    ) -> Result<(), Error> {
        let (group, area_rect) = {
            let inner = self.inner.borrow();
            let plot = plot_of(&inner, id)?;
            if plot.has_click {
                return Err(Error::AlreadyHasClickHandler(id));
            }
            let group = inner.node_handle(id).ok_or(Error::NotAPlot(id))?.group.clone();
            (group, plot.area_rect.clone())
        };
        let weak = Rc::downgrade(&self.inner);
        group.on_click(move |event| {
            let Some(inner) = weak.upgrade() else { return };
            let client = Point::new(f64::from(event.client_x()), f64::from(event.client_y()));
            let click = {
                let borrowed = inner.borrow();
                click_at(&borrowed, id, client)
            };
            // Nothing of the scene is borrowed here, so `on_click` can use it freely.
            if let Some(click) = click {
                on_click(&Scene { inner }, id, click);
            }
        })?;
        area_rect.set_attr("style", CLICKABLE_STYLE)?;
        let mut inner = self.inner.borrow_mut();
        if let Some(plot) = inner.node_handle_mut(id).and_then(|handles| handles.plot.as_mut()) {
            plot.has_click = true;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values at the viewport position `client`, on plot node `id`, or `None` if it is not inside the plotted area.
///
/// The position is taken into the `<svg>`'s user space through the content layer, which carries the zoom and the pan,
/// and then into the node's own space by taking away where the node is.
fn click_at(inner: &crate::scene::scene_inner::SceneInner, id: NodeId, client: Point) -> Option<PlotClick> {
    let plot = plot_of(inner, id).ok()?;
    let inverse = inner.content.screen_ctm().and_then(invert_matrix)?;
    let user = apply_matrix(inverse, client);
    let origin = inner.node_rect(id).ok()?.origin;
    let (x, y) = plot.layout.values_at((user.x - origin.x, user.y - origin.y))?;
    Some(PlotClick { x, y })
}
