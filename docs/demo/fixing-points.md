# Fixing points

![Fixing Points Straight](../../img/demo-fixing-points%201.png)

Demonstrates the idea of `EdgeAnchors`: a slider in the scene, from `0` to `5`, sets how many evenly spaced connector fixing points exist along a node's edges.
This is applicable for both straight and elbow connectors.

![Fixing Points Elbow](../../img/demo-fixing-points%202.png)

`0` maps to `None`, which drops back to the default arrangement where a connector automatically anchors to the node's centre.
The point along the edge where the connector anchors varies with the angle between the centres of the nodes.

`1` `EdgeAnchors` locks the connector(s) to the edge's midpoint.
So whatever the angle between the two nodes, the connector always anchors to the midpoint.

`2` or more `EdgeAnchors` snap the connector(s) to the nearest of that many candidate anchor points.

In this demo, the number of visible children always matches the slider value, down to a minimum of one.
Raising it reveals more children, each settling on its own fixing point in the parent node's edge.
Lowering it hides them again.
The slider is in a strip under the nodes, which they cannot be dragged into.
The scene is drawn again for every move of the slider, but the slider is not, so a drag of it and the keyboard focus carry on across the scenes.
