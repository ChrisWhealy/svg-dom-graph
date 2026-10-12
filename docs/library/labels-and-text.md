# Row Labels and Text Measurement

This page describes the labels a data node can draw beside the rows of its grid, and how this crate sizes anything that contains text.

- **Row labels.**
  `DataNodeContent::with_labels` and `with_labelling_style` draw a label beside each row of a grid, in the grid box's own left padding.

  `LabellingStyle` is one of three:

  - `Numeric` labels by element index, and is the default for `with_labels`.
  - `Alphabetic` labels by element index in lowercase letters: `a` to `z`, then `aa`, `ab` and so on.
  - `Text(&'static [&'static str])` labels by *row*: string `r` labels row `r`, whatever the number of columns, so `["0-4", "5-9"]` labels the rows of a 5-column grid.
    A row with no string, or an empty one, is drawn with no label at all, and no element is created for it.

  For `Numeric` and `Alphabetic`, a wider grid shows the label of the first element of each row.
  A label is hidden from assistive technology, and is named in each cell's own `aria-label` instead: `"element a, row 0, column 0: 6A"`.
  A `Text` label names the row, so it reads `"row label ms[n-3], row 0, column 0: 6A"`, and a row with no label has no such phrase.

  The label column is as wide as the widest label of a row that actually holds a cell.
  A row the layout asks for but has no value for takes no room.

- **Text is measured, never counted.**
  Every width this crate derives from text is taken from the rendered text, through `getBBox()`, and never from a character count.
  That covers a plain node's label, a data node's name and row labels, an operator node's label, the scene title, and the label of a selection toolbar button.
  A label may therefore use any script: a CJK glyph, or a letter with combining marks, takes the room it really renders at, whichever fonts the browser has.
  The values in a grid are digits or printable ASCII, so for them the widest cell is found from the values and then measured once.
