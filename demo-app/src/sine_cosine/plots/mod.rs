//! What the three plots of the sine and cosine demo hold: how each is configured, and what is drawn for an angle. Pure:
//! the page draws it with plot nodes of the library, and nothing here touches the DOM.
//!
//! Point C goes round the unit circle. Its horizontal position, W, is the cosine of the angle, and its height, H, is
//! the sine. The circle plot shows C itself. The sine plot lays H out along the angle, and the cosine plot lays W out
//! down the angle.
//!
//! The three plots are made to share their scales. The circle and the sine plot are the same height and show the same
//! range of values, so a height is the same distance up in both. The circle and the cosine plot are the same width and
//! show the same range, so a width is the same distance across in both. For that, no plot has a margin along the axis
//! that it shares. The two plots of the angle have a margin at the end of that axis, so that the label of 360° fits.

use svg_dom::root::utils::Size;
use svg_dom_graph::scene::{
    AxisPosition, Insets, LabelAnchor, PlotAxis, PlotConfig, PlotItem, PlotLabel, PlotLine, PlotPoint, PlotPoints,
    TickLabelPlacement, TickLabels,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The side of the circle plot, in pixels.
pub(super) const SIDE: f64 = 400.0;

/// The length of the sine and the cosine plots: the axis of the angle, and the margin at the end of it. The axis of the
/// angle is as long as the side of the circle, since a turn of 360° in steps of 30° is 12 steps, and the side of the
/// circle is 2.4 units in steps of 0.2, which is also 12. So 30° is the same number of pixels as 0.2 units, and the
/// sine and the cosine are drawn to the same scale as the circle in both directions.
pub(super) const LENGTH: f64 = SIDE + END_MARGIN;

/// How far each axis of the circle runs either way from the centre: the radius of 1, and a margin of 0.2 around it.
const SPAN: f64 = 1.2;

/// The distance between ticks on the axes that show a size, and between the ticks that show an angle, in degrees.
const STEP: f64 = 0.2;
const ANGLE_STEP: f64 = 30.0;

/// The margin at the end of an angle axis, which holds the label of the last tick. It is the same in the sine and the
/// cosine, so that a degree is the same distance in both.
pub(super) const END_MARGIN: f64 = 30.0;

/// How many degrees there are in a turn.
pub(super) const TURN: f64 = 360.0;

/// How fast C goes round, in degrees for each second. A turn takes twelve seconds.
pub(super) const DEGREES_PER_SECOND: f64 = 30.0;

/// The colours of the parts: the circle, C and its radius, the width W and the height H.
const CIRCLE: &str = "#64748b";
const POINT: &str = "#d63c32";
const WIDTH: &str = "#1f7a37";
const HEIGHT: &str = "#2563eb";

/// The widths of the lines, and the dash pattern of the dotted ones.
const OUTLINE_WIDTH: f64 = 1.6;
const TRACE_WIDTH: f64 = 2.6;
const DOTTED: &str = "1 5";

/// The radii of the dot for C, and of the dots where a dotted line meets an axis.
const POINT_RADIUS: f64 = 6.0;
const FOOT_RADIUS: f64 = 3.5;

/// The size of the letters that name C, W and H, and how far a name stands from its dot, in pixels.
const NAME_SIZE: f64 = 15.0;
const NAME_GAP: f64 = 15.0;

/// The size of the letters of a plot's title, the gap between its lines, and its ink, in pixels.
const TITLE_SIZE: f64 = 14.0;
const TITLE_GAP: f64 = 18.0;
const TITLE: &str = "#1a202c";

/// The lines of the title of the sine plot, and of the cosine plot.
pub(super) const SINE_TITLE: [&str; 1] = ["Sine: the height H of C against the angle"];
pub(super) const COSINE_TITLE: [&str; 3] = ["Cosine:", "the width W of C", "against the angle"];

/// Where the title of the sine plot is: its left end is at this angle, and its middle is at this height. The curve is
/// never below a height of -1, so the strip between -1 and the bottom of the plot is empty, and the title is in it.
pub(super) const SINE_TITLE_AT: (f64, f64) = (0.0, -1.1);

/// Where the title of the cosine plot is: its left end is at this width, and its first line is at this angle. The curve
/// is to the right of the vertical axis for the first quarter of the turn, and C's dotted line with it, so the title is
/// to the left of the axis, below the labels of the horizontal axis, which are along the top.
pub(super) const COSINE_TITLE_AT: (f64, f64) = (-SPAN, 32.0);

/// How far a press of Page Up or Page Down moves C, in degrees.
const BIG_STEP: f64 = 15.0;

/// No margin at all, so that the plotted area is the whole plot.
const FLUSH: Insets = Insets::new(0.0, 0.0, 0.0, 0.0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ticks of an axis that shows a size: one every [`STEP`], and a label on each but the one at zero, where the axes
/// cross. The labels stop at 1, the radius, since the end of the axis at [`SPAN`] is only the margin round the circle.
fn size_ticks() -> TickLabels {
    let last = (1.0 / STEP).round() as i32;
    TickLabels::Custom(
        (-last..=last)
            .filter(|&n| n != 0)
            .map(|n| {
                let value = f64::from(n) * STEP;
                let sign = if n < 0 { "\u{2212}" } else { "" };
                let text = if value.abs() == 1.0 { "1".to_owned() } else { format!("{:.1}", value.abs()) };
                (value, format!("{sign}{text}"))
            })
            .collect(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The axis that shows a size from -[`SPAN`] to [`SPAN`]. It is the vertical axis of the circle and the sine plot, and
/// the horizontal axis of the circle and the cosine plot.
fn size_axis() -> PlotAxis {
    PlotAxis::linear(-SPAN, SPAN).with_tick_labels(size_ticks())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The plot of the circle: both axes show a size, the same distance across as up, and cross in the middle.
pub(super) fn circle_config() -> PlotConfig {
    PlotConfig::new(Size::new(SIDE, SIDE), size_axis(), size_axis())
        .with_label_placement(TickLabelPlacement::Axis)
        .with_insets(FLUSH)
        .with_description(
            "Unit circle with point C going round it anti-clockwise. Dotted lines drop from C to the horizontal axis \
             at its width W, and to the vertical axis at its height H.",
        )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The plot of the sine: the angle runs along the horizontal axis, and the height up the vertical one.
pub(super) fn sine_config() -> PlotConfig {
    let angle = PlotAxis::linear(0.0, TURN)
        .with_major_interval(ANGLE_STEP)
        .with_tick_suffix("\u{00B0}");
    PlotConfig::new(
        Size::new(LENGTH, SIDE),
        angle,
        size_axis().with_position(AxisPosition::AtMinimum),
    )
    .with_label_placement(TickLabelPlacement::Axis)
    .with_insets(Insets::new(0.0, 0.0, END_MARGIN, 0.0))
    .with_description(
        "Graph of the height H of point C against the angle in degrees, which traces a sine wave. A dotted line \
             drops from C to the horizontal axis at H.",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ticks of the angle axis of the cosine plot. The angle runs down, away from the horizontal axis at the top, so it
/// is plotted as a negative value, and each tick is written as the positive angle that it stands for.
fn downward_angle_ticks() -> TickLabels {
    let turns = (TURN / ANGLE_STEP) as u32;
    TickLabels::Custom(
        (0..=turns)
            .map(|n| {
                let angle = f64::from(n) * ANGLE_STEP;
                (-angle, format!("{angle:.0}\u{00B0}"))
            })
            .collect(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The plot of the cosine: the width runs along the horizontal axis, which is at the top, and the angle runs down the
/// vertical axis, which is in the middle.
pub(super) fn cosine_config() -> PlotConfig {
    let width = size_axis().with_position(AxisPosition::AtMaximum);
    let angle = PlotAxis::linear(-TURN, 0.0).with_tick_labels(downward_angle_ticks());
    PlotConfig::new(Size::new(SIDE, LENGTH), width, angle)
        .with_label_placement(TickLabelPlacement::Axis)
        .with_insets(Insets::new(0.0, 0.0, 0.0, END_MARGIN))
        .with_description(
            "Graph of the width W of point C against the angle in degrees, which runs down the page. It traces a \
             cosine wave on its side. A dotted line runs from C to the vertical axis at W.",
        )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The width of C at `degrees`: the horizontal part of where it is on the circle.
pub(super) fn width(degrees: f64) -> f64 {
    degrees.to_radians().cos()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The height of C at `degrees`: the vertical part of where it is on the circle.
pub(super) fn height(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle after C has gone round for `milliseconds` more than it has at `degrees`. It starts again at 0 each turn.
pub(super) fn turned(degrees: f64, milliseconds: f64) -> f64 {
    (degrees + milliseconds / 1000.0 * DEGREES_PER_SECOND).rem_euclid(TURN)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `degrees` as a whole number of degrees from 0 up to but not including 360. A turn is 0 and not 360, so an angle just
/// before a turn does not read as one. The 0 is always a positive zero, since a negative zero is written as "-0".
pub(super) fn whole(degrees: f64) -> f64 {
    // Adding 0.0 turns a negative zero into a positive one, and changes no other number.
    degrees.round().rem_euclid(TURN) + 0.0
}

/// What is written beside the plots for an angle, so that the numbers behind the picture can be read. It is plain text
/// in three lines, which the page shows as three lines, and is never read as markup.
pub(super) fn readout(degrees: f64) -> String {
    let shown = whole(degrees);
    format!(
        "Angle {shown:.0}\u{00B0}\nWidth  W = cos = {:.2}\nHeight H = sin = {:.2}",
        width(degrees),
        height(degrees)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle, in whole degrees from 0 up to but not including 360, of the direction `(dx, dy)` from the centre of the
/// circle, where `dy` is up. `None` at the centre itself, which has no direction.
pub(super) fn angle_of(dx: f64, dy: f64) -> Option<f64> {
    (dx.hypot(dy) > f64::EPSILON).then(|| whole(dy.atan2(dx).to_degrees()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where C goes when `key` is pressed with it at `degrees`, or `None` if the key does nothing. The arrow keys move it
/// by a degree, and Right and Up go anti-clockwise, the way that the angle grows, as the keys of a slider do. Page Up
/// and Page Down move it by 15, and Home and End take it to 0 and to 359. A turn is 0, so the angle wraps in both ways.
pub(super) fn key_angle(key: &str, degrees: f64) -> Option<f64> {
    let whole = whole(degrees);
    let moved = match key {
        "ArrowRight" | "ArrowUp" => whole + 1.0,
        "ArrowLeft" | "ArrowDown" => whole - 1.0,
        "PageUp" => whole + BIG_STEP,
        "PageDown" => whole - BIG_STEP,
        "Home" => 0.0,
        "End" => TURN - 1.0,
        _ => return None,
    };
    Some(moved.rem_euclid(TURN))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of `place` at each whole degree up to `degrees`, and at `degrees` itself. This is the curve that C has
/// drawn so far.
fn trace(degrees: f64, place: impl Fn(f64) -> PlotPoint) -> Vec<PlotPoint> {
    let whole = degrees.floor() as u32;
    let mut points: Vec<PlotPoint> = (0..=whole).map(|d| place(f64::from(d))).collect();
    if degrees > f64::from(whole) {
        points.push(place(degrees));
    }
    points
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A dotted line from `from` to `to` in `colour`.
fn dotted(from: PlotPoint, to: PlotPoint, colour: &str) -> PlotItem {
    PlotLine::new([from, to], colour)
        .with_width(TRACE_WIDTH - 0.6)
        .with_dash(DOTTED)
        .into()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A small dot where a dotted line meets an axis.
fn foot(at: PlotPoint, colour: &str) -> PlotItem {
    PlotPoints::new([at], colour).with_radius(FOOT_RADIUS).into()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The dot for C.
fn point(at: PlotPoint) -> PlotItem {
    PlotPoints::new([at], POINT).with_radius(POINT_RADIUS).into()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name `text` at `at`, moved by `offset` pixels to the right and down. `anchor` says which end of it is at the
/// spot.
fn name(text: &str, at: PlotPoint, colour: &str, anchor: LabelAnchor, offset: (f64, f64)) -> PlotItem {
    PlotLabel::new(at, text)
        .with_colour(colour)
        .with_size(NAME_SIZE)
        .with_anchor(anchor)
        .with_offset(offset.0, offset.1)
        .into()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of C, which stands on the far side of it from `towards`, so it is clear of the lines that meet at C.
fn name_of_c(at: PlotPoint, towards: (f64, f64)) -> PlotItem {
    let (dx, dy) = towards;
    let length = dx.hypot(dy).max(f64::EPSILON);
    // The page counts down, so the vertical part of the direction is turned over.
    let offset = (-dx / length * NAME_GAP, dy / length * NAME_GAP + NAME_SIZE / 3.0);
    name("C", at, POINT, LabelAnchor::Middle, offset)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which way a name beside an axis stands, so that it is on the side that C is not: `-1.0` is the lower or left side.
fn away(value: f64) -> f64 {
    if value >= 0.0 { -1.0 } else { 1.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the circle plot when C is at `degrees`: the circle, the radius to C, and the two dotted lines that
/// drop from C to the axes, where W and H are.
pub(super) fn circle_items(degrees: f64) -> Vec<PlotItem> {
    let (w, h) = (width(degrees), height(degrees));
    let c = PlotPoint::new(w, h);
    let (on_x, on_y) = (PlotPoint::new(w, 0.0), PlotPoint::new(0.0, h));
    let outline = PlotLine::new(
        (0..=TURN as u32).map(|d| PlotPoint::new(width(f64::from(d)), height(f64::from(d)))),
        CIRCLE,
    );

    vec![
        outline.with_width(OUTLINE_WIDTH).into(),
        PlotLine::new([PlotPoint::new(0.0, 0.0), c], POINT)
            .with_width(TRACE_WIDTH)
            .into(),
        dotted(c, on_x, WIDTH),
        dotted(c, on_y, HEIGHT),
        foot(on_x, WIDTH),
        foot(on_y, HEIGHT),
        point(c),
        name(
            "W",
            on_x,
            WIDTH,
            LabelAnchor::Middle,
            (0.0, -away(h) * NAME_GAP + NAME_SIZE / 3.0),
        ),
        name("H", on_y, HEIGHT, LabelAnchor::Middle, (away(w) * NAME_GAP, NAME_SIZE / 3.0)),
        name_of_c(c, (-w, -h)),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The lines of a title, one under the other. The first is at `at`, and each of the others is `drop` below the one
/// before it, in the units of the vertical axis. The left end of every line is at `at`, a little way in.
fn title(lines: &[&str], at: PlotPoint, drop: f64) -> Vec<PlotItem> {
    lines
        .iter()
        .enumerate()
        .map(|(n, line)| {
            let place = PlotPoint::new(at.x, at.y - drop * n as f64);
            PlotLabel::new(place, *line)
                .with_colour(TITLE)
                .with_size(TITLE_SIZE)
                .with_anchor(LabelAnchor::Start)
                .with_offset(NAME_GAP / 2.0, TITLE_SIZE / 3.0)
                .into()
        })
        .collect()
}

/// What is drawn on the sine plot when C is at `degrees`: the curve so far, C at the end of it, and a dotted line from
/// C to the horizontal axis, where H is.
pub(super) fn sine_items(degrees: f64) -> Vec<PlotItem> {
    let h = height(degrees);
    let c = PlotPoint::new(degrees, h);
    let on_axis = PlotPoint::new(degrees, 0.0);

    let mut items = vec![
        PlotLine::new(trace(degrees, |d| PlotPoint::new(d, height(d))), HEIGHT)
            .with_width(TRACE_WIDTH)
            .into(),
        dotted(c, on_axis, HEIGHT),
        foot(on_axis, HEIGHT),
        point(c),
        name(
            "H",
            on_axis,
            HEIGHT,
            LabelAnchor::Middle,
            (0.0, -away(h) * NAME_GAP + NAME_SIZE / 3.0),
        ),
        name("C", c, POINT, LabelAnchor::Start, (NAME_GAP / 2.0, -NAME_GAP / 2.0)),
    ];
    items.extend(title(&SINE_TITLE, PlotPoint::new(SINE_TITLE_AT.0, SINE_TITLE_AT.1), 0.0));
    items
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the cosine plot when C is at `degrees`: the curve so far, C at the end of it, and a dotted line
/// from C to the vertical axis, where W is.
pub(super) fn cosine_items(degrees: f64) -> Vec<PlotItem> {
    let w = width(degrees);
    let c = PlotPoint::new(w, -degrees);
    let on_axis = PlotPoint::new(0.0, -degrees);
    let side = away(w);

    let mut items = vec![
        PlotLine::new(trace(degrees, |d| PlotPoint::new(width(d), -d)), WIDTH)
            .with_width(TRACE_WIDTH)
            .into(),
        dotted(c, on_axis, WIDTH),
        foot(on_axis, WIDTH),
        point(c),
        name("W", on_axis, WIDTH, LabelAnchor::Middle, (side * NAME_GAP, NAME_SIZE / 3.0)),
        name("C", c, POINT, LabelAnchor::Start, (NAME_GAP / 2.0, -NAME_GAP / 2.0)),
    ];
    let line_drop = TITLE_GAP / ((LENGTH - END_MARGIN) / TURN);
    let first = PlotPoint::new(COSINE_TITLE_AT.0, -COSINE_TITLE_AT.1);
    items.extend(title(&COSINE_TITLE, first, line_drop));
    items
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
