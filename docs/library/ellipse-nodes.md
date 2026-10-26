# Ellipse Nodes

`Scene::add_ellipse_node(centre, radii, label)` adds a node drawn as an ellipse, with one line of text in the middle.
A circle is an ellipse with equal radii.
`Scene::add_circle_node(centre, radius, label)` adds one, and is exactly `add_ellipse_node` with `EllipseRadii::circle(radius)`.
`add_circle_node_with` takes the same `EllipseOptions` as `add_ellipse_node_with`.
A circle that grows to hold its label stays a circle.
Where this page says ellipse, a circle is meant too.
A bad radius is reported as `InvalidEllipseGeometry`, with both radii set to the one that was given.
The radii are semi-axes measured from the centre, and not diameters.
`rx` is horizontal and `ry` is vertical.
The ellipse is not rotated.

An ellipse is a plain node.
It can be made draggable, focused with `set_focus`, moved with `move_node`, and joined by connectors.
A rectangle and an ellipse can be joined to each other, in either direction.

## The label

The label is drawn at its normal size and is never made smaller.
If the label, with a clear margin all round its box, would cross the curved outline, both radii are made larger by the same factor.
A circle therefore stays a circle, and the ratio of the radii is kept.
The radii given are the smallest the ellipse will be.

The margin is 6 user units unless `EllipseOptions::with_inner_margin` says otherwise, and it must be finite and not negative.
The label is drawn at the usual size unless `EllipseOptions::with_font_size` says otherwise.
A size that is not finite and greater than zero is `Error::InvalidEllipseFontSize`.
A smaller label needs a smaller ellipse, so this is how a small circle holds one letter.
A single letter fits a circle of radius 20 with the default margin.
A margin of 8 would grow it to about 20.6.

The size is worked out once, when the node is added, from the label as the browser measures it.
It does not change if a web font loads later.

## Coordinates

`add_ellipse_node` takes the **centre**, and the centre stays where it was put when the label makes the ellipse grow.
Every other method that takes or gives a position uses the **top-left corner of the rectangle that encloses the ellipse**, as it finally is.
So `move_node` takes that corner, and `node_rect` returns that rectangle.
A caller that needs the final size, to place a node or to restore its position, reads it from `node_rect`.

## Connectors

Each end of an edge is worked out from its own shape and its own direction.
The other end is only a centre to aim at, so the two ends of an edge between two ellipses use opposite directions.

| Connector | At an ellipse |
|---|---|
| Straight | Where the line from the ellipse's centre to the other node's centre crosses the outline. It is never snapped to a cardinal point. |
| Elbow | By the nearest of north, east, south and west, by angle. An exact tie goes clockwise: north-east is east, south-east is south, south-west is west and north-west is north. |
| Forced side | At that side's cardinal point, for a straight connector as well as an elbow. |
| Clock position | At that minute, see below. |

The cardinal for an elbow does not depend on the shape of the ellipse.
That is the difference from a rectangle, whose exit side depends on its aspect ratio.
A wide ellipse still leaves by east for a target 40° above east, where a rectangle of the same proportions would leave by north.

The rectangle end of a mixed edge keeps the rule it always had.
The two ends need not leave and arrive on matching sides.
An elbow can leave an ellipse by east and arrive at a rectangle through its top.

If the two centres are the same point there is no direction, and an ellipse end is its east point.

### Clock positions

`ConnectorOptions::with_from_clock` and `with_to_clock` put an end at a chosen minute round the ellipse.
`ClockPosition` is a whole number from 0 to 59, read clockwise from north.
The constants `NORTH`, `EAST`, `SOUTH` and `WEST` are 0, 15, 30 and 45.
`ClockPosition::try_from(60)` is an error, and a minute is never wrapped.

A minute is a **direction from the centre**.
The end is where the ray in that direction crosses the outline.
It is not the point at that angle of the ellipse's own parameter, which on a long thin ellipse is somewhere else entirely.

* A straight connector can use any minute.
* An elbow connector can use only 0, 15, 30 and 45, since it must leave an ellipse horizontally or vertically.
* The clock position stays with the edge.
  It does not change when a node moves, and nothing can change it afterwards.

### What is refused

These are checked before anything is drawn or recorded, so a refused call leaves the scene as it was.

| Mistake | Error |
|---|---|
| A clock position on an end that is a rectangle | `InvalidEllipseEndpoint { problem: ClockOnRectangle }` |
| A clock position and a forced side for the same end | `InvalidEllipseEndpoint { problem: ClockWithSide }` |
| A position along a side, as a fraction, for an ellipse | `InvalidEllipseEndpoint { problem: FractionOnEllipse }` |
| An elbow with any minute but 0, 15, 30 or 45 | `InvalidEllipseElbowClock` |
| `set_connector_type` to an elbow on an edge whose clock position is not a cardinal | `InvalidEllipseElbowClock`, and the edge is unchanged |
| `set_edge_anchors` on an ellipse | `EdgeAnchorsUnsupportedForEllipse` |
| A radius that is not finite and greater than zero, or a centre that is not finite | `InvalidEllipseGeometry` |
| Radii or a rectangle that the label makes too large to represent | `InvalidEllipseGeometry` |
| An inner margin that is negative or not finite | `InvalidEllipseMargin` |

`InvalidEllipseEndpoint` says which end with an `EdgeEnd` and what is wrong with an `EndpointProblem`, so a caller can match on both.
An ellipse is a plain node, so the calls that refuse a plain node, such as `set_data_values`, refuse it too.

## Limits

The scene keeps the rectangle that encloses an ellipse for dragging, view bounds and collision tests.
Those are conservative at the curved corners: two ellipses can count as overlapping when their curves do not touch.
Edge labels treat an ellipse the same way.
So a label may stand a little further from the curve than it needs to, and never over it.

Only the painted ellipse and its label take pointer events.
A press in a corner of the enclosing rectangle that is outside the curve goes to whatever is behind the ellipse.
It never starts a drag of the ellipse.
If background panning is active, the pointer pans the view.
Panning depends on `set_pan_mode` and on whether a toolbar is shown.
See [Pan and Wheel Zoom](pan-and-wheel-zoom.md).
If it is not, the press does nothing.

There is no setter for the label or the radii after the node is added.
There are no rotated ellipses.
The application must limit any untrusted label length, radius or number of edges at one node, as for every other node.
See [Security](../security.md).

The geometry is in `src/geometry/ellipse/`, which touches no DOM and is tested with `cargo test`.
