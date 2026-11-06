# Phase 3: the accessibility tree

`layer-c/accessibility.mjs` reads Chrome's own tree (`Accessibility.getFullAXTree`) and holds it to the model's edges.
It runs last, in a tab of its own for each scale, and never inside a timing.
The reason is that switching the tree on makes Blink keep it up to date on every change.
In each tab it looks before and after one drag, one pan and one wheel zoom:

- **Roles, names and relationships.** A node with an edge is a `group` whose name is its key and then one `Output to` or `Input from` clause for each edge.
  The clauses are the model's edges as a multiset, in whatever order.
  The graph view is one `application` named `Graph view, zoom N%`, and N follows the view's scale: 50, 100 and 200 at the start, and 125 after a wheel notch at 100%.
  Every clause names a node that exists.
- **Bounds.** At most 40 nodes are inspected, taken evenly from those that have an edge.
  The tree may hold no more than eight nodes for each node of the scene and sixty more.
  The tree held 99 nodes for 24 and 323 for 80.
- **Focus.** After Tab the one focused node that is not the page is the graph view.
  It is the element the DOM calls active, compared by the browser's own node identity and not by name.
  After the input, focus is still one element in both views of it.
- **Consistency.** After the input the roles and names of the inspected nodes are what they were, since moving a node changes no relationship.
- **Teardown.** With the scene gone, the tree holds no graph view and no group.

In the recorded runs this was 56 checks on the small manifest and 276 on the medium one, all passing, and each run states its counts as `accessibility.*` metrics.
They are counted among the run's checkpoints, under the section `accessibility`, so a failure makes the run `invalid_incorrect`.
The self-test removes the relationship clauses from one node's name and the check names it.

What it does not cover: a node with no edge is not exposed as a group at all, which is what Chrome reports.
The check records the count and does not assert it.
Only the graph view takes focus, since nodes carry no `tabindex`.
The focus check has no negative control of its own, because there is no natural way to make the tree and the DOM disagree.
The names are the library's strings, and no screen reader was run.
