# Huffman Encoding

Huffman encoding replaces each byte of a text with a bit sequence, so that the bytes that occur most often get the shortest codes.
It takes two passes over the text.
The first counts the bytes and builds the codes.
The second replaces every byte with its code and packs the result into whole bytes.

This demo currently shows the first pass.
Every value on screen is a real result, worked out in plain Rust.
The library never computes anything: the demo works out each value and then hands it to `svg-dom-graph` to draw.

The scene is large, so the gallery panel keeps the description and a button.
The button opens the scene in a window of its own, sized to fit it.

The text to encode is a text input node at the top of the diagram.
It is pre-filled with the first sentence of Jane Austen's *Pride and Prejudice*.
That is 116 bytes, with 26 distinct bytes, among them a capital letter, a comma and a full stop.
The text can be up to 128 bytes.
Editing it draws the diagram again, from the first step.
The keyboard focus and the caret go back into the new text box, so typing carries on.
A text that cannot be encoded, because it is empty or too long, says why in the line below the text box.
The diagram then keeps showing the last text that could be encoded.

## Symbols are bytes

A multi-byte UTF-8 character is counted as its separate bytes, as real Huffman encoders do.
A byte that is not printable ASCII is shown in hexadecimal.
Because there are at most 256 distinct bytes, the tree can never be deeper than 255 levels.
The 128-byte input limit keeps it much shallower: the deepest possible tree is 8 levels.

## The steps

The walk has one position for each of these steps, and "Next" and "Prev" move between them.

* **Counting.**<br>
  Each distinct byte becomes a tree of one node, weighted by how many times it occurs.
  The trees are shown lightest first.
* **Merging.**<br>
  A tree's weight is how often its bytes occur, and the trees are kept in order of weight, lightest on the left.
  Each step takes the two lightest trees and joins them under a new branch, whose weight is the sum of theirs.
  The lighter goes on the left and the other on the right, and every link is labelled `0` for left or `1` for right.
  The branch the step just made is ringed, together with the two trees it joined.
  Every merge puts one more link above every byte inside the two trees, so each of those bytes gets a code one bit longer.
  The rarest bytes are merged first, so they end up deepest, with the longest codes.
  The commonest are merged last, so they stay near the root and get the shortest.
  Where trees weigh the same, the one made first goes first, so the same text always gives the same tree.
  The step says so when a tie decided which tree was taken.
* **Code lengths.**<br>
  A byte's code length is the depth of its leaf.
  A text of one distinct byte has a tree of one node.
  Its depth of 0 would be an empty code, so it is given length 1.
* **Canonical codes.**<br>
  One step for each byte, in canonical order.

## Tree paths and canonical codes

The code table has two columns that look as though they should agree, and they do not.
"Tree path" is the route from the root to the byte's leaf.
"Code" is the byte's canonical code.
Both are valid, and both have the same length.

Canonical codes are handed out shortest first, and by byte value among bytes of one length.
Each is the previous code plus 1, shifted left by however many bits longer it is.
For the text `abracadabra`, the tree gives `b`, `c` and `d` the paths `110`, `100` and `101`.
The canonical codes are `100`, `101` and `110`.

A decoder can rebuild canonical codes from the lengths alone, so only the lengths ever need to be sent, and not the tree.
This is why the demo uses them.

## The code table

The table to the right of the tree has a row for each distinct byte, the commonest first.
Bytes that occur as often as each other are in byte order.
Its heading row has a fill of its own, so it reads as a heading.

## Following a code through the tree

As each canonical code is assigned, the route from the root down to that byte's leaf is marked with the library's secondary selection.
Every box on the route is marked, and so is the `0` or `1` label of every link.
Reading the labels down the route spells the byte's tree path, and the byte's row in the table is ringed.
This makes the difference between a tree path and a canonical code visible.
The route shows where the tree put the byte, and the code beside it shows what was assigned instead.

The boxes of the tree are data nodes, so that they can be marked this way.
A leaf holds its byte and count as text, a branch holds its weight, and a link's label holds a bit.
So each kind has its own colour.

## Reading the explanation

Each step has up to three paragraphs of explanation above the diagram, in a Markdown node.
The first says what the step did, with the merge number in bold.
The second says what that means for the codes.
The third says what is left to do, or notes that a tie decided the merge.
A byte that Markdown reads as formatting, such as `*` or `_`, is escaped in the explanation, so that it is always drawn as itself.

## Why the diagram is drawn again at every step

A node or edge cannot be removed once drawn, and every step changes which nodes exist.
So each step draws the whole diagram again, from the position alone.
It keeps the zoom and pan, and the keyboard focus on the toolbar button that was pressed.
Stepping backwards is the same work as stepping forwards.

The diagram is sized for the finished tree, so it does not change size as it is stepped.
The room for the explanation is kept at the height of its tallest step, for the same reason.
The levels of the tree are spaced far enough apart to hold the two elbow connectors of a link.
They also hold the arrowheads and the small box that labels the link.
A long text gives a wide tree, so the window is sized to fit it, and the zoom buttons scale it.
