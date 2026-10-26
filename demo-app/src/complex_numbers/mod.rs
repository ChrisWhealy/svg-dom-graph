//! `panel-complex` / `#complex-diagram` and `#complex-roots-diagram`: complex numbers drawn as Argand diagrams, with
//! sliders that change the number, and the same number written in every way a [`ComplexNumberFormat`] can write it. See
//! [`build_complex_demo`]'s own doc comment for what each part shows.

mod argand;
mod explorer;
mod roots;
mod sliders;

use argand::{rectangular_text, trimmed};
use demo_common::util::{ensure_svg_in, resize_svg, stringify};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    complex::Complex,
    scene::{
        AngleUnit, ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent, NodeValues, Scene, Side,
        ToolbarOptions,
    },
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the Argand diagram, the sliders, the explorer that they drive, and the roots of unity.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("complex_numbers/argand.rs", "argand.rs"),
    crate::source_file!("complex_numbers/sliders/mod.rs", "sliders/mod.rs"),
    crate::source_file!("complex_numbers/explorer.rs", "explorer.rs"),
    crate::source_file!("complex_numbers/roots.rs", "roots.rs"),
];

const STAGE_ID: &str = "complex-diagram-stage";
const DIAGRAM_ID: &str = "complex-diagram";

/// The space left of the first thing and above the first heading, and between one thing and the next.
const MARGIN: f64 = 20.0;
const GAP: f64 = 30.0;
const V_GAP: f64 = 40.0;
/// Room east of the widest line for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;
/// The space between one node and the next.
const NODE_GAP: f64 = 14.0;

/// The width of the diagrams that sit beside nodes.
const FEATURE_SIDE: f64 = 400.0;

/// How many lines of text high the pictures of the single numbers are. The default is a little over one.
const PICTURE_SCALE: f64 = 2.0;

/// The forms the number is written in, in the order they are placed: down the first column and then down the second.
/// Each is named in its node's label.
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
/// `values` written as `format` says, in `angle`, with three decimal places and pictures `scale` lines of text high.
fn content(values: Vec<Complex>, format: ComplexNumberFormat, angle: AngleUnit, scale: f64) -> DataNodeContent {
    DataNodeContent::new(
        NodeValues::Complex64(values),
        DataFormat::ComplexNumber(format, ComplexStyle::default().with_angle(angle).with_icon_scale(scale)),
    )
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
/// A named node holding `content`, at `top_left`, and the size it came out.
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
/// Builds the complex number demo: an Argand diagram that two sliders change, and the roots of unity.
///
/// - **One number, seven ways.** A diagram of `3 + 4i`, whose modulus is exactly 5, with its real part and imaginary part
///   dashed to the axes, and its modulus and angle. A slider runs along the bottom of the diagram, as long as the real
///   axis, and sets the real part. A second runs up the left side, as long as the imaginary axis, and sets the imaginary
///   part. As either moves, the diagram is drawn again, and each of seven nodes is given the new number: rectangular,
///   polar in degrees and in radians, trigonometric, exponential, log-polar and as the real 2×2 matrix that multiplies in
///   the same way. See [`explorer`].
/// - **The roots of unity.** The points of the unit circle that, raised to the power `n`, give 1. They are the twiddle
///   factors of an `n`-point Fourier transform. A slider above the diagram sets `n` from 1 to 20, and the diagram and the
///   two nodes of the numbers, in rectangular and in polar form, are drawn again for it. See [`roots`].
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing the stages and the slider that this builds into.
pub(crate) fn build_complex_demo() -> Result<(), String> {
    let document = demo_common::util::document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1400.0, 700.0))?;
    // A second handle on the same `<svg>`, because `Scene::new` takes the first. It creates the diagram and the sliders.
    let drawing = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?).map_err(stringify)?;
    let content_group = document
        .query_selector(&format!("#{DIAGRAM_ID} .svg-dom-graph-content"))
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| format!("#{DIAGRAM_ID} has no scene content group"))?;

    let reach = explorer::build(scene.clone(), drawing, content_group, MARGIN)?;
    resize_svg(
        &document,
        DIAGRAM_ID,
        Size::new(reach.width + MARGIN + TOOLBAR_ROOM, reach.height + MARGIN),
    )?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    roots::build()
}
