//! "One number, seven ways": an Argand diagram of one number with a slider along each axis, and the number written in
//! every form beside it.
//!
//! The slider under the diagram sets the real part and the slider beside it sets the imaginary part. Each is as long as
//! the axis it lies along, and its thumb is under the point of the axis whose value it stands for. When either moves, the
//! diagram is drawn again for the new number, and every node is given the new number with `Scene::set_data_values`, which
//! rewrites the text of each cell, and its picture and its exponent, and nothing else.
//!
//! A node is as wide as it was drawn, and `set_data_values` never resizes it. So each node is drawn for whichever of
//! some extreme numbers makes its form widest, measured in the page, and is then given the number it is to show. A number
//! the sliders reach cannot then be wider than its cell.

use super::{
    FEATURE_SIDE, GAP, MARGIN, NODE_GAP, PICTURE_SCALE, V_GAP,
    argand::{Argand, Guides, Mark, draw, rectangular_text},
    describe, sliders,
    sliders::{Orientation, Spec},
};
use demo_common::util::{clear_error, report_error, stringify};
use std::cell::RefCell;
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    complex::Complex,
    scene::{AngleUnit, ComplexNumberFormat, DataNodeContent, NodeValues, Scene},
};
use web_sys::HtmlInputElement;

/// What each slider reaches: the axes, and a slider beside one, run from minus this to this.
pub(super) const REACH: f64 = 5.0;

/// How the plane is shown: a little past the axes, for their arrowheads and names.
const EXTENT: f64 = 5.5;

/// How finely a slider moves, and what a value is rounded to.
const STEP: f64 = 0.1;

/// The number the sliders start at. Its modulus is exactly 5.
const START: Complex = Complex::new(3.0, 4.0);

/// The id of the paragraph that shows an error from a slider.
const ERROR_ID: &str = "complex-error";

/// The room to the left of the diagram for the slider beside it and its caption, and below it for the slider under it
/// and its caption.
const LEFT_BAND: f64 = 70.0;
const BOTTOM_BAND: f64 = 70.0;

/// How far a slider is from the diagram it belongs to.
const SLIDER_OFFSET: f64 = 22.0;

/// The values a node is drawn for, to find the widest it will have to be: the extremes a slider reaches, a very small
/// number whose logarithm is long, and zero.
const CANDIDATES: [f64; 5] = [-REACH, -STEP, 0.0, STEP, REACH];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which part of the number a slider sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Real,
    Imaginary,
}

/// `z` with `part` set to `value`.
fn with_part(z: Complex, part: Part, value: f64) -> Complex {
    match part {
        Part::Real => Complex::new(value, z.im),
        Part::Imaginary => Complex::new(z.re, value),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the sliders change.
struct Explorer {
    drawing: SvgRoot,
    content_group: web_sys::Element,
    /// The diagram without its number.
    template: Argand,
    /// The diagram as drawn now, so that it can be replaced.
    diagram: Option<SvgNode>,
    z: Complex,
    scene: Scene,
    nodes: Vec<NodeId>,
}

thread_local! {
    // The one explorer on the page, kept for as long as the page is, as every demo keeps its own `Scene`.
    static EXPLORER: RefCell<Option<Explorer>> = const { RefCell::new(None) };
}

impl Explorer {
    /// Draws the diagram for the current number, in place of the one before.
    fn redraw(&mut self) -> Result<(), String> {
        let z = self.z;
        let mut diagram = self.template.clone();
        diagram.marks = vec![Mark {
            z,
            label: rectangular_text(z, 2),
            guides: Guides::Both,
        }];
        diagram.description = describe(z);
        let group = draw(&self.drawing, &diagram)?;
        if let Some(old) = self.diagram.take() {
            old.remove();
        }
        self.content_group
            .append_child(group.as_element())
            .map_err(|e| format!("{e:?}"))?;
        self.diagram = Some(group);
        Ok(())
    }

    /// Sets `part` of the number to `value`, and shows the number everywhere.
    fn set(&mut self, part: Part, value: f64) -> Result<(), String> {
        self.z = with_part(self.z, part, value);
        self.redraw()?;
        for &node in &self.nodes {
            self.scene
                .set_data_values(node, NodeValues::Complex64(vec![self.z]))
                .map_err(stringify)?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What happens when a slider moves: the slider's value is rounded to a step, and shown, and the number changes.
fn moved(part: Part, input: &HtmlInputElement) {
    let value = sliders::snapped(input.value_as_number(), STEP);
    // The thumb goes to the rounded value, so that what it shows is what the number is.
    input.set_value(&value.to_string());
    let _ = input.set_attribute("aria-valuetext", &format!("{value:.1}"));
    let outcome = EXPLORER.with_borrow_mut(|slot| match slot {
        Some(explorer) => explorer.set(part, value),
        None => Ok(()),
    });
    match outcome {
        Ok(()) => clear_error(ERROR_ID),
        Err(message) => report_error(ERROR_ID, &message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A short caption at `at`, turned `turn` degrees about `at` if it is not `0.0`.
fn caption(svg: &SvgRoot, parent: &web_sys::Element, at: Point, text: &str, turn: f64) -> Result<(), String> {
    let el = svg.text(at, text).map_err(stringify)?;
    el.set_text_anchor(TextAnchor::Middle).map_err(stringify)?;
    el.set_dominant_baseline(DominantBaseline::Middle).map_err(stringify)?;
    el.set_font_size(13.0).map_err(stringify)?;
    el.set_fill("#4a5568").map_err(stringify)?;
    el.set_attr("aria-hidden", "true").map_err(stringify)?;
    if turn != 0.0 {
        el.set_attr("transform", &format!("rotate({turn} {} {})", at.x, at.y))
            .map_err(stringify)?;
    }
    parent.append_child(el.as_element()).map(|_| ()).map_err(|e| format!("{e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `values` of one number in the form `format` and `angle`.
fn node_content(z: Complex, format: ComplexNumberFormat, angle: AngleUnit) -> DataNodeContent {
    super::content(vec![z], format, angle, PICTURE_SCALE)
}

/// The number, of the ones a slider can reach, that makes the node called `name` widest in `format`. The width of a form
/// follows the signs, the digits, and the modulus and angle worked out from the number, and not any one of them, so each
/// candidate is measured in the page.
fn widest(scene: &Scene, name: &str, format: ComplexNumberFormat, angle: AngleUnit) -> Result<Complex, String> {
    let mut best: Option<(f64, Complex)> = None;
    for re in CANDIDATES {
        for im in CANDIDATES {
            let z = Complex::new(re, im);
            let size = scene
                .measure_named_data_node(name, &node_content(z, format, angle))
                .map_err(stringify)?;
            if best.is_none_or(|(width, _)| size.width > width) {
                best = Some((size.width, z));
            }
        }
    }
    best.map(|(_, z)| z).ok_or_else(|| "there are no numbers to measure".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the explorer into `scene`, whose elements are drawn with `drawing`, starting at `top`, and returns how far
/// right and how far down it reaches.
///
/// # Errors
///
/// Returns `Err` if any library call fails.
pub(super) fn build(scene: Scene, drawing: SvgRoot, content_group: web_sys::Element, top: f64) -> Result<Size, String> {
    let template = Argand {
        top_left: Point::new(MARGIN + LEFT_BAND, top),
        side: FEATURE_SIDE,
        extent: EXTENT,
        axis_half: REACH,
        unit_circle: false,
        marks: Vec::new(),
        description: String::new(),
    };
    let plane = template.plane();
    let axis_length = template.axis_length();

    // The slider under the diagram sets the real part. It is as long as the real axis, which it is centred under.
    let below = Point::new(plane.centre.x, template.top_left.y + FEATURE_SIDE + SLIDER_OFFSET);
    sliders::add(
        &drawing,
        &content_group,
        sliders::frame(below, axis_length, Orientation::Horizontal),
        &Spec {
            label: "Real part",
            class: "argand-slider-real",
            min: -REACH,
            max: REACH,
            step: STEP,
            value: START.re,
        },
        |input| moved(Part::Real, input),
    )?;
    caption(
        &drawing,
        &content_group,
        Point::new(below.x, below.y + 30.0),
        "Real part a",
        0.0,
    )?;

    // The slider beside the diagram sets the imaginary part, and is as long as the imaginary axis.
    let beside = Point::new(template.top_left.x - SLIDER_OFFSET, plane.centre.y);
    sliders::add(
        &drawing,
        &content_group,
        sliders::frame(beside, axis_length, Orientation::Vertical),
        &Spec {
            label: "Imaginary part",
            class: "argand-slider-imaginary",
            min: -REACH,
            max: REACH,
            step: STEP,
            value: START.im,
        },
        |input| moved(Part::Imaginary, input),
    )?;
    caption(
        &drawing,
        &content_group,
        Point::new(beside.x - 30.0, beside.y),
        "Imaginary part b",
        -90.0,
    )?;

    // The nodes, in two columns, the first four forms and then the rest. Each is drawn for the number that makes its form
    // widest, and then given the number the sliders start at.
    let nodes_x = template.top_left.x + FEATURE_SIDE + GAP;
    let (first, second) = super::FORMS.split_at(4);
    let (mut column_x, mut right, mut bottom) = (nodes_x, nodes_x, top);
    let mut nodes = Vec::with_capacity(super::FORMS.len());
    for column in [first, second] {
        let (mut node_y, mut widest_node): (f64, f64) = (top, 0.0);
        for &(name, format, angle) in column {
            let wide = widest(&scene, name, format, angle)?;
            let (node, size) =
                super::add_node(&scene, Point::new(column_x, node_y), name, node_content(wide, format, angle))?;
            scene
                .set_data_values(node, NodeValues::Complex64(vec![START]))
                .map_err(stringify)?;
            nodes.push(node);
            widest_node = widest_node.max(size.width);
            node_y += size.height + NODE_GAP;
        }
        bottom = bottom.max(node_y - NODE_GAP);
        right = right.max(column_x + widest_node);
        column_x += widest_node + GAP;
    }

    let mut explorer = Explorer {
        drawing,
        content_group,
        template,
        diagram: None,
        z: START,
        scene,
        nodes,
    };
    explorer.redraw()?;
    EXPLORER.with_borrow_mut(|slot| *slot = Some(explorer));

    Ok(Size::new(right, bottom.max(top + FEATURE_SIDE + BOTTOM_BAND).max(top + V_GAP)))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn a_slider_sets_only_its_own_part() {
        let z = Complex::new(3.0, 4.0);

        assert_eq!(with_part(z, Part::Real, -1.5), Complex::new(-1.5, 4.0));
        assert_eq!(with_part(z, Part::Imaginary, 2.0), Complex::new(3.0, 2.0));
    }

    #[test]
    fn the_sliders_start_at_a_number_of_modulus_five_that_they_can_reach() {
        assert!((START.norm() - 5.0).abs() < 1e-12);
        assert!(START.re.abs() <= REACH && START.im.abs() <= REACH);
    }

    #[test]
    fn the_candidates_include_the_extremes_zero_and_the_smallest_step() {
        assert!(CANDIDATES.contains(&-REACH) && CANDIDATES.contains(&REACH));
        assert!(CANDIDATES.contains(&0.0));
        assert!(CANDIDATES.contains(&STEP) && CANDIDATES.contains(&-STEP));
        // They are numbers a slider can reach: whole steps.
        for candidate in CANDIDATES {
            let steps = candidate / STEP;
            assert!(
                (steps - steps.round()).abs() < 1e-9,
                "{candidate} is not a whole number of steps"
            );
        }
    }

    #[test]
    fn the_axes_are_shorter_than_the_plane_so_that_there_is_room_for_the_arrowheads() {
        const { assert!(REACH < EXTENT) };
        // Room for the arrowhead, which reaches a few units past the axis, and its name.
        let scale = FEATURE_SIDE / (2.0 * EXTENT);
        assert!((EXTENT - REACH) * scale > 12.0);
    }

    #[test]
    fn a_slider_is_as_long_as_its_axis() {
        let diagram = Argand {
            top_left: Point::new(0.0, 0.0),
            side: FEATURE_SIDE,
            extent: EXTENT,
            axis_half: REACH,
            unit_circle: false,
            marks: Vec::new(),
            description: String::new(),
        };
        let axis = diagram.axis_length();
        let frame = sliders::frame(Point::new(0.0, 0.0), axis, Orientation::Horizontal);

        // The thumb's centre runs the length of the axis, which is REACH either side of the origin at the plane's scale.
        let travel = frame.size.width - sliders::THUMB;
        assert!((travel - 2.0 * REACH * diagram.plane().scale).abs() < 1e-9);
    }
}
