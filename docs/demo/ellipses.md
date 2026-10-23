# Ellipses and circles

Shows `Scene::add_circle_node` and `Scene::add_ellipse_node`.
The panel is under "Basic Nodes".
See [Ellipse Nodes](../library/ellipse-nodes.md) for the rules.

## On the left

* **A** is a circle of radius 22. The letter fits, so the circle is as large as it was given.
* **Distance** is a circle of radius 20. The word does not fit, so the circle grows, and is still a circle.
* **Tall** is an ellipse that is taller than it is wide.
* **Wide ellipse with a long label** grows by one factor in both directions, so it keeps its proportions.
* **Rectangle** is an ordinary box, joined to the wide ellipse and to the clock ellipse, which shows that the two shapes mix.

The first three connectors are straight, and each lands on the curved outline on the line between the two centres.
The last two are elbows.
The elbow from the wide ellipse leaves by its south point, since the rectangle is below it.
The elbow from the rectangle is forced to leave by its east side, because a box as wide as this one leaves by its top when left to itself, and that would run the connector up through the wide ellipse.
Drag the rectangle so that its east side is further right than the clock ellipse's north point, and above it. The connector still leaves east, then turns south, west and south into the north point, and keeps a clear gap from the rectangle as it turns. An elbow never goes back into the node it is leaving.

## On the right

The **Clock** ellipse is wide, with radii 110 and 55.
The slider chooses a minute from 0 to 59, and the connector leaves the ellipse at that minute.
The circle it points to is placed in that direction, labelled with the minute.

* 0 is north, 15 is east, 30 is south and 45 is west. These are the exact ends of the axes.
* Minute 10 is 60° clockwise from north. On a wide ellipse that is **not** where 60° of the ellipse's own parameter would be.
* Minute 59 is just west of north.

Next to the slider, the panel says how many degrees the minute is clockwise from north.

## Dragging

Every node can be dragged, and the connectors follow.
The slider draws the scene again. It keeps the zoom and pan, and where each node was dragged to.
The circle that the clock connector points at is not kept, since the minute decides where it goes.
