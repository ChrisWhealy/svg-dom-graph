# Connector Routing

![Connector Routing Straight](../../img/demo-connector-routing%201.png)

Shows two boxes and radio buttons that toggle between straight and elbow connectors.

When the elbow connector is selected, the slider dynamically controls the elbow's corner radius up to a maximum that fits the available space.
This maximum is calculated as half the available vertical/horizontal space between the two boxes.

![Connector Routing Elbow](../../img/demo-connector-routing%202.png)

Drag a box to see the connector reroute.

The slider's own `max` value dynamically tracks how much rounding room the nodes' current positions actually allow.
See the doc comment for `build_elbow_demo` in `demo-app/src/elbow.rs` for exactly how this works.

Drag the boxes close together and watch the corner radius slider itself get pulled down, not just the rendered corner.
Drag them apart again, and the sliders' maximum value rises with it.
