# Text Nodes

Two kinds of node hold text instead of numeric values.
A Markdown node explains something in formatted prose, next to the thing it explains.
A text input node holds a text box in which the user can type.
Neither is a data node, so neither can have cells.
A call to `set_selection` on either is therefore rejected.

## Markdown nodes

- `add_markdown_node`/`add_markdown_node_with` draw a box `width` wide, whose height is calculated according to the quantity of text being displayed.

  `node_rect` reports the height, so whatever sits below the node can be spaced correctly.
  `measure_markdown_node` returns the rendered size without adding the node to the DOM.
  The width of a character is measured in the browser, and is taken as 0.6 of the font size when the `<svg>` is not displayed.
  So a measurement made while the scene is hidden can differ from the drawn size once it is visible.
  It differs by a whole line if a line break moves.
  Measure and draw under the same conditions, and read `node_rect` when the exact height matters.
  When creating a text node:
  * it must contain at least one, non-whitespace character, or the call returns `Error::EmptyMarkdown`, and
  * `width` must provide enough space for at least a few characters, and for the widest list marker in the text with one character beside it.
    A numbered list starting at `999999999.` needs a wider node than a bulleted one, and a narrower node returns `Error::InvalidNodeGeometry`.

## Supported Markdown Syntax

Only a subset of the full markdown syntax is available here:

Blocks:

- A heading is one to three `#`, a space, and some text.
  Four or more `#` are drawn as a level 3 heading.
  A run of `#` after the text is decoration only if it follows a space, so `# Title ##` is `Title` and `# C#` is `C#`.
  A `#` alone is text.
- A paragraph runs until a blank line, and its lines join with a space.
  A line that ends in two spaces or a backslash breaks the line.
- A bullet item starts with `-`, `*` or `+` followed by a space.
- A numbered item starts with a number followed by a `.` or `)` and a space.
  A line indented by two spaces or more carries on the item before it, even if it starts with a marker.
  So a nested list is drawn as text in its parent's item.
  A numbered list counts up from its first number.

Inline:

- `**bold**` or `__bold__`, `*italic*` or `_italic_`, and `***both***`.
- `` `code` `` is drawn in a different colour.
  Use double backticks to display the backtick character.
- A backslash before punctuation makes that character literal.

An emphasis marker with no closing marker is literal text.
Emphasis nests inside emphasis when the markers differ in length, as in `**bold with *italic* inside**`.
An underscore only opens or closes emphasis at the edge of a word, so the word `case` inside `snake_case_name` is left alone.

## Unsupprted Markdown Syntax

Links, images, block quotes, tables, code blocks, HTML and nested lists are not understood, and are drawn as the characters they were typed with.

## Why No Dependency on an External Markdown Crate?

The only crates on which `svg-dom-graph` depends are `svg-dom`, `wasm-bindgen` and `web-sys` — and nothing else.

A Markdown crate such as `comrak` is a general CommonMark parser, and it produces HTML or a syntax tree; neither of which are helpful here.

A markdown node draws SVG `<text>` that this crate lays out itself, in a monospace font.
So what it needs is lines of styled runs, already wrapped.
The subset above is all a node needs, and it takes a few hundred lines that have no DOM in them and are tested natively.
Text drawn this way is never HTML, so nothing in it can be taken for markup.

## Layout

Every line is wrapped at a width measured from a real element, so the wrapping is right for whichever monospace font the browser uses.
A bold or italic character is as wide as a regular one in a monospace font.
So a run's width is its character count multiplied by a single number.
All characters are counted as one column, so text that is wider than a column, such as some East Asian characters, can overrun its line.
A word longer than a whole line is split.
Between two words of prose one space is drawn, however much whitespace was written.
Whitespace inside a code span is kept exactly as written, so `A  B` is not drawn as `A B`.
It counts toward the width of the line.
Where a line breaks, the whitespace there is dropped.
A heading is larger, so fewer of its characters fit a line, and a list item keeps its text under its first word.

The node is named `Text` for assistive technology, and the text itself is exposed as it is drawn.

## Text input nodes

- `add_text_input_node`/`add_text_input_node_with` draw a box with a `label` above a one-line text box, which starts holding `value`.

  The label is also the text box's accessible name, so it may not be empty or only whitespace.
  `on_input` is called with the scene, the node's id and the new text each time it changes.
- `text_input_value` reads the text, and `set_text_input_value` replaces it.

  Replacing the text does not call `on_input`, since the caller is the one making the change.
- `text_input_caret` and `focus_text_input` read the caret, and give the text box the focus with the caret at a given place.

  A caller that draws the whole scene again each time the text changes needs both.
  The text box it was typing in is then gone, and with it both the focus and the cursor location.
  Read the cursor location (caret) before drawing, and give the new text box the focus afterward.
  Both ends of the selection are in UTF-16 code units, as the browser counts them.

The text box is an ordinary HTML `<input>` inside an SVG `<foreignObject>`, so the caret, selection, clipboard and input methods are the browser's own.
The node moves, zooms and pans with the scene, and a connector can be anchored to it.

Typing in the text box does not pan or zoom the scene.
The scene's keyboard handling is on a separate element that cannot have descendants.

A node made draggable can still be typed in.
A pointer that goes down on the text box belongs to the text box.
A pointer that goes down anywhere else on the node drags it.
