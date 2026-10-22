//! `panel-float` / `#float-diagram`: the same nine floating point values written in every way a [`FloatFormat`] can
//! write them, one node per format, so that the rows line up and each format can be read against the others. See
//! [`build_float_demo`]'s own doc comment for what each node shows.

use demo_common::util::{ensure_svg_in, resize_svg, stringify};
use std::{cell::RefCell, f64::consts::PI};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::scene::{
    DataFormat, DataNodeContent, FloatFormat, GridLayout, LabellingStyle, NodeValues, Scene, Side, ToolbarOptions,
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("floats.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// Image locations
const IMAGE_LOCATIONS: &[&str; 2] = &[
    "img/floating_point_single_precision.svg",
    "img/floating_point_double_precision.svg",
];

/// The values every node shows, one to a row. They are chosen to need every kind of width: a number of a few digits, a
/// tiny negative one, a large one, a sum that has no short decimal form, the two zeros, and the three special values.
const VALUES: [f64; 9] = [
    PI,
    -0.000_123_456,
    1_234_567.891,
    0.1 + 0.2,
    0.0,
    -0.0,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

/// What each row is, written beside the first node of every line.
const ROW_LABELS: &[&str] = &[
    "pi", "tiny negative", "large", "0.1 + 0.2", "zero", "negative zero", "NaN", "infinity", "-infinity",
];

/// The space left of the first node and above the first line, and between one node and the next, and one line and the
/// next.
const MARGIN: f64 = 20.0;
const H_GAP: f64 = 30.0;
const V_GAP: f64 = 36.0;
/// Room east of the widest line for the zoom toolbar.
const TOOLBAR_ROOM: f64 = 80.0;
/// The space between a picture and the node below it.
const IMAGE_GAP: f64 = 12.0;

/// The pictures of how the bits of an `f32` and an `f64` are divided, shown above the binary nodes.
const SINGLE_PRECISION: Image = Image {
    href: IMAGE_LOCATIONS[0],
    width: 960.0,
    height: 195.0,
};
const DOUBLE_PRECISION: Image = Image {
    href: IMAGE_LOCATIONS[1],
    width: 960.0,
    height: 195.0,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node of a line: a `name`, and `values` in `format`, one to a row. The first node of a line is also labelled with
/// what each row is.
struct Column {
    name: &'static str,
    values: Vec<f64>,
    format: DataFormat,
    /// `true` for an `f32`, which is these values rounded to the nearest `f32`.
    single: bool,
    labelled: bool,
    /// A picture shown above this node, as wide as the node.
    image: Option<Image>,
}

/// A picture served from `demo/img/`, with the pixel size it was made at. Only the proportions are used.
#[derive(Clone, Copy)]
struct Image {
    href: &'static str,
    width: f64,
    height: f64,
}

impl Column {
    fn f64(name: &'static str, format: FloatFormat) -> Self {
        Self {
            name,
            values: VALUES.to_vec(),
            format: DataFormat::Float(format),
            single: false,
            labelled: false,
            image: None,
        }
    }

    fn f32(name: &'static str, format: DataFormat) -> Self {
        Self {
            name,
            values: VALUES.to_vec(),
            format,
            single: true,
            labelled: false,
            image: None,
        }
    }

    fn bits(name: &'static str, format: DataFormat, single: bool) -> Self {
        Self {
            name,
            values: VALUES.to_vec(),
            format,
            single,
            labelled: false,
            image: None,
        }
    }

    fn labelled(mut self) -> Self {
        self.labelled = true;
        self
    }

    fn with_image(mut self, image: Image) -> Self {
        self.image = Some(image);
        self
    }

    /// This column's node content.
    fn content(&self) -> DataNodeContent {
        let values = if self.single {
            NodeValues::F32(self.values.iter().map(|&v| v as f32).collect())
        } else {
            NodeValues::F64(self.values.clone())
        };
        let content = DataNodeContent::new(values, self.format).with_layout(GridLayout::Columns(1));
        if self.labelled {
            content.with_labelling_style(LabellingStyle::Text(ROW_LABELS))
        } else {
            content
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `Scene`'s own group, which the zoom and pan transform is applied to. A picture appended there moves with the
/// nodes, where one appended to the `<svg>` itself would stay put.
fn content_group(document: &web_sys::Document) -> Result<web_sys::Element, String> {
    document
        .query_selector("#float-diagram .svg-dom-graph-content")
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| "#float-diagram has no scene content group".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the floating point demo: nine values, and one node for each way of writing them. Each line of nodes is read
/// across, and the rows line up, so a row is one value written every way.
///
/// - **Line 1: significant digits and decimal places.** The first node is the shortest digits that read back as the
///   same value. Then come decimal places, which count the digits after the point whatever the size of the value, and
///   significant digits, which count from the first nonzero digit. So two decimal places keep `0.000123456` as `-0.00`,
///   and three significant digits keep it as `-0.000123`. The digits of a large value that are not significant are
///   written as zeros.
/// - **Line 2: scientific notation,** where the precision counts the digits of the mantissa.
/// - **Line 3: types and bits.** The same values as an `f32`, which has fewer digits to lose, and the IEEE-754 bits of
///   an `f32` and an `f64` in hexadecimal.
/// - **Line 4: the same bits in binary,** where the sign bit, the exponent and the fraction can be told apart.
///
/// Every node is as wide as its widest value needs, and what that value is changes with the format. Under three
/// significant digits it is the tiny one, whose leading zeros make it the widest. Under decimal places it is the large
/// one. Under scientific notation the numbers are much the same width, so the sign decides, and in every format that
/// writes a sign, `-infinity` is wider than a short number.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#float-diagram`.
pub(crate) fn build_float_demo() -> Result<(), String> {
    let document = demo_common::util::document()?;
    ensure_svg_in(&document, "float-diagram-stage", "float-diagram", Size::new(1400.0, 1700.0))?;
    // A second handle on the same `<svg>`, because `Scene::new` takes the first. It creates the pictures.
    let pictures = SvgRoot::attach("float-diagram").map_err(stringify)?;
    let scene = Scene::new(SvgRoot::attach("float-diagram").map_err(stringify)?).map_err(stringify)?;

    let lines = [
        vec![
            Column::f64("Shortest", FloatFormat::shortest()).labelled(),
            Column::f64("2 decimal places", FloatFormat::decimal_places(2)),
            Column::f64("6 decimal places", FloatFormat::decimal_places(6)),
            Column::f64("3 significant digits", FloatFormat::significant_digits(3)),
            Column::f64("8 significant digits", FloatFormat::significant_digits(8)),
        ],
        vec![
            Column::f64("Scientific, shortest", FloatFormat::shortest().scientific()).labelled(),
            Column::f64("Scientific, 3 decimal places", FloatFormat::decimal_places(3).scientific()),
            Column::f64(
                "Scientific, 4 significant digits",
                FloatFormat::significant_digits(4).scientific(),
            ),
        ],
        vec![
            Column::f32("f32, shortest", DataFormat::Decimal).labelled(),
            Column::f32(
                "f32, 3 significant digits",
                DataFormat::Float(FloatFormat::significant_digits(3)),
            ),
            Column::bits("f32 bits (hexadecimal)", DataFormat::Hexadecimal, true),
            Column::bits("f64 bits (hexadecimal)", DataFormat::Hexadecimal, false),
        ],
        vec![
            Column::bits("f32 bits (binary)", DataFormat::Binary, true)
                .labelled()
                .with_image(SINGLE_PRECISION),
            Column::bits("f64 bits (binary)", DataFormat::Binary, false).with_image(DOUBLE_PRECISION),
        ],
    ];

    let content = content_group(&document)?;
    let mut y = MARGIN;
    let mut right: f64 = 0.0;
    for line in &lines {
        let mut x = MARGIN;
        let mut tallest: f64 = 0.0;
        let mut placed = Vec::new();
        for column in line {
            let node = scene
                .add_named_data_node(Point::new(x, y), column.name, column.content())
                .map_err(stringify)?;
            let rect: Rect = scene.node_rect(node).map_err(stringify)?;
            placed.push((node, column.image, x, rect.size.width));
            x += rect.size.width + H_GAP;
            tallest = tallest.max(rect.size.height);
        }

        // A picture is as wide as its node, so its height is known only now that the node's width is. The nodes of this
        // line are then moved down to make room for the tallest picture above them.
        let picture_room = placed
            .iter()
            .filter_map(|&(_, image, _, width)| image.map(|i| width * i.height / i.width))
            .fold(0.0, f64::max);
        if picture_room > 0.0 {
            for &(node, image, x, width) in &placed {
                scene
                    .move_node(node, Point::new(x, y + picture_room + IMAGE_GAP))
                    .map_err(stringify)?;
                if let Some(image) = image {
                    let size = Size::new(width, width * image.height / image.width);
                    // Bottom-aligned, so that every picture sits directly on its own node.
                    let top = y + picture_room - size.height;
                    let node = pictures.image(image.href, Point::new(x, top), size).map_err(stringify)?;
                    content.append_child(node.as_element()).map_err(|e| format!("{e:?}"))?;
                }
            }
            tallest += picture_room + IMAGE_GAP;
        }
        right = right.max(x - H_GAP);
        y += tallest + V_GAP;
    }

    resize_svg(
        &document,
        "float-diagram",
        Size::new(right + MARGIN + TOOLBAR_ROOM, y - V_GAP + MARGIN),
    )?;
    scene.refresh_layout().map_err(stringify)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
