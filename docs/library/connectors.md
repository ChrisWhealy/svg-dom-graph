# Connectors

A connector is a directed, arrow-tipped line between two nodes, drawn either as a straight line or as an elbow.

- `add_edge`/`add_edge_with` draw a directed, arrow-tipped connector between two nodes.

   Both endpoints' `aria-label` and `<title>` values are extended with an "Output to"/"Input from" relationship clause.
   So which node feeds which is never conveyed by the connector's own `<path>` alone.

   `set_connector_type` can be used to switch an existing connector between straight and elbowed, with configurable corner rounding — see `ConnectorOptions` and `ConnectorType`.

   `set_edge_anchors` can be used to reconfigure a node's own fixing points live.

   `ConnectorOptions` can also force the side of a node that a connector leaves from, or arrives at, with `with_from_side` and `with_to_side`.
   `with_from_position` and `with_to_position` then pin the end a fraction of the way along that side, so a connector can leave a node exactly below one of its own columns.
   A binary or arithmetic operator anchors its own two incoming connectors itself, so `to_side` and `to_position` have no effect on those.

## Edge labels

`Scene::set_edge_label(edge, Some("8"))` gives an edge a label: a small box with a weight or a name in it, drawn beside its connector.
`None` takes it away.

A label is not a node.
It has no connectors and cannot be dragged, and the library never looks at what it says, so a weight is not checked and a caller who adds them up does so itself.
It is a group of its own in the same layer as the connector, so zooming and panning move the two together without either being routed again.
A label takes no pointer events, so it never gets in the way of dragging a node or the background.

It has a background, and its accessible name says which edge it is on, as in `Edge from A to B: 8`.
Its group carries `data-svg-dom-graph-edge-label`, whose value is the index of the edge, so a test or a stylesheet can find one without relying on the order of the elements.
Its text is one line with no control character, and it is not empty, or the call is `Error::InvalidEdgeLabel`.

### Where a label goes

The rules are in `src/geometry/label`, which touches no DOM, so a label can be placed on every frame of a drag without measuring anything.
The text is measured once, when it is set.

- A label goes on one of at most eight candidates: up to four anchors along the connector's straight runs, each on both sides of its run.
  The longest run is preferred, and of runs as long, the later one.
  It is above a run that is closer to horizontal, and to the right of one that is closer to vertical.
  A rounded corner, the arrowhead and the start of the line are not used.
- A candidate is clear if it overlaps no node, no label of an edge added earlier, no part of the arrowhead and no other run of its own connector.
  A clear candidate wins, by its soft score: how far its anchor is from the first candidate's, in the scene's own units, and 16 more for the far side of a run.
  If none is clear, the one that overlaps least wins. The weight is never hidden.
- A label keeps the candidate it had while that is still clear, so it does not jump about during a drag.
  It only leaves it if another clear candidate scores at least 12 points better.
  Eleven is not enough.
  A candidate that is no longer clear is left at once.
  Only the candidate's ordinal is kept, and its position is worked out again from the new route.

### When a label is placed again

Whenever its connector is routed again: when either node is dragged or moved with `move_node`, when `set_connector_type` is called, and when `set_edge_anchors` is called for either node.
The path and the label are written from the one route.

Labels are ranked by the order their edges were added in, and an earlier label has priority.
After a connector is routed again, its label is placed first, keeping clear of every label of higher priority.
A label of lower priority that is now in the way of one that moved is then placed again, once.
That is the whole of it: there are two passes and never more, so the work is bounded and labels cannot chase one another.
A graph too crowded for every label to be clear is allowed to overlap, as documented above.

A label that could not be moved because a write failed is where it was, and the error is returned.
It is placed again the next time its connector is routed, or when `set_edge_label` is called again for the edge.

## Edge focus

`Scene::set_edge_focus(edge, true)` draws an edge's connector in a deep blue, more than twice as thick, with `aria-current="true"`.
`false` puts it back.

Focus is a state of each edge on its own, so any number of edges can be focused at once, which is how a whole path is shown.
It is never "the current edge", and focusing one does not unfocus another.
It changes the look of the line and nothing else: not the route, the endpoints, the label or the nodes.
It stays through a reroute.
It does not interact with `set_focus`, which rings a node: the two draw on different elements.
Colour is not all that tells a focused edge apart, since it is also thicker, and the page can say in words what the state means.
Setting the state an edge is already in writes nothing.

## Sides

- `Side` (`North`, `South`, `East`, `West`) names one side of a rectangle.
  It is used both for the side of a box a connector leaves from, and for the edge of a scene its toolbar is fixed to.
