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

## Sides

- `Side` (`North`, `South`, `East`, `West`) names one side of a rectangle.
  It is used both for the side of a box a connector leaves from, and for the edge of a scene its toolbar is fixed to.
