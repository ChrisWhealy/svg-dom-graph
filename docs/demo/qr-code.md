# QR Code Generator

A QR code is a square grid of dark and light modules that a phone can read.
This demo makes the smallest kind: Version 1, which is 21 by 21 modules, with error correction level L.
It encodes up to 17 ASCII characters, in byte mode.
Every value on screen is a real result, worked out in plain Rust by an encoder written for this demo.
The library never computes anything: the demo works out each value and then hands it to `svg-dom-graph` to draw.

The scene is large, so the gallery panel keeps the description and a button.
The button opens the scene in a window of its own, sized to fit it.

## The scene

The graph on the left follows the text through the stages that make the symbol.
Each node has the same number of cells for every text, so the scene is built once.
Stepping, and typing new text, only write new values into the nodes already drawn, with `Scene::set_data_values`.
A node the walk has not reached yet is drawn faint by `Scene::set_unreached_cells`.

The matrix of 21 by 21 modules is on the right.
It is plain SVG in the scene's own layer, drawn with a fixed handful of elements however many modules have a colour.
There are three paths, one each for the modules that are dark, light and not yet coloured, and one path each for the outline of a role and the highlight of a step.
A module that has no colour yet is pale blue, so that it cannot be taken for a light one.

The text input node holds at most 17 characters.
The library has no limit of its own, so the demo sets `maxlength` on the node's `<input>`.
A character that cannot be encoded is not taken.
The reason is shown, and the text goes back to what made the symbol on show, with the caret where it was.

## The steps

The walk has 62 steps, and the same 62 for every text.
"Next" and "Prev" move between them, and six buttons jump to the first step of a stage.

* **Input and bytes.**<br>
  The text, and its bytes in a hex dump.
  Each character is one byte.
  The text node and the hex dump share one horizontal midline.
* **The bitstream.**<br>
  Four steps, one for each part.
  The mode indicator is `0100`, which says byte mode.
  The character count is the number of bytes, in 8 bits.
  Then come the bytes themselves, and a terminator of up to four zero bits.
  Each part is marked in the bitstream node, and a note beside the node says what it holds.
* **The data codewords.**<br>
  The bits are cut into 19 codewords of 8 bits.
  The pad codewords `EC` and `11` fill what is left, in turn.
* **Reed-Solomon.**<br>
  One step for each of the 19 data codewords, each marked in the data codewords as it is taken in.
  The remainder of the division of the data, as a polynomial over GF(256), by a fixed generator is the 7 error correction codewords.
* **The function modules.**<br>
  The finder patterns, the separators, the timing patterns and the dark module are coloured.
  The 30 modules that hold the format information are reserved but have no colour yet, since they say which mask is used.
* **Placement.**<br>
  One step for each of the 26 codewords.
  They fill the remaining modules two columns at a time, from the bottom right, upwards and then downwards, skipping the timing column.
* **Masks.**<br>
  One step for each of the eight masks.
  A mask flips a pattern of the data modules, and each candidate gets its own format information.
  Each is then scored, and the score of every candidate so far is in the table.
* **The final symbol.**<br>
  The candidate with the lowest total wins, and the lowest mask wins a tie.

The graph is joined by edges that the data flows along.
Two arrive on the north edge of the codewords node.
One lands over the middle of the 19 data codewords, and one over the middle of the 7 error correction codewords, which fill the last 7 places.

## The matrix and its roles

A module has a colour and a role, and either can be known first.
The list of roles outlines every module that has a role, one role at a time, and says what the role is for.
The finder patterns, the separators, the timing patterns, the dark module and the format modules are the roles of the function modules.
The data and error correction roles belong to the modules that hold codeword bits.
A role is never shown by colour alone: the outline is a shape, and the live region names the role and how many modules it has.

## The scores

The penalty of a candidate has four parts.

* **N1.** Each run of five or more modules of one colour in a row or column scores 3, and 1 more for each module past five.
* **N2.** Each 2 by 2 block of one colour scores 3.
* **N3.** Each place where the pattern dark, light, dark, dark, dark, light, dark with four light modules beside it starts scores 40.
* **N4.** Every whole 5% that the share of dark modules is away from half scores 10.

N3 has a convention of its own, which is this demo's.
Every row and column has four light modules added at each end first, so that a pattern at the edge of the symbol has the light modules it needs.
Every start position counts, and a pattern with light on both sides is found by both patterns and scores 80.
python-qrcode adds no light modules at the ends, and segno scores a pattern with light on both sides once, so neither always gives this score.
Any of the eight masks gives a symbol that scans, so the choice only decides which one is shown.

## Exporting the symbol

"Export SVG" asks the browser to save the finished symbol as `qr-code.svg`.
The button is dimmed until the walk reaches the last step.
The matrix on screen is exactly 21 by 21 modules and has no quiet zone.
The file adds one, of four light modules on every side, so the picture is 29 by 29 modules.

The file is pure black on pure white.
Every module is 10 pixels wide, a whole number, and the edges are not antialiased, so the squares are sharp at any size.
A run of dark modules in a row is one rectangle.
The text is only written as escaped character data in the file's description, never as markup.
A control character is written as a visible escape, since XML cannot hold one.

Asking for the download does not change the walk or what is drawn.
If the request cannot be made, the reason is shown and the symbol on show stays as it was.
A page can only request a download, so what is announced is that the download was requested.
The browser may still block it, ask where to save it, or cancel it, and the page is not told.
The file is held in memory under an address of its own.
The address is let go of after ten seconds if the request was made, and at once if anything fails after it was made.

## How it is checked

The encoder, the symbol and the export are plain Rust, so they have native unit tests.
They are compared with two independent encoders, with python-qrcode as the oracle and segno as a cross-check.
segno pads a stream that is already on a codeword boundary with a whole extra zero codeword, which the standard does not, so its symbols hold other codewords than ours.
The generated golden vectors in `qr_core/golden.rs` and `qr_core/symbol_golden.rs` therefore come from python-qrcode.
`generate_golden.py` and `generate_symbol_golden.py` regenerate them, and say in their doc comments how each library counts.

`export/decode_exports.py` decodes the exported files with a reader that shares nothing with the encoder.
It draws each file from its own rectangles, at a whole number of pixels for each module with the quiet zone, and reads the drawing with ZXing-C++.
It also reads the mask from the format information in the picture, and checks that the modules are those of python-qrcode's symbol for the same text and mask.

```sh
QR_EXPORT_DIR=/some/dir cargo test -p svg-dom-graph-demo write_exports -- --ignored
python3 demo-app/src/qr_code/export/decode_exports.py /some/dir
```

ZXing-C++ reports no symbol at all for a QR code whose content is empty, even python-qrcode's own at every mask.
So for the empty text, it is the comparison with python-qrcode that shows the symbol is right.
pyzbar is used too, where the system has the libzbar library.

## Accessibility

Every control is a labelled button that works from the keyboard, and a dimmed button also leaves the tab order.
A live region announces each step, and what a chosen role outlines, in a sentence or two, and never reads out the 441 modules.
The matrix is also on the page as text, in a section that is closed until it is opened, with one line for each row.
The graph's nodes name themselves, and say which of their cells are not yet computed.
