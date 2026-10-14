//! `panel-text-nodes` / `#text-nodes-diagram`: a Markdown node and a text input node. See [`build_text_nodes_demo`]'s own
//! doc comment for what each demonstrates.

use crate::util::{document, ensure_svg_in, required_element, stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::scene::{ConnectorOptions, DragOptions, Scene, Side};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("text_nodes.rs");

thread_local! {
    // Keeps the `Scene` alive for the page's lifetime, as `tree::SCENE` does.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the text-nodes demo: a Markdown node, a text input node, and a connector between them.
///
/// 1. The Markdown node is [`markdown`]'s text, which uses every supported construct: three heading levels, bold, italic, code,
///    wrapped paragraphs and bullet and numbered lists with a hanging indent, a hard break, and escapes. Its height is
///    whatever the text needs, so the text input node is placed from [`Scene::node_rect`] below it.
/// 2. The text input node holds one line of text the browser edits. Each change calls back with the new text, which
///    this demo writes to the line below the diagram. The text box keeps the keyboard focus while the diagram is
///    panned, zoomed, or the nodes are dragged, and typing in it never pans or zooms the diagram.
/// 3. Both nodes can be dragged, and the connector between them follows. A text input node is a node like any other.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#text-nodes-diagram-stage` or `#text-nodes-echo`, or if any library call
/// fails.
pub(crate) fn build_text_nodes_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(
        &document,
        "text-nodes-diagram-stage",
        "text-nodes-diagram",
        Size::new(620.0, 640.0),
    )?;
    let svg = SvgRoot::attach("text-nodes-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    // The size the node is measured to have, before it is drawn, is reported beside the size it is drawn at.
    let measured = scene.measure_markdown_node(580.0, &markdown()).map_err(stringify)?;
    let markdown = scene
        .add_markdown_node(Point::new(20.0, 20.0), 580.0, &markdown())
        .map_err(stringify)?;
    let below = scene.node_rect(markdown).map_err(stringify)?;
    required_element(&document, "text-nodes-measure")?.set_text_content(Some(&format!(
        "measured {}x{}, drawn {}x{}",
        measured.width, measured.height, below.size.width, below.size.height
    )));

    let echo = required_element(&document, "text-nodes-echo")?;
    let initial = "Edit this text";
    echo.set_text_content(Some(initial));
    // Counts the calls the callback receives, so that a change the callback was not told about can be told apart.
    let calls = required_element(&document, "text-nodes-calls")?;
    calls.set_text_content(Some("0"));
    let count = std::cell::Cell::new(0_u32);
    let input = scene
        .add_text_input_node(
            Point::new(20.0, below.origin.y + below.size.height + 50.0),
            580.0,
            "Text to echo",
            initial,
            move |_, _, text| {
                count.set(count.get() + 1);
                calls.set_text_content(Some(&count.get().to_string()));
                echo.set_text_content(Some(text));
            },
        )
        .map_err(stringify)?;

    // Replacing the text from code is a change the caller makes, so it does not call the callback. The text holds a
    // character outside the Basic Multilingual Plane, which is two UTF-16 code units.
    let replace_scene = scene.clone();
    let replace = Closure::<dyn FnMut()>::new(move || {
        let _ = replace_scene.set_text_input_value(input, "Replaced \u{2603} \u{1F600}!");
    });
    required_element(&document, "text-nodes-replace")?
        .add_event_listener_with_callback("click", replace.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the replace listener: {e:?}"))?;
    replace.forget();

    // A box narrower than its own label, to show the label shrinking to fit.
    let input_rect = scene.node_rect(input).map_err(stringify)?;
    scene
        .add_text_input_node(
            Point::new(20.0, input_rect.origin.y + input_rect.size.height + 20.0),
            100.0,
            "Text to be compressed",
            "",
            |_, _, _| {},
        )
        .map_err(stringify)?;

    let downward = ConnectorOptions::default()
        .with_from_side(Some(Side::South))
        .with_to_side(Some(Side::North));
    scene.add_edge_with(markdown, input, downward).map_err(stringify)?;

    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(markdown, drag_options).map_err(stringify)?;
    scene.make_draggable_with(input, drag_options).map_err(stringify)?;

    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
    Ok(())
}
