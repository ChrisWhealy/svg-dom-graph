//! The two text-node panels:
//!
//! - `panel-markdown` / `#markdown-diagram` — [`build_markdown_demo`]: a Markdown node.
//! - `panel-text-input` / `#text-input-diagram` — [`build_text_input_demo`]: a text input node, whose text appears in
//!   an ASCII node and a hexdump node as it is typed.

use crate::util::{document, ensure_svg_in, required_element, stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::scene::{
    ConnectorOptions, DataFormat, DataNodeContent, DragOptions, GridLayout, HexDumpFormat, NodeValues, Scene, Side,
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("text_nodes.rs");

thread_local! {
    // Keep each demo's `Scene` alive for the page's lifetime, as `tree::SCENE` does.
    static MARKDOWN_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
    static TEXT_INPUT_SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

/// How many bytes of the typed text the ASCII node and the hexdump node hold. The library can change the value of a
/// node but not how many values it holds, so both are a buffer of this size. Text past it is not shown, and text
/// shorter than it leaves the rest of the buffer as zero bytes.
const BUFFER_LEN: usize = 32;

/// How many columns the ASCII node lays its [`BUFFER_LEN`] bytes out in.
const ASCII_COLUMNS: usize = 16;

/// A word with no spaces in it that is longer than a whole line of the node, so the node has to split it.
const LONG_WORD: &str =
    "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";

/// The Markdown the first node shows. It uses every construct the node supports, so that the demo is its own reference.
fn markdown() -> String {
    format!(
        "\
# A Markdown node

A node can hold **bold**, *italic* and ***both***, and `inline code`. A line that is too long for the node wraps \
between words, and a word longer than a whole line, such as {LONG_WORD}, is split.
A line ending in two spaces breaks the line.

## Lists

- Each bullet wraps with a hanging indent, so a long item such as this one keeps its text under its own first word.
- Another bullet.

1. Numbered items count up.
2. As the numbers say.

### Escapes

A backslash makes punctuation literal: \\*not italic\\* and \\`not code\\`."
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Markdown demo: one Markdown node.
///
/// The node is [`markdown`]'s text, which uses every supported construct: three heading levels, bold, italic, code,
/// wrapped paragraphs and bullet and numbered lists with a hanging indent, a hard break, and escapes. Its width is
/// chosen here and its height is whatever the text needs. The size it is measured to have before it is drawn is
/// reported beside the size it is drawn at. The node can be dragged, like any other node.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#markdown-diagram-stage` or `#markdown-measure`, or if any library call
/// fails.
pub(crate) fn build_markdown_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, "markdown-diagram-stage", "markdown-diagram", Size::new(620.0, 560.0))?;
    let svg = SvgRoot::attach("markdown-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let measured = scene.measure_markdown_node(580.0, &markdown()).map_err(stringify)?;
    let node = scene
        .add_markdown_node(Point::new(20.0, 20.0), 580.0, &markdown())
        .map_err(stringify)?;
    let drawn = scene.node_rect(node).map_err(stringify)?;
    required_element(&document, "markdown-measure")?.set_text_content(Some(&format!(
        "measured {}x{}, drawn {}x{}",
        measured.width, measured.height, drawn.size.width, drawn.size.height
    )));

    scene
        .make_draggable_with(node, DragOptions::default().with_bounds(Some(bounds)))
        .map_err(stringify)?;

    MARKDOWN_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text as the [`BUFFER_LEN`] bytes the ASCII node and the hexdump node hold: its UTF-8 bytes, cut off at the end
/// of the buffer, and padded with zero bytes if shorter.
fn buffer_of(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(BUFFER_LEN, 0);
    bytes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the text input demo: a text input node, an ASCII node and a hexdump node, with a connector from the text
/// input node to each of the others.
///
/// 1. The text input node holds one line of text the browser edits. The text box keeps the keyboard focus while the
///    diagram is panned, zoomed, or the nodes are dragged, and typing in it never pans or zooms the diagram.
/// 2. Each change calls back with the new text. The callback writes the text to the line below the diagram, counts the
///    call, and gives the bytes of the text to the other two nodes with `Scene::set_data_values`. The ASCII node shows
///    each byte as a character, and the hexdump node as `hexdump -C` does. Both hold [`BUFFER_LEN`] bytes. See
///    [`BUFFER_LEN`] for what that means for text that is shorter or longer.
/// 3. A button replaces the text from code, which is a change the caller makes and so does not call the callback.
/// 4. A second, narrower text input node shows a label that is wider than its box shrinking to fit.
/// 5. All the nodes can be dragged, and the connectors follow.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#text-input-diagram-stage`, `#text-input-echo`, `#text-input-calls` or
/// `#text-input-replace`, or if any library call fails.
pub(crate) fn build_text_input_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(
        &document,
        "text-input-diagram-stage",
        "text-input-diagram",
        Size::new(700.0, 700.0),
    )?;
    let svg = SvgRoot::attach("text-input-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let initial = "Edit this text";
    let ascii = scene
        .add_named_data_node(
            Point::new(20.0, 250.0),
            "The text as ASCII",
            DataNodeContent::new(NodeValues::U8(buffer_of(initial)), DataFormat::Ascii)
                .with_layout(GridLayout::Columns(ASCII_COLUMNS)),
        )
        .map_err(stringify)?;
    let ascii_rect = scene.node_rect(ascii).map_err(stringify)?;
    let hexdump = scene
        .add_named_data_node(
            Point::new(20.0, ascii_rect.origin.y + ascii_rect.size.height + 60.0),
            "The text as a hexdump",
            DataNodeContent::new(
                NodeValues::U8(buffer_of(initial)),
                DataFormat::HexDump(HexDumpFormat::new().with_offset(true)),
            ),
        )
        .map_err(stringify)?;

    let echo = required_element(&document, "text-input-echo")?;
    echo.set_text_content(Some(initial));
    // Counts the calls the callback receives, so that a change the callback was not told about can be told apart.
    let calls = required_element(&document, "text-input-calls")?;
    calls.set_text_content(Some("0"));
    let count = std::cell::Cell::new(0_u32);
    let input = scene
        .add_text_input_node(Point::new(20.0, 20.0), 580.0, "Text to echo", initial, move |scene, _, text| {
            count.set(count.get() + 1);
            calls.set_text_content(Some(&count.get().to_string()));
            echo.set_text_content(Some(text));
            // Both nodes were built with BUFFER_LEN values, so neither call can be refused for their number.
            let _ = scene.set_data_values(ascii, NodeValues::U8(buffer_of(text)));
            let _ = scene.set_data_values(hexdump, NodeValues::U8(buffer_of(text)));
        })
        .map_err(stringify)?;

    // Replacing the text from code is a change the caller makes, so it does not call the callback, and the other two
    // nodes keep what they showed. The text holds a character outside the Basic Multilingual Plane, which is two UTF-16
    // code units.
    let replace_scene = scene.clone();
    let replace = Closure::<dyn FnMut()>::new(move || {
        let _ = replace_scene.set_text_input_value(input, "Replaced \u{2603} \u{1F600}!");
    });
    required_element(&document, "text-input-replace")?
        .add_event_listener_with_callback("click", replace.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the replace listener: {e:?}"))?;
    replace.forget();

    // A box narrower than its own label, to show the label shrinking to fit.
    let input_rect = scene.node_rect(input).map_err(stringify)?;
    scene
        .add_text_input_node(
            Point::new(20.0, input_rect.origin.y + input_rect.size.height + 20.0),
            100.0,
            "Long labels shrink!",
            "",
            |_, _, _| {},
        )
        .map_err(stringify)?;

    let downward = ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North));
    scene.add_edge_with(input, ascii, downward).map_err(stringify)?;
    scene.add_edge_with(ascii, hexdump, downward).map_err(stringify)?;

    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    for node in [input, ascii, hexdump] {
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
    }

    TEXT_INPUT_SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
