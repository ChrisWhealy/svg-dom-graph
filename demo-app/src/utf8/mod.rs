//! The UTF-8 panel:
//!
//! - `panel-utf8` / `#utf8-diagram` — [`build_utf8_demo`]: UTF-8 encoding of one of four characters, one for each length
//!   of sequence.

pub(crate) mod encode;
mod layout;

use crate::{
    source_file,
    svg_buttons::{self, Button},
    util::{MARGIN, document, ensure_svg_in, required_element, resize_svg, stringify},
};
use encode::{SAMPLES, button_label, byte_template_bits, encode, explain, hex_of, payload_groups};
use layout::{centred_lefts, centred_over};
use std::cell::{Cell, RefCell};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::scene::{
    BitValues, ConnectorOptions, ConnectorType, DataFormat, DataNodeContent, DragOptions, GridLayout, NodeValues,
    Scene, Side, ToolbarOptions,
};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the encoding, where the rows of nodes go, then the scene that draws it.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("utf8/encode.rs", "encode.rs"),
    source_file!("utf8/layout.rs", "layout.rs"),
    source_file!("utf8/mod.rs", "mod.rs"),
];

thread_local! {
    // Keeps the demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does. Every change draws a new one.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    // Which of `SAMPLES` is shown.
    static SELECTED: Cell<usize> = const { Cell::new(0) };
}

const STAGE_ID: &str = "utf8-diagram-stage";
const DIAGRAM_ID: &str = "utf8-diagram";

/// The room east of the nodes for the zoom toolbar, and the gap between one row of the column and the next, which is wide
/// enough for a connector, and between the note and the row under it.
const TOOLBAR_ROOM: f64 = 80.0;
const ROW_GAP: f64 = 50.0;

/// The gap between the boxes of two neighbouring nodes in a row of bits, which is small, so that the row reads as one.
const NODE_GAP: f64 = 20.0;

/// The gap between the buttons and the note under them.
const NOTE_GAP: f64 = 20.0;

/// The size of a button and the gap between one and the next. The buttons are as wide as they need to be for the four of them to be as wide as the widest row of bits, and the note under them is as wide as the four.
const MIN_BUTTON_WIDTH: f64 = 180.0;
const BUTTON_HEIGHT: f64 = 36.0;
const BUTTON_GAP: f64 = 16.0;

/// The fill and the outline of a button, chosen or not.
const CHOSEN: (&str, &str) = ("#bcd9f7", "#2b5d9b");
const NOT_CHOSEN: (&str, &str) = ("#f1f1f1", "#767676");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `values` as a row of one-bit cells.
fn bit_row(values: &[u8]) -> Result<DataNodeContent, String> {
    Ok(DataNodeContent::new(
        NodeValues::Bits(BitValues::new(1, values.to_vec()).map_err(stringify)?),
        DataFormat::Binary,
    )
    .with_layout(GridLayout::Columns(values.len())))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of the node for byte `byte` of a sequence of `len` bytes: `base` alone for a sequence of one, and with the
/// number of the byte otherwise.
fn byte_name(base: &str, len: usize, byte: usize) -> String {
    if len == 1 { base.to_owned() } else { format!("{base} (byte {})", byte + 1) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of the node for the group of code point bits that fills byte `byte` of a sequence of `len` bytes. A group
/// can be as narrow as three bits, and a node is at least as wide as its name, so with more than one byte the name is
/// short enough to leave the nodes of the row clear of each other.
fn group_name(len: usize, byte: usize) -> String {
    if len == 1 {
        "Code point bits".to_owned()
    } else {
        format!("Bits {}", byte + 1)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds the four buttons that choose the character, along the top, and returns the y of the bottom of the row. The
/// chosen one is a toggle button that is pressed.
fn add_buttons(document: &web_sys::Document, selected: usize, left: f64, width: f64) -> Result<f64, String> {
    let layer = svg_buttons::layer(document, DIAGRAM_ID)?;
    for (index, &c) in SAMPLES.iter().enumerate() {
        let label = button_label(c);
        svg_buttons::add_button(
            document,
            &layer,
            Rect {
                origin: Point::new(left + index as f64 * (width + BUTTON_GAP), MARGIN),
                size: Size::new(width, BUTTON_HEIGHT),
            },
            &Button {
                label: &label,
                action: &format!("choose-{index}"),
                colours: if index == selected { CHOSEN } else { NOT_CHOSEN },
                pressed: Some(index == selected),
            },
        )?;
    }
    Ok(MARGIN + BUTTON_HEIGHT)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for the character `SAMPLES[selected]`, replacing whatever scene was there. It keeps the zoom and pan
/// the old scene had, and the keyboard focus on a button.
///
/// The library cannot change how many values a node holds, so the column is drawn again for each character, with
/// exactly as many values as the character needs. The positions the nodes were dragged to are not kept.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
fn build(selected: usize) -> Result<(), String> {
    let view = SCENE.with_borrow(|slot| slot.as_ref().map(Scene::view));
    let focused = focused_selector();
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let c = SAMPLES[selected];
    let encoded = encode(c);
    let len = encoded.bytes.len();

    // Everything is centred on one axis that stays put: the buttons, the note under them, and every row of the column. It is
    // as wide as a row of four bytes.
    let byte_box = scene
        .measure_named_data_node(&byte_name("UTF-8 bits", SAMPLES.len(), 0), &bit_row(&[0; 8])?)
        .map_err(stringify)?
        .width;
    let widest_row = SAMPLES.len() as f64 * byte_box + (SAMPLES.len() - 1) as f64 * NODE_GAP;
    // The four buttons, and the note under them, are as wide as the widest row of bits, so that the block has the same width
    // for every character, and the margin at its left is one margin, not more because a row is wider than the buttons.
    let count = SAMPLES.len() as f64;
    let button_width = ((widest_row - (count - 1.0) * BUTTON_GAP) / count).max(MIN_BUTTON_WIDTH);
    let buttons_width = count * button_width + (count - 1.0) * BUTTON_GAP;
    let span = buttons_width.max(widest_row);
    let axis = MARGIN + span / 2.0;
    let left = axis - buttons_width / 2.0;
    let buttons_bottom = add_buttons(&document, selected, left, button_width)?;
    // The contents of the two rows of bits. A group is as wide as the bits the template leaves in its byte, so the groups
    // are uneven, and the bytes are eight wide.
    let mut from = 0;
    let mut groups = Vec::with_capacity(len);
    for (n, &width) in payload_groups(len).iter().enumerate() {
        groups.push((group_name(len, n), bit_row(&encoded.payload[from..from + width])?));
        from += width;
    }
    let mut byte_rows = Vec::with_capacity(len);
    for n in 0..len {
        byte_rows.push((byte_name("UTF-8 bits", len, n), bit_row(&encoded.bits[8 * n..8 * n + 8])?));
    }
    // The row of bytes is centred on the axis the way a person sees it: the node in the middle, or the gap between the two
    // in the middle, is on the axis. Each group of code point bits is then centred over the byte it fills, so that the
    // groups and the bytes are in the same columns however uneven the groups are. The bits of a group are not always
    // directly over the bits of the byte that they fill, since the groups are narrower than the bytes.
    let widths_of = |rows: &[(String, DataNodeContent)]| -> Result<Vec<f64>, String> {
        rows.iter()
            .map(|(name, content)| {
                scene
                    .measure_named_data_node(name, content)
                    .map(|size| size.width)
                    .map_err(stringify)
            })
            .collect()
    };
    let byte_widths = widths_of(&byte_rows)?;
    let byte_lefts = centred_lefts(&byte_widths, NODE_GAP, axis);
    let group_lefts = centred_over(&byte_lefts, &byte_widths, &widths_of(&groups)?);

    let down = |elbow: bool| {
        let options = ConnectorOptions::default()
            .with_from_side(Some(Side::South))
            .with_to_side(Some(Side::North));
        if elbow {
            options.with_connector_type(ConnectorType::Elbow { corner_radius: 8.0 })
        } else {
            options
        }
    };
    let mut nodes = Vec::new();
    // The note is under the buttons, as wide as the row of them. Its height depends on the character, and the column is under
    // it, so room is kept for the tallest, and the column stays where it is as the character changes.
    let note_top = buttons_bottom + NOTE_GAP;
    let mut note_height = 0.0_f64;
    for sample in SAMPLES {
        let text = explain(sample, &encode(sample));
        note_height = note_height.max(scene.measure_markdown_node(buttons_width, &text).map_err(stringify)?.height);
    }
    let note = scene
        .add_markdown_node(Point::new(left, note_top), buttons_width, &explain(c, &encoded))
        .map_err(stringify)?;
    let top = note_top + note_height + ROW_GAP;

    // The code point, over the middle of the rows of bits.
    let point = DataNodeContent::new(NodeValues::U32(vec![encoded.code_point]), DataFormat::Hexadecimal);
    let point_size = scene.measure_named_data_node("Code point", &point).map_err(stringify)?;
    let point_node = scene
        .add_named_data_node(Point::new(axis - point_size.width / 2.0, top), "Code point", point)
        .map_err(stringify)?;
    nodes.push(point_node);

    // The groups of code point bits, then the bits of the bytes.
    let groups_top = top + point_size.height + ROW_GAP;
    let (mut group_ids, mut groups_height) = (Vec::new(), 0.0_f64);
    for ((name, content), left) in groups.into_iter().zip(group_lefts) {
        let height = scene.measure_named_data_node(&name, &content).map_err(stringify)?.height;
        let id = scene
            .add_named_data_node(Point::new(left, groups_top), &name, content)
            .map_err(stringify)?;
        groups_height = groups_height.max(height);
        group_ids.push(id);
    }
    let bytes_top = groups_top + groups_height + ROW_GAP;
    let (mut bytes, mut bytes_height) = (Vec::new(), 0.0_f64);
    for (n, ((name, content), left)) in byte_rows.into_iter().zip(byte_lefts).enumerate() {
        let height = scene.measure_named_data_node(&name, &content).map_err(stringify)?.height;
        let id = scene
            .add_named_data_node(Point::new(left, bytes_top), &name, content)
            .map_err(stringify)?;
        // The bits of the template are marked, so that the rest are seen to be the code point's.
        scene
            .set_secondary_selection(id, &byte_template_bits(len, n))
            .map_err(stringify)?;
        bytes_height = bytes_height.max(height);
        bytes.push(id);
    }
    for (&group, &byte) in group_ids.iter().zip(&bytes) {
        scene.add_edge_with(point_node, group, down(len > 1)).map_err(stringify)?;
        scene.add_edge_with(group, byte, down(true)).map_err(stringify)?;
    }

    // The bytes themselves, under the rows of bits.
    let hex = DataNodeContent::new(NodeValues::U8(encoded.bytes.clone()), DataFormat::Hexadecimal)
        .with_layout(GridLayout::Columns(len));
    let hex_size = scene.measure_named_data_node("UTF-8 bytes", &hex).map_err(stringify)?;
    let hex_node = scene
        .add_named_data_node(
            Point::new(axis - hex_size.width / 2.0, bytes_top + bytes_height + ROW_GAP),
            "UTF-8 bytes",
            hex,
        )
        .map_err(stringify)?;
    for &byte in &bytes {
        scene.add_edge_with(byte, hex_node, down(true)).map_err(stringify)?;
    }
    nodes.extend(group_ids);
    nodes.extend(bytes);
    nodes.push(hex_node);

    // The canvas is as wide and tall as its nodes need, with room east of them for the zoom toolbar.
    nodes.push(note);
    let (mut right, mut bottom) = (left + buttons_width, 0.0_f64);
    for &node in &nodes {
        let rect = scene.node_rect(node).map_err(stringify)?;
        right = right.max(rect.origin.x + rect.size.width);
        bottom = bottom.max(rect.origin.y + rect.size.height);
    }
    let size = Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN);
    resize_svg(&document, DIAGRAM_ID, size)?;
    scene.refresh_layout().map_err(stringify)?;

    let drag_options = DragOptions::default().with_bounds(Some(Rect {
        origin: Point::new(0.0, 0.0),
        size,
    }));
    for &node in &nodes {
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
    }
    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;
    if let Some(view) = view {
        scene.set_view(view).map_err(stringify)?;
    }

    required_element(&document, "utf8-result")?.set_text_content(Some(&format!(
        "{c} U+{:04X} is {}",
        encoded.code_point,
        hex_of(c)
    )));
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    if let Some(selector) = focused {
        focus(&selector);
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A selector for the button in the diagram that has keyboard focus, if one does, by its action.
fn focused_selector() -> Option<String> {
    let document = document().ok()?;
    let active = document.active_element()?;
    document.get_element_by_id(DIAGRAM_ID)?.contains(Some(&active)).then_some(())?;
    let action = active.get_attribute("data-action")?;
    Some(format!("#{DIAGRAM_ID} g[data-action=\"{action}\"]"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Gives keyboard focus to the element `selector` finds, if there is one. Without it, a keyboard user would find that
/// focus had gone back to the start of the page when the scene was drawn again.
fn focus(selector: &str) {
    let Ok(document) = document() else { return };
    if let Ok(Some(element)) = document.query_selector(selector) {
        if let Ok(element) = element.dyn_into::<web_sys::SvgElement>() {
            let _ = element.focus();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button was pressed. Shows the character it stands for. A failure is logged to the console, since the button's own
/// listener has nowhere to return it to.
fn choose(action: &str) {
    let Some(index) = action
        .strip_prefix("choose-")
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|&n| n < SAMPLES.len())
    else {
        return;
    };
    SELECTED.set(index);
    if let Err(e) = build(index) {
        web_sys::console::error_1(&format!("Could not draw the character: {e}").into());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the UTF-8 demo: four buttons along the top, one for a character of each length, and under them a column of
/// nodes for the character that is chosen.
///
/// The column is the path the bits of the character take, from top to bottom:
///
/// 1. The code point, as the hexadecimal number Unicode gives the character.
/// 2. The bits of the code point that go into the bytes, as many as the template has room for: 7, 11, 16 or 21.
/// 3. The bits of the encoded bytes, a byte to a row. A character takes one byte up to U+007F, two up to U+07FF, three up
///    to U+FFFF and four above that. The bits of the template, which say how long the sequence is and where each byte
///    sits in it, are highlighted. The rest are the bits of step 2, in order.
/// 4. The bytes in hexadecimal. These are what is stored or sent.
///
/// A note under the buttons says why the character takes that many bytes. So the column grows and shrinks with the
/// character: the bits of the bytes have one, two, three or four rows. Every value is worked out in plain Rust by
/// [`encode`], which fills in the template and is tested against the standard library's own encoder for every character.
/// The library does no encoding. A node holds the same number of values for as long as it lives, so choosing a character
/// draws the scene again.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
pub(crate) fn build_utf8_demo() -> Result<(), String> {
    let document = document()?;
    svg_buttons::watch(&document, STAGE_ID, choose)?;
    build(SELECTED.get())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
