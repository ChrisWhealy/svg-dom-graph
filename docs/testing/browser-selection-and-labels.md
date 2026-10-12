# Browser Tests: Selection and Labels

These tests belong to the browser integration tests in `tests/drag/`; see [Browser Integration Tests](browser-tests.md) for how they are run.

- marking cells (`Scene::set_secondary_selection`, `set_unreached_cells`, `set_focus`):
  - secondary cells drawn with a teal fill and a dashed outline, independent of the selection
  - unreached cells drawn faint, with every other mark winning over it, and an accessible-name clause such as "not yet computed: cells 16 to 63"
  - a whole node ringed on any node kind, and more than one ringed at once
  - a repeated request not being skipped as a no-op after an earlier one failed partway through
  - restyling a cell writing only the attributes that changed
- row labels (`DataNodeContent::with_labels`, `with_labelling_style`, `LabellingStyle`):
  - off by default, and never drawn for a single value
  - numeric and alphabetic labels reading as expected, including past `z`
  - the label sitting left of its cell at the data text's own size, and costing a node a little width and no height
  - a label being hidden from assistive technology and quoted in the cell's accessible name, including after `set_data_values`
  - a `Text` label naming its row: it is drawn beside that row, introduced as a "row label" in the name, and left out of both when the row has none
  - a node's width following the *rendered* width of its widest label, whatever the script, not its character count
  - a label for a row that has no cell taking no room, while a node asked for three rows keeps its height when only two have values

- cell selection (`Selection`, `Scene::set_selection`):
  - `Selection::Cell`/`Row`/`Column` recolouring a single-value node, a one-dimensional grid and a two-dimensional grid's row/column band plus its own optional focused cell
  - `Selection::None` fully resetting a previously selected node, not just the cells a prior call touched
  - the focused cell's own thicker stroke width, distinguishing it from a banded cell and from an unselected one by more than colour alone
  - the node's own `aria-label` describing the current selection as text, exposing it to assistive technology as well as through the visual properties of colour and stroke width
  - rejecting a `Selection` that names a plain label node, a foreign-scene id, or a cell/row/column index out of range for the node's own actual value count or grid shape, all before recolouring anything
- the selection toolbar (`Scene::show_selection_toolbar`):
  - showing and hiding it, and rejecting a node that is not a data node, and invalid options
  - resetting the node to unstarted when it is shown
  - stepping by click and by keyboard, and a disabled button doing nothing
  - the stride buttons moving a whole group of cells, being disabled at the ends, and being named for their label
  - a stride button being at least as wide as its label's rendered text, whatever the script
  - `on_step` reentering the same `Scene`, and its lifetime once the toolbar is hidden
