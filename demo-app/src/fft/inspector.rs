//! The inspector scene: one [`Scene::add_butterfly_node`] that shows the selected butterfly of the overview. It is
//! drawn once for a size and a signal, and every choice of butterfly only gives its nodes new numbers.

use super::{
    algorithm::{FftTrace, PairTrace},
    sizing::{content, only, widest_value},
};
use demo_common::{
    complex::Complex,
    util::{MARGIN, document, required_element, resize_svg, stringify},
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{Butterfly, ButterflyOperands, ButterflyResults, Scene, Side, ToolbarOptions, ViewTransform},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const GAP: f64 = 70.0;
const NODE_GAP: f64 = 24.0;
/// Room east of the outputs for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The inspector scene and the nodes a butterfly's numbers go in.
pub(super) struct Inspector {
    scene: Scene,
    a: NodeId,
    b: NodeId,
    w: NodeId,
    butterfly: Butterfly,
    sum_out: NodeId,
    difference_out: NodeId,
}

impl Inspector {
    /// Draws the inspector for `trace` into `#svg_id`. `origin` is where the butterfly's frame was dragged to in the
    /// scene this one replaces. It is kept, unless the new canvas cannot hold the frame there. Returns the inspector,
    /// and whether the frame had to be moved to stay inside.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the page has no `#svg_id`, or if any library call fails.
    pub(super) fn build(
        svg_id: &str,
        trace: &FftTrace,
        origin: Option<Point>,
        view: Option<ViewTransform>,
    ) -> Result<(Self, bool), String> {
        let document = document()?;
        required_element(&document, svg_id)?;
        let scene = Scene::new(SvgRoot::attach(svg_id).map_err(stringify)?).map_err(stringify)?;

        let all = || trace.pairs.iter().flatten();
        let widest = |name: &str, pick: fn(&PairTrace) -> Complex| widest_value(&scene, name, all().map(pick));
        let (wa, wb, ww) = (widest("a", |p| p.a)?, widest("b", |p| p.b)?, widest("w", |p| p.w)?);
        let (wp, ws, wd) = (
            widest("w\u{00B7}b", |p| p.product)?,
            widest("a + w\u{00B7}b", |p| p.sum)?,
            widest("a \u{2212} w\u{00B7}b", |p| p.difference)?,
        );

        let node =
            |at: Point, name: &str, z: Complex| scene.add_named_data_node(at, name, content(z)).map_err(stringify);
        let height_of = |id: NodeId| -> Result<f64, String> { Ok(scene.node_rect(id).map_err(stringify)?.size.height) };
        let mut y = MARGIN;
        let a = node(Point::new(MARGIN, y), "a", wa)?;
        y += height_of(a)? + NODE_GAP;
        let b = node(Point::new(MARGIN, y), "b", wb)?;
        y += height_of(b)? + NODE_GAP;
        let w = node(Point::new(MARGIN, y), "w", ww)?;
        let inputs_bottom = y + height_of(w)?;
        let inputs_right = [a, b, w].iter().try_fold(0.0_f64, |right, &id| -> Result<f64, String> {
            let rect = scene.node_rect(id).map_err(stringify)?;
            Ok(right.max(rect.origin.x + rect.size.width))
        })?;

        let butterfly = scene
            .add_butterfly_node(
                Point::new(inputs_right + GAP, MARGIN),
                ButterflyOperands::new(a, b, w),
                ButterflyResults::new(content(wp), content(ws), content(wd)),
            )
            .map_err(stringify)?;
        let frame = scene.node_rect(butterfly.frame).map_err(stringify)?;
        let outputs_x = frame.origin.x + frame.size.width + GAP;
        let level = |id: NodeId| -> Result<f64, String> { Ok(scene.node_rect(id).map_err(stringify)?.origin.y) };
        let sum_out = node(Point::new(outputs_x, level(butterfly.sum)?), "a + w\u{00B7}b", ws)?;
        let difference_out = node(Point::new(outputs_x, level(butterfly.difference)?), "a \u{2212} w\u{00B7}b", wd)?;
        scene.add_edge(butterfly.sum, sum_out).map_err(stringify)?;
        scene.add_edge(butterfly.difference, difference_out).map_err(stringify)?;

        let right = [sum_out, difference_out]
            .iter()
            .try_fold(0.0_f64, |right, &id| -> Result<f64, String> {
                let rect = scene.node_rect(id).map_err(stringify)?;
                Ok(right.max(rect.origin.x + rect.size.width))
            })?;
        let bottom = inputs_bottom.max(frame.origin.y + frame.size.height);
        let canvas = Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN);
        resize_svg(&document, svg_id, canvas)?;
        scene.refresh_layout().map_err(stringify)?;

        let mut moved = false;
        if let Some(wanted) = origin {
            let x = wanted.x.clamp(0.0, (canvas.width - frame.size.width).max(0.0));
            let y = wanted.y.clamp(0.0, (canvas.height - frame.size.height).max(0.0));
            moved = (x - wanted.x).abs() > 0.5 || (y - wanted.y).abs() > 0.5;
            scene.move_butterfly(&butterfly, Point::new(x, y)).map_err(stringify)?;
        }
        if let Some(view) = view {
            scene.set_view(view).map_err(stringify)?;
        }
        scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

        Ok((
            Self {
                scene,
                a,
                b,
                w,
                butterfly,
                sum_out,
                difference_out,
            },
            moved,
        ))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The scene, for its view.
    pub(super) const fn scene(&self) -> &Scene {
        &self.scene
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the frame is, by the corner of its rectangle: the point to give [`Self::build`] to keep it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the library cannot report the rectangle.
    pub(super) fn frame_origin(&self) -> Result<Point, String> {
        self.scene
            .node_rect(self.butterfly.frame)
            .map(|rect| rect.origin)
            .map_err(stringify)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives every node the numbers of `pair`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a write fails. Writing the same pair again repairs a partly written one.
    pub(super) fn show(&self, pair: &PairTrace) -> Result<(), String> {
        let set = |id: NodeId, z: Complex| self.scene.set_data_values(id, only(z)).map_err(stringify);
        set(self.a, pair.a)?;
        set(self.b, pair.b)?;
        set(self.w, pair.w)?;
        set(self.butterfly.product, pair.product)?;
        set(self.butterfly.sum, pair.sum)?;
        set(self.butterfly.difference, pair.difference)?;
        set(self.sum_out, pair.sum)?;
        set(self.difference_out, pair.difference)
    }
}
