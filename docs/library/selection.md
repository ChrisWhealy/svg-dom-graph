# Selection and Marking Cells

A data node can mark its own cells in four independent ways.
They are a selection, secondary cells, cells that are not reached yet, and a ring around the whole node.
A selection toolbar steps one data node through its cells.

## Marking cells

- `set_selection` highlights a cell, row, or column of a `DataNodeContent` grid.
  It exposes the current selection through colour, stroke width, and its own `aria-label`: see `Selection`.
- `set_secondary_selection` marks further cells that a step derives from the current selection.
   For example the other two lanes read by the Keccak `Chi` function.
   They are given a teal background colour and a dashed outline, and their own `aria-label` text.
   These cells are independent of `Selection`, so the selection toolbar is unaffected.
   This also updates the node's own accessible name; it does not create a live-region announcement.

- `set_unreached_cells` marks the cells of a data node that have not been computed yet, and draws them faint.
  A Markdown node is one value, so `[0]` dims all of its text.
  Its accessible name gains ", not yet computed".
  A walk through an algorithm fills its arrays step by step, and a placeholder zero looks exactly like a computed zero.
  The mark is independent of `Selection` and of the secondary cells.
  It also adds ", not yet computed: cells 16 to 63" to the node's accessible name.

- `set_focus` rings, or removes the ring from, the outer box of any node, as a whole.
  A host driving its own multi-node walk uses it to mark whichever nodes the current step puts in focus.

## Selection toolbar

- `show_selection_toolbar`/`hide_selection_toolbar` add and remove a second button bar, for stepping through the cells of one data node: `Prev`, `Next` and `Restart`.
  `has_selection_toolbar` reports whether one is shown.
  See `SelectionToolbarOptions` and `SelectionStride`.

    The library owns the node's `Selection`.
    The host supplies an `on_step` callback, which is told about each change as a `SelectionTransition`.
    Its `from` and `to` are each a flat position, or `None` for unstarted.
   There is no cursor stored anywhere else: each button reads the node's current selection, steps it, and writes the next one back.
   `Prev` and `Restart` are disabled while the node is unstarted, and `Next` is disabled at the last cell.

   `SelectionToolbarOptions::stride` adds two further buttons, `Prev <label>` and `Next <label>`, that move a whole group of cells at a time.
   A button is as wide as its label's rendered text needs, so a stride label may be in any script.

   `Restart` and `Prev` from the first cell both leave the node unstarted, and `on_step` cannot tell them apart.
   Showing the bar always resets the node to unstarted.
   So a host that redraws a diagram on each step re-applies its position with `set_selection` straight afterwards.
