# Text Nodes

This demo shows the two nodes that hold text: a Markdown node, and a text input node.
Both are described in [Text Nodes](../library/text-nodes.md).

## The diagram

* **The Markdown node.**<br>
  It holds a document that uses every construct the node understands: three heading levels, bold, italic and code runs, wrapped paragraphs, bulleted and numbered lists with a hanging indent, a hard break and escapes.
  It also holds a word that is longer than a whole line, which is split.
  The node's height is whatever the text needs.
* **The text input node.**<br>
  It sits below the Markdown node, placed from the Markdown node's own `node_rect`.
  Each change to its text calls back, and the demo writes the new text to the line above the diagram.
* **The connector.**<br>
  Both nodes can be dragged, and the connector between them follows, as for any other node.
  A pointer that goes down on the text box goes to the text box, so it can be clicked into and typed in even though the node is draggable.
  Dragging the label or the padding drags the node.
