//! A thin layer over `SvgRoot` for the many small elements a plot is made of. Each is created, styled and appended to
//! its group in one call, and a failure at any step removes everything drawn so far.

use super::super::{GRID_FONT_FAMILY, render_guard::RenderGuard};
use crate::{
    error::Error,
    model::plot::{Side, Spot},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a piece of text is drawn.
pub(super) struct TextStyle<'a> {
    pub(super) size: f64,
    pub(super) fill: &'a str,
    /// The colour of a halo drawn behind the letters, or `None` for no halo.
    pub(super) halo: Option<&'a str>,
    /// Degrees clockwise about the text's own position.
    pub(super) rotation: f64,
}

/// The width of the stroke of a halo, half of which is outside the letters.
const HALO_WIDTH: f64 = 3.5;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) struct Painter<'a> {
    svg: &'a SvgRoot,
    group: SvgNode,
    guard: RenderGuard,
}

impl<'a> Painter<'a> {
    /// A painter that draws into `group`, and removes it, and all it holds, if it is dropped before
    /// [`finish`](Self::finish).
    pub(super) fn new(svg: &'a SvgRoot, group: SvgNode) -> Self {
        Self {
            svg,
            guard: RenderGuard::new(group.clone()),
            group,
        }
    }

    /// Everything went well: the group stays.
    pub(super) fn finish(self) -> SvgNode {
        self.guard.disarm();
        self.group
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Styles `node`, which is already in the document, and puts it in the group. It is removed if anything fails.
    fn place(
        &mut self,
        node: &SvgNode,
        style: impl FnOnce(&SvgNode) -> Result<(), svg_dom::Error>,
    ) -> Result<(), Error> {
        self.guard.track(node.clone());
        style(node)?;
        self.group.append(node)?;
        self.guard.release();
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(super) fn rect(
        &mut self,
        origin: Point,
        size: Size,
        fill: &str,
        stroke: &str,
        width: f64,
    ) -> Result<SvgNode, Error> {
        let node = self.svg.rect(origin, size)?;
        self.place(&node, |n| {
            n.set_fill(fill)?;
            n.set_stroke(stroke)?;
            n.set_stroke_width(width)
        })?;
        Ok(node)
    }

    pub(super) fn line(
        &mut self,
        from: (f64, f64),
        to: (f64, f64),
        stroke: &str,
        width: f64,
        dash: Option<&str>,
    ) -> Result<(), Error> {
        let node = self.svg.line(Point::new(from.0, from.1), Point::new(to.0, to.1))?;
        self.place(&node, |n| {
            n.set_stroke(stroke)?;
            n.set_stroke_width(width)?;
            dash.map_or(Ok(()), |d| n.set_attr("stroke-dasharray", d))
        })
    }

    pub(super) fn polyline(
        &mut self,
        points: &[(f64, f64)],
        stroke: &str,
        width: f64,
        dash: Option<&str>,
    ) -> Result<(), Error> {
        let points: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        let node = self.svg.polyline(&points)?;
        self.place(&node, |n| {
            n.set_fill("none")?;
            n.set_stroke(stroke)?;
            n.set_stroke_width(width)?;
            n.set_attr("stroke-linejoin", "round")?;
            dash.map_or(Ok(()), |d| n.set_attr("stroke-dasharray", d))
        })
    }

    pub(super) fn path(&mut self, d: &str, stroke: &str, width: f64) -> Result<(), Error> {
        let node = self.svg.path(d)?;
        self.place(&node, |n| {
            n.set_fill("none")?;
            n.set_stroke(stroke)?;
            n.set_stroke_width(width)
        })
    }

    pub(super) fn dot(&mut self, at: (f64, f64), radius: f64, fill: &str, outline: &str) -> Result<(), Error> {
        let node = self.svg.circle(Point::new(at.0, at.1), radius)?;
        self.place(&node, |n| {
            n.set_fill(fill)?;
            n.set_stroke(outline)?;
            n.set_stroke_width(1.5)
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `text` at `spot`. A halo is painted first, so that the letters stay legible over a line.
    pub(super) fn text(&mut self, spot: Spot, text: &str, style: &TextStyle<'_>) -> Result<(), Error> {
        let (x, y) = spot.at;
        let node = self.svg.text(Point::new(x, y), text)?;
        self.place(&node, |n| {
            n.set_text_anchor(match spot.anchor {
                Side::Start => TextAnchor::Start,
                Side::Middle => TextAnchor::Middle,
                Side::End => TextAnchor::End,
            })?;
            n.set_dominant_baseline(DominantBaseline::Middle)?;
            n.set_font_family(GRID_FONT_FAMILY)?;
            n.set_font_size(style.size)?;
            n.set_fill(style.fill)?;
            if let Some(halo) = style.halo {
                n.set_stroke(halo)?;
                n.set_stroke_width(HALO_WIDTH)?;
                n.set_attr("paint-order", "stroke")?;
                n.set_attr("stroke-linejoin", "round")?;
            }
            if style.rotation != 0.0 {
                n.set_attr("transform", &format!("rotate({:.2} {x:.2} {y:.2})", style.rotation))?;
            }
            Ok(())
        })
    }
}
