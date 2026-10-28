# Plot Nodes

A plot node is a graph of values, drawn as a node.
It has a plane, two axes, ticks, a grid and a background.
Lines, dots and labels are drawn on it by value, and a click on it reports the values at the place that was clicked.

Like every other node, a plot does no arithmetic.
The caller works out what is drawn.
It is a node like any other: a connector can be anchored to it, it takes the focus ring, and it has an accessible name.
It is not made draggable, since a plot is something to read and to click.

## Using it

```rust,ignore
let config = PlotConfig::new(
    Size::new(660.0, 340.0),
    PlotAxis::linear(0.0, 1.0).with_title("time"),
    PlotAxis::linear(-3.0, 3.0).with_major_interval(1.0).with_position(AxisPosition::AtMinimum),
);
let plot = scene.add_plot_node(Point::new(20.0, 20.0), config)?;
scene.set_plot_items(plot, &[PlotLine::new(points, "#e5534b").into()])?;
scene.on_plot_click(plot, |scene, plot, click| { /* click.x, click.y */ })?;
```

| Method | What it does |
|---|---|
| `add_plot_node` and `add_plot_node_with` | Draw the box, the plotted area, the grid, the axes, the ticks, the titles and the legend |
| `set_plot_items` | Draws lines, dots and labels, in place of whatever was drawn before |
| `set_plot_description` | Changes what assistive technology reads |
| `plot_area` | The plotted area in the scene's own space, for laying out something against it |
| `plot_point_to_scene` | Where a value is in the scene's own space |
| `on_plot_click` | Reports a click inside the plotted area as the values of the axes |

## The configuration

`PlotConfig::new` takes the size of the whole node and the two axes.
Everything else has a default, and a `with_` method changes it.

### The plane

`PlotPlane::Real` is the default.
It is a chart of `y` against `x`, with the tick labels in the margin.
`PlotPlane::Complex` is an Argand diagram.
Its horizontal axis is the real part and its vertical axis is the imaginary part.

A complex plot differs from a real one only in what it assumes when a setting is left out.

| Left out | Real | Complex |
|---|---|---|
| Axis titles | none | `Re` and `Im` |
| Tick labels on the vertical axis | the number | the number and `i` |
| Where the tick labels are | in the margin | beside the axes |
| The accessible name | "Real plane plot: …" | "Complex plane plot: …" |

Points are `(x, y)` pairs either way, and a `Complex` converts to one.
A click on a complex plot gives `PlotClick::complex()`.

### The axes

A `PlotAxis` has these settings.

| Setting | Default | What it does |
|---|---|---|
| `min` and `max` | given | The range. They need not be the same on the two axes |
| `scale` | `Linear` | `PlotAxis::logarithmic` makes it logarithmic, and both ends must then be above zero |
| `title` | none, or the plane's | Text that names the axis |
| `major_interval` | a round one, about six | A difference of value, or for a logarithmic axis a number of decades |
| `minor` | none, or `Decade` on a logarithmic axis | `MinorTicks::Divisions(n)` splits each major interval, and `Decade` ticks 2 to 9 times each power of ten |
| `tick_labels` | `Auto` | `None` writes none, and `Custom` puts a tick and a name at each value given |
| `tick_suffix` | none, or the plane's | Written after each automatic label |
| `position` | `AtZero` | Where the axis line is drawn: a value on the other axis, or one of its edges |
| `arrow` | off | An arrowhead at the greater end of the axis line |

`Custom` tick labels replace the automatic ticks and the interval.
They are how a tick is put at a value that has a name, such as a quarter of a window.
A custom tick outside the axis is not drawn.

The horizontal axis line is drawn at a height, and the vertical one at a position along the axis.
So the `position` of the horizontal axis is a value of `y`, and the `position` of the vertical axis is a value of `x`.
A line that would fall outside the plot is drawn at the nearest edge.
Zero on a logarithmic axis is the edge it starts at.

### The background and the grid

The background is a hex colour, `#rgb` or `#rrggbb`.
It is the same pale blue as an ordinary node unless it is set.
It has to be hex because the grid is worked out from it.

A major grid line is the background made 14% darker, and a minor one is made 7% darker, unless a colour is given.
So a grid is seen without competing with what is plotted on it.
`PlotGrid` turns either kind off, sets a colour, and sets a width.

### The margins, the legend and the description

The margins are only as big as what they hold.
A real plot has room on the left and below for its tick labels and titles, and above for a legend.
A complex plot has the same small margin all round.
`with_insets` sets them by hand, and `with_label_placement` moves the tick labels.

A legend is a row of entries above the plot, each with a sample and a name.
A sample is a line, which can be dashed, or a dot.

The description is what assistive technology reads.
Without one, a plot is named from its axes.
It does not say what is plotted, so a caller that changes that should say so with `set_plot_description`, and in text.

## What is drawn on a plot

`set_plot_items` takes a list of items, drawn in order, so a later one is over an earlier one.

| Item | What it is |
|---|---|
| `PlotLine` | Straight segments through points, with a colour, a width and a dash pattern |
| `PlotPoints` | Dots, with a fill, a radius and an outline |
| `PlotLabel` | A line of text at a point, moved by an offset in user units, turned, and on a halo of the background |

Items are placed by the value they stand for.
A label's offset is in user units and not in values.
So a label stays the same distance from its point however the axes are scaled.

* A line is cut where it leaves the plotted area.
* A dot or a label whose point is outside it is not drawn.
* A point that cannot be placed is skipped, and breaks a line there.
  So a gap in the data is a gap in the line.
  That is a point that is not finite, or is zero or less on a logarithmic axis.

Only what is plotted changes.
The grid, the axes and the labels stay as they are.
So `set_plot_items` is the call to make as often as a slider moves.
The new items are drawn out of the way and then put in place of the old, so a failure leaves what was there before.

## Clicks

`on_plot_click` calls its handler with the scene, the node's id and a `PlotClick` holding the values of the axes where it was clicked.
A click in the margin is not reported.
The plotted area takes the crosshair cursor, so that it can be seen to be clickable.

The values are found through the zoom and the pan of the scene, so a click is right in a zoomed scene.
On a logarithmic axis, the value is the one that the position stands for.

A click is a mouse or touch gesture, and cannot be made from the keyboard.
A caller that reacts to it should offer another way to the same result, such as a slider.

The handler is called with nothing of the scene borrowed, so it may draw on the plot it was called from.
It is held by a listener that holds only a `Weak` reference to the scene, so it does not keep the scene alive.
A listener cannot be removed, so a second call for the same node returns `Error::AlreadyHasClickHandler`.

## Errors

A configuration that cannot be drawn is refused with `Error::InvalidPlotConfig`, which holds a `PlotConfigError`.
It says which axis is at fault where that applies.
The checks are made before anything is drawn, so a rejected call leaves the scene as it was.

* A size that is not finite, or margins that leave no plotted area.
* An axis that does not run from a finite value to a greater one.
* A logarithmic axis that starts at zero or below.
* An interval that is not above zero, or that would make more than 400 ticks.
* Minor ticks that do not suit the scale: `Divisions` on a logarithmic axis, or `Decade` on a linear one.
* A custom tick that is not a usable value.
* A background that is not a hex colour.
* A width that is not above zero, and an empty description.

A plot method given a node of another kind returns `Error::NotAPlot`.
