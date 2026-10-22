# Browser Tests: Dragging and Connectors

These tests belong to the browser integration tests in `tests/drag/`; see [Browser Integration Tests](browser-tests.md) for how they are run.

- ordinary and scaled-coordinate dragging, proving the client-pixel-to-user-space conversion
- listener and scene lifetime, so a dropped `Scene` leaves no dangling drag handler
- multiple simultaneous pointers, so one pointer cannot drive or end another pointer's drag
- self-loop rejection
- foreign-scene node and edge ids
- unique marker ids across scenes sharing one `<svg>`
- drop-collision handling (`CollisionPolicy::PushClear`/`Allow`), and rejecting a second `make_draggable` call for the same node
- `DragOptions::bounds`:
  - clamping a drag to a rectangle at both edges
  - leaving an unbounded drag unconstrained
  - a node clamped to the edge remaining draggable afterward
  - rejecting a non-finite origin or a negative width/height
  - accepting a zero-width/height rectangle
  - a rejected `bounds` leaving the node not draggable at all, and
  - `CollisionPolicy::PushClear`'s own corrective push staying within `bounds` too, not just the pointermove that preceded it
- straight and elbow connector routing (`ConnectorType`), including corner-radius validation and live updates via `set_connector_type`, plus clamping to the available room and its automatic restoration once a drag gives a corner more room
- edge labels (`Scene::set_edge_label`):
  - the group in the content layer with its text, background, name and marker, taking no pointer events
  - the label being clear of its line, its arrowhead and both nodes, at rest and through a run of moves
  - a change of connector type placing the label beside the new route, clear of every run of an elbow
  - setting the same text again writing nothing, and a longer text growing the box and updating the name
  - removing and putting back a label, and refusing an empty or multi-line label and an edge of another scene
  - two connectors leaving one node keeping their labels apart through a run of drags, a later label giving way to an earlier one, and a later label in the way of an earlier one that moves being moved aside
  - zooming moving labels with the content without placing them again
- edge focus (`Scene::set_edge_focus`): the look, any number at once, nothing else changing, surviving a reroute, independence from a node's focus ring, and a repeat writing nothing
- per-node connector fixing points (`EdgeAnchors`), including:
  - zero-value rejection
  - matching the elbow connector's default midpoint anchor at one fixing point (this does not hold for a straight connector, whose unsnapped default is the continuous ray/boundary crossing)
  - a straight connector's own snap onto a fixing point and its exact round trip back to `None`'s boundary crossing
  - live reconfiguration via `set_edge_anchors` reaching every incident edge

- relationships (`Scene::add_edge`/`Scene::add_edge_with`):
  - both of a new edge's own endpoints get their own `aria-label`/`<title>` extended with an "Output to"/"Input from" clause naming the other
  - a source feeding more than one destination gets one "Output to" clause per edge, in the order each edge was added, never merged into a list
  - a binary operator's own two auto-wired inputs each append their own "Input from" clause, in the same order the operator constructor received them
  - a plain label node, which starts with no `aria-label` at all, has one seeded with its own visible text the first time a clause is appended
  - a relationship clause survives a later `Scene::set_selection` on the same node, rather than being truncated away
