# Butterfly

A butterfly is the building block of a fast Fourier transform.
It reads two values `a` and `b` and a twiddle factor `w`.
It produces the two values `a + w·b` and `a − w·b`.

`Scene::add_butterfly_node` draws one as a multiplication, an addition and a subtraction inside a labelled frame, with three input ports on its west edge.
The three operator nodes are ordinary operator nodes, and the frame is a plain node.
Together they behave as a single unit.

## Using it

```rust,ignore
let operands = ButterflyOperands::new(a, b, twiddle);
let results = ButterflyResults::new(product, sum, difference);
let butterfly = scene.add_butterfly_node(Point::new(300.0, 100.0), operands, results)?;
```

* `ButterflyOperands` names three existing data nodes, each holding one complex number.
* `ButterflyResults` holds the three values to show.
  The caller works them out, because `svg-dom-graph` never does arithmetic.
  `product` is `w·b`, `sum` is `a + w·b`, and `difference` is `a − w·b`.
* The call returns a `Butterfly` that names `frame`, `port_a`, `port_b`, `port_w`, `product`, `sum` and `difference`.

## What it draws

The layout is the usual signal-flow diagram of a butterfly.

* The addition is in the first row and the second column.
* The multiplication is in the second row and the first column.
* The subtraction is in the second row and the second column.
  The multiplication stands where the middle of its east side is level with the subtraction's second input.
  So the product runs horizontally into the subtraction.
* The multiplication reads `b` and the twiddle factor `w`.
* The addition and the subtraction each read `a` and the product.
  The subtraction is not commutative, so it carries the usual "L" and "R" port markers.
* Every connector is straight except two, which are elbows.
  The product runs up out of the multiplication's north edge and into the addition.
  `A` runs across into the subtraction's "L" input.
  It turns in the gap just before the subtraction, so it goes round the multiplication and not through it.
  Those two cross once, and no other two do.
* The addition and the subtraction output from the middle of their east edges.
* The frame is sized to hold the three operator nodes, and is drawn behind them.
  Its title, "Butterfly", is at its top left corner.

### The inputs

The frame has three input ports on its west edge, labelled `A`, `B` and `W` from the top down.
Each port is a small circle node centred on the edge, so it is a fixing point that connectors end at and start from.

* A connector runs from `a` to port `A`, from `b` to port `B`, and from the twiddle factor to port `W`.
* Inside the frame, `A` feeds the addition and the subtraction, which reads it as its "L" input.
* `B` and `W` feed the multiplication.
* The product feeds the addition, and the subtraction as its "R" input.

That is nine connectors in all.
Each operator node takes both of its inputs on its own west side, a quarter and three quarters of the way down it.
That is `NodeOptions::with_input_side(Some(Side::West))`.
The operand that lies higher takes the higher point, so the two connectors do not cross at the node.
The node has no fixing points configured, so its output leaves from the middle of its east side.

Each port is as small as holds its letter.
The letter is drawn at `EllipseOptions::with_font_size(Some(9.0))`, since a circle must grow to hold a label of the usual size.

## Moving it

The frame is already draggable.
Dragging it carries the three ports and the three operator nodes by the same amount, and their connectors follow.
`Scene::move_butterfly` does the same from code.
Do not call `make_draggable` on the frame, because it is already draggable.

The ports and the operator nodes are not draggable on their own.
They keep their place in the frame.

Dropping the frame never pushes it aside.
The frame lies over its own operator nodes, so the usual push clear of an overlapped node would always push it away from them.
Other nodes that it is dropped on are not checked either.

## Changing the values

`Scene::set_data_values` replaces the result of `product`, `sum` or `difference`.
The butterfly keeps its size and place.

## What it refuses

Every check runs before anything is drawn.

* `Error::ButterflyNotComplex` if an operand or a result is not one complex number.
* `Error::OperatorTypeMismatch` if the operands and results mix `Complex32` and `Complex64`.
* `Error::DuplicateOperands` if the twiddle factor and `b` are the same node.
* `Error::UnknownNode` or `Error::OperandNotData` if an operand is not a data node of this scene.
* `Error::InvalidNodeGeometry` if the position is not finite.

A write that fails while drawing removes everything the call added.
So a failed call leaves the scene as it was.

## Limits

* A butterfly takes complex numbers only.
* The frame's label is fixed.
* The library has no way to remove a butterfly once it is drawn.
* If moving one operator node fails during a drag, the drag cannot report it.
  The next report of the drag moves that node again by the whole distance it is behind.
