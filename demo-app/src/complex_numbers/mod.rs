//! `panel-complex` / `#complex-diagram`: complex numbers drawn as Argand diagrams, with their values written beside them,
//! and the same numbers written in every way a [`ComplexNumberFormat`] can write them. See [`build_complex_demo`]'s own
//! doc comment for what each part shows.

mod argand;

use argand::{Argand, Guides, Mark, draw, rectangular_text, trimmed};
use demo_common::util::{ensure_svg_in, resize_svg, stringify};
use std::cell::RefCell;
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    complex::Complex,
    scene::{
        AngleUnit, ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Side,
        ToolbarOptions,
    },
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The file behind this demo: the Argand diagram, which is all arithmetic and drawing.
pub(crate) const FILES: &[crate::SourceFile] = &[crate::source_file!("complex_numbers/argand.rs", "argand.rs")];

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

const STAGE_ID: &str = "complex-diagram-stage";
const DIAGRAM_ID: &str = "complex-diagram";

/// The space left of the first thing and above the first heading, and between one thing and the next.
const MARGIN: f64 = 20.0;
const GAP: f64 = 30.0;
const V_GAP: f64 = 40.0;
/// Room east of the widest line for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;
/// The height of a heading, and the space between it and what it heads.
const HEADING_ROOM: f64 = 34.0;
/// The space between a diagram and the node under it, and between one node and the next.
const NODE_GAP: f64 = 14.0;

/// The width of the diagrams, one in each quadrant, and the larger ones that sit beside nodes.
const QUADRANT_SIDE: f64 = 290.0;
const FEATURE_SIDE: f64 = 400.0;

/// One number in each quadrant of the plane, counterclockwise from the first. The moduli differ, and so do the angles,
/// so that no two diagrams look alike.
const QUADRANTS: [Complex; 4] = [
    Complex::new(3.0, 2.0),
    Complex::new(-2.0, 3.0),
    Complex::new(-3.0, -2.0),
    Complex::new(2.0, -3.0),
];

/// The number that is written six ways. Its modulus is exactly 5, a 3-4-5 triangle, so every form has digits that are
/// easy to check by eye.
const FEATURE: Complex = Complex::new(3.0, 4.0);

/// How many roots of unity the third diagram shows.
const ROOTS: usize = 8;

/// The forms the feature number is written in, in the order they are placed: down the first column and then down the
/// second. Each is named in its node's label.
const FORMS: [(&str, ComplexNumberFormat, AngleUnit); 7] = [
    ("Rectangular", ComplexNumberFormat::Rectangular, AngleUnit::Degrees),
    ("Polar", ComplexNumberFormat::Polar, AngleUnit::Degrees),
    ("Trigonometric", ComplexNumberFormat::Trigonometric, AngleUnit::Degrees),
    ("Polar, in radians", ComplexNumberFormat::Polar, AngleUnit::Radians),
    ("Exponential", ComplexNumberFormat::Exponential, AngleUnit::Degrees),
    ("Log-polar", ComplexNumberFormat::LogPolar, AngleUnit::Degrees),
    ("Matrix", ComplexNumberFormat::Matrix, AngleUnit::Degrees),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many lines of text high the pictures of the single numbers are, and of the eight roots of unity, which are many
/// cells in one node and so are drawn smaller. The default is a little over one.
const PICTURE_SCALE: f64 = 2.0;
const ROOTS_PICTURE_SCALE: f64 = 1.5;

/// `values` written as `format` says, in `angle`, with three decimal places and pictures `scale` lines of text high.
fn content(values: Vec<Complex>, format: ComplexNumberFormat, angle: AngleUnit, scale: f64) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::Complex64(values),
        DataFormat::ComplexNumber(format, ComplexStyle::default().with_angle(angle).with_icon_scale(scale)),
    )
}

/// The `n` complex roots of unity, counterclockwise from 1.
fn roots_of_unity(n: usize) -> Result<Vec<Complex>, String> {
    Complex::unit_roots(n, n).map_err(stringify)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is described to assistive technology for a diagram of `z`.
fn describe(z: Complex) -> String {
    format!(
        "Argand diagram of {}: real part {}, imaginary part {}, modulus {}, angle {} degrees",
        rectangular_text(z, 2),
        trimmed(z.re, 2),
        trimmed(z.im, 2),
        trimmed(z.norm(), 2),
        trimmed(z.arg().to_degrees(), 2),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds a heading to the scene's content group at `top_left`.
fn heading(svg: &SvgRoot, content_group: &web_sys::Element, top_left: Point, text: &str) -> Result<(), String> {
    let at = Point::new(top_left.x, top_left.y + HEADING_ROOM / 2.0);
    let el = svg.text(at, text).map_err(stringify)?;
    el.set_dominant_baseline(DominantBaseline::Middle).map_err(stringify)?;
    el.set_text_anchor(TextAnchor::Start).map_err(stringify)?;
    el.set_font_size(18.0).map_err(stringify)?;
    el.set_attr("font-weight", "600").map_err(stringify)?;
    el.set_fill("#1a202c").map_err(stringify)?;
    el.set_attr("role", "heading").map_err(stringify)?;
    el.set_attr("aria-level", "3").map_err(stringify)?;
    append(content_group, &el)
}

/// Draws `diagram` into the scene's content group.
fn place(svg: &SvgRoot, content_group: &web_sys::Element, diagram: &Argand) -> Result<(), String> {
    let group: SvgNode = draw(svg, diagram)?;
    append(content_group, &group)
}

fn append(content_group: &web_sys::Element, node: &SvgNode) -> Result<(), String> {
    content_group
        .append_child(node.as_element())
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// A named node holding `values` in `format`, at `top_left`, and the size it came out.
fn add_node(
    scene: &Scene,
    top_left: Point,
    name: &str,
    content: DataNodeContent,
) -> Result<(svg_dom_graph::NodeId, Size), String> {
    let id = scene.add_named_data_node(top_left, name, content).map_err(stringify)?;
    let rect = scene.node_rect(id).map_err(stringify)?;
    Ok((id, rect.size))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the complex number demo: three Argand diagrams, each with the numbers that go with it.
///
/// - **One number in each quadrant.** Four diagrams, each with a number's modulus as a solid line from the origin and
///   its angle as an arc from the real axis, and both written on the diagram: `r` and `θ`. The dot is labelled with the
///   number in rectangular form. Under each is a node with the same number in polar form, so that the labels on the
///   line and the arc and the digits in the node can be read against one another. The angle of a number in the third
///   and fourth quadrants is negative: it is measured clockwise.
/// - **One number, seven ways.** `3 + 4i`, whose modulus is exactly 5, drawn large, with its real part and imaginary
///   part dashed to the axes as well as its modulus and angle, and a node for each way of writing
///   it: rectangular, polar in degrees and in radians, trigonometric, exponential, log-polar and as the real 2×2
///   matrix that multiplies in the same way. The matrix is the one node whose cell is two rows high.
/// - **The eighth roots of unity.** The points of the unit circle that, raised to the eighth power, give 1. They are
///   the twiddle factors of an eight-point Fourier transform. Each is a turn of an eighth, 45°, further round than the
///   one before, so the moduli of all eight in the polar node are exactly 1 and the angles go up in steps of 45°. The
///   rectangular node shows the same eight numbers as the two legs of each right triangle.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#complex-diagram-stage`.
pub(crate) fn build_complex_demo() -> Result<(), String> {
    let document = demo_common::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1400.0, 1800.0))?;
    // A second handle on the same `<svg>`, because `Scene::new` takes the first. It creates the diagrams.
    let drawing = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    let content_group = document
        .query_selector(&format!("#{DIAGRAM_ID} .svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("#{DIAGRAM_ID} has no scene content group"))?;

    // One number in each quadrant.
    let mut y = MARGIN;
    let mut right: f64 = 0.0;
    heading(&drawing, &content_group, Point::new(MARGIN, y), "One number in each quadrant")?;
    y += HEADING_ROOM;
    let mut tallest: f64 = 0.0;
    for (i, &z) in QUADRANTS.iter().enumerate() {
        let x = MARGIN + i as f64 * (QUADRANT_SIDE + GAP);
        let diagram = Argand {
            top_left: Point::new(x, y),
            side: QUADRANT_SIDE,
            extent: 4.5,
            unit_circle: false,
            marks: vec![Mark {
                z,
                label: rectangular_text(z, 2),
                guides: Guides::Polar,
            }],
            description: describe(z),
        };
        place(&drawing, &content_group, &diagram)?;
        let (_, size) = add_node(
            &scene,
            Point::new(x, y + QUADRANT_SIDE + NODE_GAP),
            "Polar",
            content(vec![z], ComplexNumberFormat::Polar, AngleUnit::Degrees, PICTURE_SCALE),
        )?;
        tallest = tallest.max(QUADRANT_SIDE + NODE_GAP + size.height);
        right = right.max(x + QUADRANT_SIDE);
    }
    y += tallest + V_GAP;

    // One number, seven ways.
    heading(&drawing, &content_group, Point::new(MARGIN, y), "One number, seven ways")?;
    y += HEADING_ROOM;
    let diagram = Argand {
        top_left: Point::new(MARGIN, y),
        side: FEATURE_SIDE,
        extent: 5.5,
        unit_circle: false,
        marks: vec![Mark {
            z: FEATURE,
            label: rectangular_text(FEATURE, 2),
            guides: Guides::Both,
        }],
        description: describe(FEATURE),
    };
    place(&drawing, &content_group, &diagram)?;

    // Two columns of nodes. The first holds the first four forms and the second the rest, so the second starts as far
    // right as the widest of the first needs.
    let columns_x = MARGIN + FEATURE_SIDE + GAP;
    let (first, second) = FORMS.split_at(4);
    let mut column_x = columns_x;
    let mut bottom: f64 = 0.0;
    for column in [first, second] {
        let mut node_y = y;
        let mut widest: f64 = 0.0;
        for &(name, format, angle) in column {
            let (_, size) = add_node(
                &scene,
                Point::new(column_x, node_y),
                name,
                content(vec![FEATURE], format, angle, PICTURE_SCALE),
            )?;
            widest = widest.max(size.width);
            node_y += size.height + NODE_GAP;
        }
        bottom = bottom.max(node_y - NODE_GAP);
        right = right.max(column_x + widest);
        column_x += widest + GAP;
    }
    y += FEATURE_SIDE.max(bottom - y) + V_GAP;

    // The roots of unity.
    heading(&drawing, &content_group, Point::new(MARGIN, y), "The eighth roots of unity")?;
    y += HEADING_ROOM;
    let roots = roots_of_unity(ROOTS)?;
    let diagram = Argand {
        top_left: Point::new(MARGIN, y),
        side: FEATURE_SIDE,
        extent: 1.45,
        unit_circle: true,
        marks: roots
            .iter()
            .enumerate()
            .map(|(k, &z)| Mark {
                z,
                label: format!("w{k}"),
                guides: Guides::None,
            })
            .collect(),
        description: format!(
            "Argand diagram of the {ROOTS} eighth roots of unity, evenly spaced round the unit circle"
        ),
    };
    place(&drawing, &content_group, &diagram)?;
    let mut node_y = y;
    for (name, format) in [
        ("Rectangular", ComplexNumberFormat::Rectangular),
        ("Polar", ComplexNumberFormat::Polar),
    ] {
        let node = DataNodeContent::new(
            NodeValues::Complex64(roots.clone()),
            DataFormat::ComplexNumber(format, ComplexStyle::default().with_icon_scale(ROOTS_PICTURE_SCALE)),
        )
        .with_layout(GridLayout::Columns(4));
        let (_, size) = add_node(&scene, Point::new(columns_x, node_y), name, node)?;
        right = right.max(columns_x + size.width);
        node_y += size.height + NODE_GAP;
    }
    y += FEATURE_SIDE.max(node_y - NODE_GAP - y);

    resize_svg(&document, DIAGRAM_ID, Size::new(right + MARGIN + TOOLBAR_ROOM, y + MARGIN))?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
