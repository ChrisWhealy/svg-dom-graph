# Fixing points

![Fixing Points Straight](../../img/demo-fixing-points%201.png)

Demonstrates the idea of `EdgeAnchors`: a slider from `0` to `5` sets how many evenly spaced connector fixing points exist along a node's edges.
This is applicable for both straight and elbow connectors.

![Fixing Points Elbow](../../img/demo-fixing-points%202.png)

`0` maps to `None`, which drops back to the default arrangement where a connector automatically anchors to the node's centre.
An important distinction here is that the point along the edge to which the connector anchors will vary based on the angle between the centres of the nodes.

`1` `EdgeAnchors` locks the connector(s) to the edge's midpoint; therefore, no matter what the angle between the two nodes, the connector will always anchor to the midpoint.

`2` or more `EdgeAnchors` snap the connector(s) to the nearest of that many candidate anchor points.

In this demo, the number of visible children always matches the slider value, down to a minimum of one.
Raising it reveals more children, each settling on its own fixing point in the parent node's edge.
Lowering it hides them again.
