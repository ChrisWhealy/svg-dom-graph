# Butterfly

Demonstrates `Scene::add_butterfly_node`, the building block of a fast Fourier transform.
See [Butterfly](../library/butterfly.md) for the library side.

A butterfly reads two values `a` and `b` and a twiddle factor `w`.
It produces `a + w·b` and `a − w·b`.
The panel is in the "Operators" group.

* **The diagram.**<br>
  The inputs `a`, `b` and `w` are in a column on the left.
  The butterfly is in the middle: a multiplication, an addition and a subtraction inside one frame.
  The two outputs are on the right, level with the nodes that produce them.
* **The layout.**<br>
  It is the usual signal-flow diagram.
  The addition is at the top right, and the multiplication and the subtraction are in the row below it.
  Every connector is straight except two elbows: the product into the addition, and `a` into the subtraction.
  Those two cross once.
  The elbow from `a` turns in the gap before the subtraction, so it does not run through the multiplication.
  The product runs horizontally into the subtraction.
  The outputs leave from the middle of the east edges of the addition and the subtraction.
  The frame has small input circles `A`, `B` and `W` on its west edge.
  They are fixing points: `a`, `b` and `w` are connected to them, and the connectors inside the frame start from them.
* **The inputs.**<br>
  `a` is `2 + i` and `b` is `1 + 2i`, and neither changes.
* **The slider.**<br>
  It steps `k` from 0 to 7.
  The twiddle factor `w` is `e^(−2πik/8)`, a twiddle factor of an eight-point transform.
  So each step turns the product by one more eighth of a turn.
  A slider has the keyboard too: Tab to it, and use the arrow keys, Home and End.
* **The arithmetic.**<br>
  The demo works out `w·b`, `a + w·b` and `a − w·b` in plain Rust.
  It gives each node its new number with `Scene::set_data_values`.
  The library never does the arithmetic.
* **Dragging.**<br>
  Drag the frame, and the three operator nodes go with it.
  Their connectors follow.
  The frame lands where the pointer lets go, even over the outputs.
* **Sizing.**<br>
  `set_data_values` never resizes a node.
  So the demo first draws each node for the widest number the slider can give it, and then shows the starting number.
