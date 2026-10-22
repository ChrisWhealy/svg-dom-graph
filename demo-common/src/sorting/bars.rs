//! Drawing a sort as bars: thin nodes standing on a common baseline across the scene, each as tall as the value it
//! stands for. Swapping two elements shows as two bars changing places. A bar's colour says what the algorithm is doing
//! with it.
//!
//! This is the same for every sorting algorithm. The algorithm only says, for each step, the value of each bar and its
//! [`Mark`].

use super::{
    Frame, Mark, TOOLBAR_ROOM,
    array::{HIGHEST, LEN},
    controls,
};
use crate::{
    svg_buttons,
    util::{MARGIN, document, ensure_svg_in, hide_step_driver, required_element, resize_svg, stringify},
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeOptions, NodeValues, Scene, Side, ToolbarOptions},
};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How wide a bar is, how far apart the bars are, and how many pixels tall a bar is for each unit of its value.
const BAR_WIDTH: f64 = 44.0;
const BAR_GAP: f64 = 12.0;
const UNIT: f64 = 2.8;

/// The gap between the row of buttons and the tallest a bar can be, and between the baseline and the note under it.
const CHART_GAP: f64 = 30.0;

/// The narrowest the note is. It is as wide as the bars if they are wider.
const NOTE_WIDTH: f64 = 600.0;

/// The fill of a bar for each [`Mark`]. An ordinary bar is the library's usual light blue.
const PIVOT: &str = "#e5534b";
const EXAMINED: &str = "#4cc86a";
const CANDIDATE: &str = "#5b8def";
const GROUP: &str = "#a8dccf";
const DONE: &str = "#d9c9f3";
const OUTSIDE: &str = "#eef1f5";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How tall the bar for `value` is.
pub(super) fn bar_height(value: u8) -> f64 {
    f64::from(value) * UNIT
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the bar for `value`.
pub(super) fn bar_size(value: u8) -> Size {
    Size::new(BAR_WIDTH, bar_height(value))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The options that give a bar the fill for `mark`.
fn options(mark: Mark) -> NodeOptions {
    let fill = match mark {
        Mark::Plain => None,
        Mark::Pivot => Some(PIVOT),
        Mark::Examined => Some(EXAMINED),
        Mark::Candidate => Some(CANDIDATE),
        Mark::Group => Some(GROUP),
        Mark::Done => Some(DONE),
        Mark::Outside => Some(OUTSIDE),
    };
    NodeOptions::default().with_fill(fill)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was drawn: the scene, and the "Step" node that drives the toolbar.
pub(super) struct Drawn {
    pub(super) scene: Scene,
    pub(super) step: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `frame`, which is step number `position` of `count` of the demo with the ids that start with
/// `prefix`, in the stage, replacing whatever was there. The caller keeps the zoom and pan, and wires the toolbar to
/// the returned "Step" node. If `slide` is set and the frame swapped two bars, they slide from where they were to where
/// they are now, unless the person has asked for less motion.
///
/// From the top: the buttons that choose the array, the bars on their baseline, and a note on the step.
///
/// # Errors
///
/// Returns `Err` if the page is missing a part of the panel, or if any library call fails.
pub(super) fn draw(prefix: &str, frame: &Frame, position: usize, count: usize, slide: bool) -> Result<Drawn, String> {
    let (stage_id, diagram_id) = (format!("{prefix}-diagram-stage"), format!("{prefix}-diagram"));
    let document = document()?;
    ensure_svg_in(&document, &stage_id, &diagram_id, Size::new(1000.0, 1000.0))?;
    required_element(&document, &diagram_id)?.set_inner_html("");
    let svg = SvgRoot::attach(&diagram_id).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let bars_width = LEN as f64 * BAR_WIDTH + (LEN - 1) as f64 * BAR_GAP;
    let width = bars_width.max(NOTE_WIDTH);
    let axis = MARGIN + width / 2.0;
    let buttons_bottom = controls::add_buttons(&document, &diagram_id, axis)?;

    // The bars stand on one baseline, as far down as the tallest a bar can be.
    let baseline = buttons_bottom + CHART_GAP + bar_height(HIGHEST);
    let mut nodes = Vec::with_capacity(frame.values.len() + 1);
    let first = axis - bars_width / 2.0;
    for (index, (&value, &mark)) in frame.values.iter().zip(&frame.marks).enumerate() {
        let size = bar_size(value);
        let left = first + index as f64 * (BAR_WIDTH + BAR_GAP);
        nodes.push(
            scene
                .add_node_with(Point::new(left, baseline - size.height), size, value.to_string(), options(mark))
                .map_err(stringify)?,
        );
    }
    add_baseline(&document, &diagram_id, first - BAR_GAP, first + bars_width + BAR_GAP, baseline)?;
    if let (true, Some((a, b))) = (slide && !prefers_less_motion(), frame.swap) {
        let left = |index: usize| first + index as f64 * (BAR_WIDTH + BAR_GAP);
        let top = |index: usize| baseline - bar_height(frame.values[index]);
        let bars = bar_groups(&document, &diagram_id)?;
        // Each bar has come from the other one's place.
        for (index, other) in [(a, b), (b, a)] {
            if let Some(group) = bars.get(index) {
                slide_from(group, (left(other), top(index)), (left(index), top(index)))?;
            }
        }
    }

    // The note on the step, under the baseline, as wide as the widest thing above it.
    let note_text = format!("### Step {} of {count}\n{}", position + 1, frame.note);
    let note = scene
        .add_markdown_node(Point::new(axis - width / 2.0, baseline + CHART_GAP), width, &note_text)
        .map_err(stringify)?;
    nodes.push(note);

    // The canvas is as tall as the note's bottom needs, with room east for the zoom toolbar.
    let mut bottom = 0.0_f64;
    for &node in &nodes {
        let rect = scene.node_rect(node).map_err(stringify)?;
        bottom = bottom.max(rect.origin.y + rect.size.height);
    }
    resize_svg(
        &document,
        &diagram_id,
        Size::new(MARGIN + width + MARGIN + TOOLBAR_ROOM, bottom + MARGIN),
    )?;
    scene.refresh_layout().map_err(stringify)?;

    let step = add_step_driver(&scene, &diagram_id, count)?;
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    Ok(Drawn { scene, step })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the baseline that the bars stand on, as a line in the scene's own layer, so it zooms and pans with the bars.
/// Assistive technology skips it: it says nothing the bars do not.
fn add_baseline(document: &web_sys::Document, diagram_id: &str, from: f64, to: f64, y: f64) -> Result<(), String> {
    let layer = svg_buttons::layer(document, diagram_id)?;
    let line = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "line")
        .map_err(|e| format!("could not make the baseline: {e:?}"))?;
    for (name, value) in [
        ("x1", from.to_string()),
        ("x2", to.to_string()),
        ("y1", y.to_string()),
        ("y2", y.to_string()),
        ("stroke", "#767676".to_owned()),
        ("stroke-width", "2".to_owned()),
        ("aria-hidden", "true".to_owned()),
    ] {
        line.set_attribute(name, &value)
            .map_err(|e| format!("could not set {name} on the baseline: {e:?}"))?;
    }
    layer
        .append_child(&line)
        .map(|_| ())
        .map_err(|e| format!("could not add the baseline: {e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Step" is a small array of one cell for each step, placed far off-canvas, purely to drive the selection toolbar. It
/// is hidden from assistive technology, since the toolbar's own buttons and the live region announce the position.
fn add_step_driver(scene: &Scene, diagram_id: &str, count: usize) -> Result<NodeId, String> {
    let values: Vec<u16> = (1..=count).map(|n| u16::try_from(n).unwrap_or(u16::MAX)).collect();
    let step = scene
        .add_named_data_node(
            Point::new(-10_000.0, -10_000.0),
            "Step",
            DataNodeContent::new(NodeValues::U16(values), DataFormat::Decimal).with_layout(GridLayout::Rows(1)),
        )
        .map_err(stringify)?;
    hide_step_driver(diagram_id)?;
    Ok(step)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the person has asked their system for less motion. Then a swap is not animated, and the bars are drawn where
/// they have gone.
fn prefers_less_motion() -> bool {
    let Some(window) = web_sys::window() else { return false };
    let query = js_sys::Reflect::get(&window, &"matchMedia".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call1(&window, &"(prefers-reduced-motion: reduce)".into()).ok());
    query
        .and_then(|list| js_sys::Reflect::get(&list, &"matches".into()).ok())
        .and_then(|matches| matches.as_bool())
        .unwrap_or(false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The groups of the bars of the scene drawn in `#diagram_id`, from left to right: the node whose box is a rectangle as
/// wide as a bar, followed by its value. The buttons and the note are not.
fn bar_groups(document: &web_sys::Document, diagram_id: &str) -> Result<Vec<web_sys::Element>, String> {
    let all = document
        .query_selector_all(&format!("#{diagram_id} g"))
        .map_err(|e| format!("could not look for the bars: {e:?}"))?;
    let width = BAR_WIDTH.to_string();
    let mut bars = Vec::with_capacity(LEN);
    for index in 0..all.length() {
        let Some(group) = all.item(index).and_then(|node| node.dyn_into::<web_sys::Element>().ok()) else {
            continue;
        };
        let is_bar = group.first_element_child().is_some_and(|child| {
            child.tag_name().eq_ignore_ascii_case("rect")
                && child.get_attribute("width").as_deref() == Some(width.as_str())
        });
        if is_bar {
            bars.push(group);
        }
    }
    Ok(bars)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How long a bar takes to slide to its new place.
const SLIDE_SECONDS: &str = "0.5s";

/// Slides `group` from the place `from` to the place `to`, which is where its own `transform` already has it, so that
/// the bar ends where it is drawn. The animation is a SMIL `animateTransform` on the bar, which needs no timer of ours:
/// it holds the bar at its last place when it finishes, and goes when the scene is drawn again.
fn slide_from(group: &web_sys::Element, from: (f64, f64), to: (f64, f64)) -> Result<(), String> {
    let document = document()?;
    let animation = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "animateTransform")
        .map_err(|e| format!("could not make the slide: {e:?}"))?;
    for (name, value) in [
        ("attributeName", "transform".to_owned()),
        ("type", "translate".to_owned()),
        ("from", format!("{} {}", from.0, from.1)),
        ("to", format!("{} {}", to.0, to.1)),
        ("dur", SLIDE_SECONDS.to_owned()),
        ("fill", "freeze".to_owned()),
        // The slide starts when it is told to, and not at the start of the document's own time, which is long past.
        ("begin", "indefinite".to_owned()),
    ] {
        animation
            .set_attribute(name, &value)
            .map_err(|e| format!("could not set {name} on the slide: {e:?}"))?;
    }
    group
        .append_child(&animation)
        .map_err(|e| format!("could not add the slide: {e:?}"))?;
    let begin = js_sys::Reflect::get(&animation, &"beginElement".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    if let Some(begin) = begin {
        begin
            .call0(&animation)
            .map_err(|e| format!("could not start the slide: {e:?}"))?;
    }
    Ok(())
}
