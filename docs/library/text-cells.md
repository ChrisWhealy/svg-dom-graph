# Text Cells

`NodeValues::Text` holds strings, one to a cell, and `DataFormat::Text` writes them.
Like every other type, all the values of a node share one.
A text node has a colour of its own, a pale neutral grey, and a name for assistive technology: `text`.

```rust
DataNodeContent::new(
    NodeValues::Text(vec!["0".into(), "∞".into(), "—".into()]),
    DataFormat::Text,
)
.with_cell_width_hint("9999999999")
```

## What a cell may hold

- Any Unicode text on one line, and an empty string.
- Never markup.
  A string is set as the text of an SVG text element, so `<b>x</b>` is drawn as those characters and makes no element.
- Runs of spaces are kept as they are.
- No control character, which includes a line break and a tab.
  Text with one is rejected with `Error::InvalidTextContent` before anything is drawn.

`Text` values need `DataFormat::Text`, and `DataFormat::Text` needs `Text` values.
Any other pairing is also `Error::InvalidTextContent`.
No operator accepts text, and one given it is rejected with `Error::OperatorOnText`.

## Row labels, selection and the grid

A text node is a grid like any other.
`GridLayout`, `with_labels`, `LabellingStyle`, the selection and the secondary selection work as they do for numbers.
The name of a cell says its row and column, as in `row 0, column 2: ∞`.
An empty cell is read out as `empty`, because a blank cell would be read out as nothing at all.

## How wide a cell is

A string has no fixed width.
A character such as `∞` or `—` may come from a font other than the monospace font the cells use.
Such a font gives it an advance of its own, and a character count says nothing about that.
So the browser measures every string, and every cell takes the width of the widest, plus its padding.

`set_data_values` does not resize cells.
A node that is drawn showing `∞` and is later given `1250` would have to outgrow its cells.
`DataNodeContent::with_cell_width_hint` prevents that.
It gives a sample string, which is measured and nothing else.
The sample is never drawn, never counted as a value, and never part of a node's name.
Each cell is at least as wide as the sample, and a string that is wider than the sample still gets the room it needs.
A caller who wants a width that never changes chooses the widest string the node will ever show.
The sample is held to the same rule as a cell, so it may not contain a control character.

## Replacing the strings

`Scene::set_data_values` needs `Text` values again, the same number of them, with no control character in any.
Only a string that changed is written, and the accessible name of the cell follows it.
Any other replacement is `Error::IncompatibleNodeValues`, and leaves the node as it was.
