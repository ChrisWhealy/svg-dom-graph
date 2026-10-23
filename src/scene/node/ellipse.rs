//! Elliptical plain nodes: an ellipse with a single centred text label, and the options that shape one.
//!
//! The ellipse is drawn as a real SVG `<ellipse>`. Its group carries the translation, and its children are drawn in
//! local coordinates, exactly as for a rectangular node. So moving one only ever writes the group's `transform`, and
//! the graph's model keeps the rectangle that encloses the ellipse as the node's `rect`. Hit testing, dragging, view
//! bounds and collision tests all go on using that rectangle, which makes them conservative at the curved corners. The
//! radii are kept beside it in the model, so that routing never has to guess a shape from a rectangle.
//!
//! The label is never shrunk. If it, with its margin, would cross the curved outline, the ellipse is made larger by one
//! factor on both radii, so a circle stays a circle. The size is worked out once, when the node is added, from the
//! label as the browser measures it. See [`crate::geometry::ellipse`] for the maths.

use super::{LABEL_FONT_SIZE, plain::plain_box_handles, render_guard::RenderGuard};
use crate::{
    colours::{BOX_STROKE, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    geometry::ellipse::{EllipseRadii, enclosing_rect, fitted_radii},
    model::node::NodeId,
    scene::{BoxHandles, Scene},
};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};

/// The clear space kept between a label's box and the curved outline, in the scene's user units, when
/// [`EllipseOptions`] does not say otherwise.
pub(crate) const DEFAULT_INNER_MARGIN: f64 = 6.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures [`Scene::add_ellipse_node_with`]: the fill, and the clear space kept round the label.
///
/// Build one with [`EllipseOptions::default`] and the `with_` methods. A struct literal does not compile outside this
/// crate, so a field can be added later without breaking a caller.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, as it is for [`NodeOptions`](crate::scene::NodeOptions).
/// Every field this type gains must itself stay `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseOptions {
    fill: Option<&'static str>,
    inner_margin: f64,
}

impl Default for EllipseOptions {
    fn default() -> Self {
        Self {
            fill: None,
            inner_margin: DEFAULT_INNER_MARGIN,
        }
    }
}

impl EllipseOptions {
    /// Returns `self` with `fill` set: the fill of the ellipse, as any SVG paint such as `"#d6f2ee"`. `None` keeps the
    /// usual light blue, the same as a plain rectangular node.
    ///
    /// ```
    /// use svg_dom_graph::scene::EllipseOptions;
    /// let options = EllipseOptions::default().with_fill(Some("#d6f2ee"));
    /// assert_eq!(options.fill(), Some("#d6f2ee"));
    /// ```
    #[must_use]
    pub fn with_fill(mut self, fill: Option<&'static str>) -> Self {
        self.fill = fill;
        self
    }

    /// Returns `self` with the clear space kept between the label's box and the curved outline set to `inner_margin`
    /// user units. It must be finite and `>= 0.0`, which [`Scene::add_ellipse_node_with`] checks. The default is 6.
    ///
    /// ```
    /// use svg_dom_graph::scene::EllipseOptions;
    /// let options = EllipseOptions::default().with_inner_margin(4.0);
    /// assert_eq!(options.inner_margin(), 4.0);
    /// ```
    #[must_use]
    pub fn with_inner_margin(mut self, inner_margin: f64) -> Self {
        self.inner_margin = inner_margin;
        self
    }

    /// The fill, if one was chosen.
    pub fn fill(&self) -> Option<&'static str> {
        self.fill
    }

    /// The clear space kept round the label, in user units.
    pub fn inner_margin(&self) -> f64 {
        self.inner_margin
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What drawing an ellipse produced: its handles, the radii it ended up with once the label was fitted, and the
/// rectangle that encloses it.
pub(super) struct DrawnEllipse {
    pub(super) handles: BoxHandles,
    pub(super) radii: EllipseRadii,
    pub(super) rect: svg_dom::root::utils::Rect,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws an ellipse and its centred label under one `<g>`.
///
/// The label is measured first, at its normal size, and the radii are fitted to it. Every child is then drawn in local
/// coordinates, with the ellipse's own centre at `(rx, ry)`, and the group carries the origin of the enclosing
/// rectangle. The label's *visual* bounding box is centred on the ellipse, which compensates for any offset the
/// browser gives a text's box from the point it was placed at.
///
/// A [`RenderGuard`] removes whatever was created if any step fails.
fn draw_ellipse(
    svg: &SvgRoot,
    scratch: &mut String,
    centre: Point,
    radii: EllipseRadii,
    label: &str,
    fill: Option<&str>,
    margin: f64,
) -> Result<DrawnEllipse, Error> {
    let invalid = || Error::InvalidEllipseGeometry {
        centre,
        rx: radii.rx(),
        ry: radii.ry(),
    };

    let group = svg.group()?;
    let mut guard = RenderGuard::new(group.clone());

    let label_el = svg.text(Point::origin(), label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;

    // Where the visual box of the text is, relative to the point it was placed at, and how big it is.
    let measured = label_el.bounding_box()?;
    let offset = Point::new(
        measured.origin.x + measured.size.width / 2.0,
        measured.origin.y + measured.size.height / 2.0,
    );
    let fitted = fitted_radii(radii, measured.size, margin).ok_or_else(invalid)?;
    let rect = enclosing_rect(centre, fitted).ok_or_else(invalid)?;
    let (rx, ry) = (fitted.rx(), fitted.ry());
    if !offset.x.is_finite() || !offset.y.is_finite() {
        return Err(invalid());
    }

    let ellipse_el = svg.ellipse(Point::new(rx, ry), Size::new(rx, ry))?;
    guard.track(ellipse_el.clone());
    ellipse_el.set_fill(fill.unwrap_or(PLAIN_BOX_FILL))?;
    ellipse_el.set_stroke(BOX_STROKE)?;
    ellipse_el.set_stroke_width(1.5)?;

    // Moves the text so that the middle of its visual box is the middle of the ellipse.
    label_el.set_attr_display(scratch, "x", rx - offset.x)?;
    label_el.set_attr_display(scratch, "y", ry - offset.y)?;

    group.append(&ellipse_el)?;
    group.append(&label_el)?;
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", rect.origin.x, rect.origin.y))?;

    guard.disarm();
    Ok(DrawnEllipse {
        handles: plain_box_handles(group, ellipse_el, None, label),
        radii: fitted,
        rect,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a circular node with a centred label, and returns its id.
    ///
    /// A circle is an ellipse whose two radii are equal, so this is [`add_ellipse_node`](Self::add_ellipse_node) with
    /// [`EllipseRadii::circle`]`(radius)`. Everything said there applies: `centre` is the centre of the circle and stays
    /// put when the label makes the circle grow, the label is never made smaller, the circle grows by one factor in both
    /// directions so it stays a circle, and [`node_rect`](Self::node_rect) gives the final enclosing square.
    ///
    /// # Errors
    ///
    /// See [`add_circle_node_with`](Self::add_circle_node_with).
    pub fn add_circle_node(&self, centre: Point, radius: f64, label: impl Into<String>) -> Result<NodeId, Error> {
        self.add_circle_node_with(centre, radius, label, EllipseOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a circular node with a centred label, and returns its id. `options` sets the fill and the clear space round
    /// the label, as for [`add_ellipse_node_with`](Self::add_ellipse_node_with).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEllipseMargin`] if the inner margin is not a finite value `>= 0.0`.
    ///
    /// Returns [`Error::InvalidEllipseGeometry`] if the centre is not finite, if `radius` is not finite and greater than
    /// zero, or if the radius the label needs, or the square that encloses it, is not finite. Both radii in the error
    /// are `radius`.
    ///
    /// All of these are checked before anything is drawn or recorded, so a rejected call leaves the scene exactly as it
    /// was.
    pub fn add_circle_node_with(
        &self,
        centre: Point,
        radius: f64,
        label: impl Into<String>,
        options: EllipseOptions,
    ) -> Result<NodeId, Error> {
        self.add_ellipse_node_with(centre, EllipseRadii::circle(radius), label, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an elliptical node with a centred label, and returns its id.
    ///
    /// Equivalent to [`add_ellipse_node_with`](Self::add_ellipse_node_with) with [`EllipseOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_ellipse_node_with`](Self::add_ellipse_node_with).
    pub fn add_ellipse_node(
        &self,
        centre: Point,
        radii: EllipseRadii,
        label: impl Into<String>,
    ) -> Result<NodeId, Error> {
        self.add_ellipse_node_with(centre, radii, label, EllipseOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an elliptical node with a centred label, and returns its id.
    ///
    /// An ellipse is a node like any other plain node. It can be made draggable, focused, and joined by connectors. A
    /// circle is the case of equal radii, made with [`EllipseRadii::circle`]. The radii are semi-axes, measured from the
    /// centre, and are not diameters. The ellipse is not rotated.
    ///
    /// # The label
    ///
    /// The label is one line of plain text, drawn at the normal label size and never made smaller. If it, with
    /// `options`' inner margin all round its box, would cross the curved outline, both radii are made larger by the same
    /// factor. A circle therefore stays a circle, and the ratio of the radii is kept. The ellipse is only ever made
    /// larger, so the radii given are its smallest size. The size is worked out here, once, from the label as the
    /// browser measures it. It does not change if the font loads later.
    ///
    /// # Coordinates
    ///
    /// `centre` is the centre of the ellipse, and it stays put when the label makes the ellipse grow. Every other
    /// method that takes or gives a position, [`move_node`](Self::move_node) and [`node_rect`](Self::node_rect) among
    /// them, uses the top-left corner of the rectangle that encloses the ellipse as it finally is. So a caller that
    /// needs the final size, to place or restore a node, reads it from `node_rect`.
    ///
    /// # Connectors
    ///
    /// A straight connector lands where the line between the two centres crosses the outline. An elbow connector leaves
    /// by the nearest of north, east, south and west. See [`ConnectorOptions::with_from_clock`](
    /// crate::scene::ConnectorOptions::with_from_clock) for putting an end at a chosen minute round the ellipse.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEllipseMargin`] if the inner margin is not a finite value `>= 0.0`.
    ///
    /// Returns [`Error::InvalidEllipseGeometry`] if the centre is not finite, if either radius is not finite and greater
    /// than zero, or if the radii the label needs, or the rectangle that encloses them, are not finite.
    ///
    /// Both are checked before anything is drawn or recorded, and a failure while drawing removes what was drawn, so a
    /// rejected call leaves the scene exactly as it was.
    pub fn add_ellipse_node_with(
        &self,
        centre: Point,
        radii: EllipseRadii,
        label: impl Into<String>,
        options: EllipseOptions,
    ) -> Result<NodeId, Error> {
        let margin = options.inner_margin;
        if !margin.is_finite() || margin < 0.0 {
            return Err(Error::InvalidEllipseMargin(margin));
        }
        if !centre.x.is_finite() || !centre.y.is_finite() || !radii.is_valid() {
            return Err(Error::InvalidEllipseGeometry {
                centre,
                rx: radii.rx(),
                ry: radii.ry(),
            });
        }

        let label = label.into();
        let mut inner = self.inner.borrow_mut();
        // Taken out for the call, as `add_node_with` does. See `SceneInner::scratch`.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_ellipse(&inner.svg, &mut scratch, centre, radii, &label, options.fill, margin);
        inner.scratch = scratch;
        let drawn = result?;
        inner.attach(&drawn.handles.group)?;
        let id = inner.graph.add_node(drawn.rect, label);
        if let Some(node) = inner.graph.node_mut(id) {
            node.ellipse = Some(drawn.radii);
        }
        inner.insert_node_handle(id, drawn.handles);
        Ok(id)
    }
}
