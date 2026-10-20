//! The Base64 panel:
//!
//! - `panel-base64` / `#base64-diagram` — [`build_base64_demo`]: Base64 encoding of typed text, a group of three bytes
//!   at a time.

mod encode;

use crate::{
    source_file,
    util::{MARGIN, document, ensure_svg_in, mark_unreached, required_element, required_input, resize_svg, stringify},
};
use encode::{Alphabet, PADDING, encode, length_note, utf8_prefix};
use std::cell::{Cell, RefCell};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{
        BitValues, ConnectorOptions, DataFormat, DataNodeContent, DragOptions, GridLayout, HexDumpFormat, NodeValues,
        Scene, Side, ToolbarOptions,
    },
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the encoding, then the scene that draws it.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("base64/encode.rs", "encode.rs"),
    source_file!("base64/mod.rs", "mod.rs"),
];

/// What the callbacks need to find again: the scene on the page, and what it shows.
struct Demo {
    scene: Scene,
    alphabet: Alphabet,
    /// The text the scene was drawn for.
    text: String,
    /// The scene's text input node.
    input: NodeId,
}

thread_local! {
    // Keeps the demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does. Every change draws a new one.
    static DEMO: RefCell<Option<Demo>> = const { RefCell::new(None) };
    // Whether the user is part way through composing text with an input method editor.
    static COMPOSING: Cell<bool> = const { Cell::new(false) };
}

const STAGE_ID: &str = "base64-diagram-stage";
const DIAGRAM_ID: &str = "base64-diagram";

/// The text the demo opens on. It is 62 bytes, which do not divide into groups of three, so the last group needs padding.
const INITIAL_TEXT: &str = "This is what some text looks like when encoded using Base64...";

/// How many bytes of the typed text are encoded: 256, which is not a whole number of groups, so the last group of a
/// full node needs padding. Text past it is not encoded, and the cut falls between characters, never inside one. The hex dump always has room for this many.
const INPUT_LEN: usize = 256;

/// How many values the six-bit node lays out on a row, which is four whole groups.
const ROW_LEN: usize = 16;

/// The gap between the two hex dumps of a row, wide enough for a connector, and the room east of them for the zoom
/// toolbar.
const GAP: f64 = 100.0;
const TOOLBAR_ROOM: f64 = 80.0;

/// How wide the text box is, at most, and how wide the note beside it is.
const INPUT_WIDTH: f64 = 580.0;
const NOTE_WIDTH: f64 = 360.0;

/// The space between one row of nodes and the next, wide enough for a connector.
const ROW_GAP: f64 = 60.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `values` followed by enough `fill` to make `len` values.
fn padded(values: &[u8], len: usize, fill: u8) -> Vec<u8> {
    let mut out = values.to_vec();
    out.resize(len, fill);
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene for `text` with the characters of `alphabet`, replacing whatever scene was there, and returns the
/// new text input node. It keeps the zoom and pan the old scene had.
///
/// The library cannot change how many values a node holds, so the six-bit values and the characters are drawn again for
/// each change, with exactly as many values as the text needs. A node stays as small as its text allows, and an empty
/// text has neither. The hex dump always holds [`INPUT_LEN`] bytes, with the ones the text does not reach dimmed. The
/// positions the nodes were dragged to are not kept.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
fn build(text: &str, alphabet: Alphabet) -> Result<NodeId, String> {
    let view = DEMO.with_borrow(|slot| slot.as_ref().map(|d| d.scene.view()));
    let document = document()?;
    ensure_svg_in(&document, STAGE_ID, DIAGRAM_ID, Size::new(1000.0, 1000.0))?;
    required_element(&document, DIAGRAM_ID)?.set_inner_html("");
    let svg = SvgRoot::attach(DIAGRAM_ID).map_err(stringify)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let bytes = utf8_prefix(text, INPUT_LEN);
    let encoded = encode(bytes, alphabet);

    let hexdump_content = DataNodeContent::new(
        NodeValues::U8(padded(bytes, INPUT_LEN, 0)),
        DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
    );
    // The text box is centred over the hex dump, and is no wider than the dump.
    let hexdump_size = scene
        .measure_named_data_node("Input bytes", &hexdump_content)
        .map_err(stringify)?;
    let input_width = hexdump_size.width.min(INPUT_WIDTH);
    let input = scene
        .add_text_input_node(
            Point::new(MARGIN + (hexdump_size.width - input_width) / 2.0, MARGIN),
            input_width,
            "Text to encode",
            text,
            |_, id, text| typed(id, text),
        )
        .map_err(stringify)?;
    let input_rect = scene.node_rect(input).map_err(stringify)?;

    // What the text box holds that the encoding leaves out, beside it.
    let note = scene
        .add_markdown_node(
            Point::new(input_rect.origin.x + input_rect.size.width + GAP / 2.0, MARGIN),
            NOTE_WIDTH,
            &length_note(text.len(), bytes.len(), INPUT_LEN),
        )
        .map_err(stringify)?;
    let note_rect = scene.node_rect(note).map_err(stringify)?;

    let top = (input_rect.origin.y + input_rect.size.height).max(note_rect.origin.y + note_rect.size.height) + ROW_GAP;
    let hexdump = scene
        .add_named_data_node(Point::new(MARGIN, top), "Input bytes", hexdump_content)
        .map_err(stringify)?;
    let unreached: Vec<usize> = (bytes.len()..INPUT_LEN).collect();
    mark_unreached(&scene, hexdump, &unreached)?;
    let hexdump_rect = scene.node_rect(hexdump).map_err(stringify)?;

    let downward = ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North));
    // The six-bit values have two fixing points on their north edge, a quarter and three quarters of the way along. The
    // link from the input bytes lands on the left one, and the link back up to the Base64 text leaves from the right.
    let into_sextets = downward.with_to_position(Some(0.25));
    let back_up = ConnectorOptions::default()
        .with_from_side(Some(Side::North))
        .with_from_position(Some(0.75))
        .with_to_side(Some(Side::South));
    scene.add_edge_with(input, hexdump, downward).map_err(stringify)?;

    let mut nodes = vec![input, note, hexdump];
    if !encoded.sextets.is_empty() {
        // The data flows down from the input bytes to the six-bit values, then back up to the Base64 text, which is a
        // hex dump beside the input bytes. The Base64 text is a third longer than the input, so the six-bit values go
        // under whichever of the two is lower.
        let characters = scene
            .add_named_data_node(
                Point::new(hexdump_rect.origin.x + hexdump_rect.size.width + GAP, hexdump_rect.origin.y),
                "Base64 text",
                DataNodeContent::new(
                    NodeValues::U8(encoded.text.clone()),
                    DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
                ),
            )
            .map_err(stringify)?;
        let characters_rect = scene.node_rect(characters).map_err(stringify)?;
        let sextets_top = (hexdump_rect.origin.y + hexdump_rect.size.height)
            .max(characters_rect.origin.y + characters_rect.size.height)
            + ROW_GAP;
        let sextets = scene
            .add_named_data_node(
                Point::new(MARGIN, sextets_top),
                "Six-bit values",
                DataNodeContent::new(
                    NodeValues::Bits(BitValues::new(6, encoded.sextets.clone()).map_err(stringify)?),
                    DataFormat::Binary,
                )
                .with_layout(GridLayout::Columns(ROW_LEN)),
            )
            .map_err(stringify)?;
        // The six bits behind a padding character are zeros that the padding made up, so the encoding does not use them.
        let padding: Vec<usize> = (0..encoded.text.len()).filter(|&i| encoded.text[i] == PADDING).collect();
        mark_unreached(&scene, sextets, &padding)?;
        scene.add_edge_with(hexdump, sextets, into_sextets).map_err(stringify)?;
        scene.add_edge_with(sextets, characters, back_up).map_err(stringify)?;
        nodes.extend([sextets, characters]);
    }

    // The canvas is as wide and tall as its nodes need, with room east of them for the zoom toolbar.
    let mut right = 0.0_f64;
    let mut bottom = 0.0_f64;
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

    required_element(&document, "base64-result")?.set_text_content(Some(&String::from_utf8_lossy(&encoded.text)));
    required_element(&document, "base64-alphabet")?
        .set_text_content(Some(std::str::from_utf8(alphabet.characters()).map_err(stringify)?));

    DEMO.with_borrow_mut(|slot| {
        *slot = Some(Demo {
            scene,
            alphabet,
            text: text.to_owned(),
            input,
        });
    });
    Ok(input)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box now holds `text`. Draws the scene again for it, unless the user is composing text with an input method
/// editor. The scene is replaced along with its text box, and a composition does not survive that. So the scene waits
/// until the composition ends: see [`composition_ended`].
fn typed(id: NodeId, text: &str) {
    if !COMPOSING.get() {
        redraw(id, text);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the scene again for `text`, then gives the new text box the keyboard focus, with the caret where it was, so
/// that typing carries on in it. A failure is logged to the console, since the text box's own callback has nowhere to
/// return it to.
fn redraw(id: NodeId, text: &str) {
    let state = DEMO.with_borrow(|slot| slot.as_ref().map(|d| (d.alphabet, d.scene.text_input_caret(id).ok())));
    let result = state
        .ok_or_else(|| "the diagram is not built".to_owned())
        .and_then(|(alphabet, caret)| {
            let input = build(text, alphabet)?;
            DEMO.with_borrow(|slot| match slot {
                Some(demo) => demo.scene.focus_text_input(input, caret).map_err(stringify),
                None => Ok(()),
            })
        });
    if let Err(e) = result {
        web_sys::console::error_1(&format!("Could not draw the text: {e}").into());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The user has finished composing. Draws the scene for the text the text box holds now, if it is not the text the scene
/// was drawn for. The browser can send the last `input` event either before or after `compositionend`, and the text may
/// not be in the box until the event that follows it. So this looks once the event has been handled, and draws nothing
/// if the `input` event has already done so.
fn composition_ended() {
    let value = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(&format!("#{DIAGRAM_ID} input")).ok().flatten())
        .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value());
    let input = DEMO.with_borrow(|slot| slot.as_ref().filter(|d| Some(&d.text) != value.as_ref()).map(|d| d.input));
    if let (Some(input), Some(value)) = (input, value) {
        redraw(input, &value);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Watches the stage for the start and end of a composition. Composition events bubble, and the stage outlives every
/// scene that is drawn in it, so these two listeners are attached once.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached.
fn watch_composition(document: &web_sys::Document) -> Result<(), String> {
    let stage = required_element(document, STAGE_ID)?;
    let started = Closure::<dyn FnMut()>::new(|| COMPOSING.set(true));
    let ended = Closure::<dyn FnMut()>::new(|| {
        COMPOSING.set(false);
        // Not at once: the text box may not hold the composed text until the events that follow have been handled.
        if let Some(window) = web_sys::window() {
            let later = Closure::once_into_js(composition_ended);
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
        }
    });
    for (name, listener) in [("compositionstart", &started), ("compositionend", &ended)] {
        stage
            .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach the {name} listener: {e:?}"))?;
    }
    started.forget();
    ended.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the same text again with `alphabet`. The text is that of the text box in the scene on the page.
fn choose(alphabet: Alphabet) {
    let text = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(&format!("#{DIAGRAM_ID} input")).ok().flatten())
        .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default();
    if let Err(e) = build(&text, alphabet) {
        web_sys::console::error_1(&format!("Could not draw the alphabet: {e}").into());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Base64 demo: a text input node and the nodes that follow it down the page.
///
/// 1. The text input node holds the text to encode. Each change encodes it again.
/// 2. The bytes of the text as a hex dump. It always has room for 256 bytes, and dims the ones the text does not reach.
/// 3. The bits of every group of three bytes cut into four six-bit values, under the hex dump. They are shown in binary
///    as a bit array of six-bit values, so each is exactly six digits. Each value is an index into the alphabet `A-Z`,
///    `a-z`, `0-9`, then `+` and `/`, or `-` and `_` in the URL-safe alphabet. There are exactly as many as the text
///    needs, so the node has a row only when there are values for it.
/// 4. The character for each value, as a second hex dump beside the first, so that the data flows down the page and
///    then back up. A group of one or two bytes is short, so its last two or one characters are the padding `=`, and the
///    six-bit values behind them are dimmed.
///
/// Every value is worked out in plain Rust by [`encode`]. The library does no encoding. A node holds the same number of
/// values for as long as it lives, so each change draws the scene again, as the Huffman demo does. Text past
/// [`INPUT_LEN`] bytes is not encoded.
///
/// A scene is not drawn while the user is composing text with an input method editor, since drawing it replaces the
/// text box and ends the composition. It is drawn when the composition ends.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing a part of the panel, or if any library call fails.
pub(crate) fn build_base64_demo() -> Result<(), String> {
    let document = document()?;
    for (id, alphabet) in [
        ("base64-alphabet-standard", Alphabet::Standard),
        ("base64-alphabet-url", Alphabet::UrlSafe),
    ] {
        let radio = required_input(&document, id)?;
        radio.set_checked(alphabet == Alphabet::Standard);
        let on_change = Closure::<dyn FnMut()>::new(move || choose(alphabet));
        radio
            .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
            .map_err(|e| format!("could not attach the listener for {id}: {e:?}"))?;
        on_change.forget();
    }
    watch_composition(&document)?;
    build(INITIAL_TEXT, Alphabet::Standard).map(|_| ())
}

#[cfg(test)]
mod unit_tests;
