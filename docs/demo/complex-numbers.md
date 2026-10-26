# Complex Numbers

Demonstrates `NodeValues::Complex64` and every `ComplexNumberFormat`, with the complex plane drawn as Argand diagrams.
The real part of a number runs along the horizontal axis, and the imaginary part runs up the vertical one.
So every number is a point, and every form is a different way of saying where it is.
The demo draws the diagrams as SVG in the scene's own group, so they zoom and pan with the nodes.
Each diagram has one description for assistive technology.

* **One number, seven ways.**<br>
  A diagram of `3 + 4i`, whose modulus is exactly 5, with its modulus and angle written on it.
  Its real part and imaginary part form the two legs of the right triangle that the number makes with the real axis.
  The **adjacent** leg runs along the real axis from the origin, in green.
  The **opposite** leg runs up or down from there to the number, in purple.
  They are the colours and the legs of the picture in a rectangular cell, and each leg carries its own label, `a` or `b`.
* **The sliders.**<br>
  A slider runs along the bottom of the diagram and sets the real part.
  A second runs up its left side and sets the imaginary part.
  Each is **exactly as long as the axis it lies along**, from -5 to 5.
  So its thumb sits directly under, or beside, the point of the axis whose value it stands for.
* **How the sliders match their axes.**<br>
  The slider is one thumb wider than the axis.
  A thumb's centre travels from half a thumb in at one end to half a thumb in at the other.
  The thumb has the same width in every browser, and the track is exactly that much shorter than the input.
  The demo puts that style in the page itself, with the width that `THUMB` says, and not in `style.css`.
  The length and the style that makes it true must come from one place.
  A stylesheet that a browser kept from an earlier visit cannot then change them.
  The geometry was measured in Chrome and Firefox, at the default zoom and zoomed in three times.
  The input is always the axis plus one thumb, scaled.
* **The vertical slider.**<br>
  It is the same element turned a quarter turn anticlockwise, so its minimum is at the bottom.
  Both sliders are range inputs inside the scene, an HTML `<input>` in a `<foreignObject>`, so they zoom and pan with the diagram.
  They work from the keyboard, and each has a name and a value read as a number to one place.
* **The seven nodes.**<br>
  As either slider moves, the demo draws the diagram again.
  It also gives each of the seven nodes the new number with `Scene::set_data_values`.
  That call rewrites a node's cell, its picture and its exponent, and nothing else.
  The forms are rectangular, polar in degrees, trigonometric, polar in radians, exponential, log-polar and matrix.
* **Sizing the nodes.**<br>
  `set_data_values` never resizes a node.
  So the demo first draws each node for whichever reachable number makes its form widest.
  The candidates are the corners, a very small number whose logarithm is long, and zero.
  The demo measures each candidate in the page, and then gives the node `3 + 4i`.
  No number the sliders reach can be wider than its cell.
* **The roots of unity.**<br>
  The points of the unit circle that give 1 when raised to the power `n`.
  They are the twiddle factors of an `n`-point Fourier transform.
  A slider above the diagram sets `n` from 1 to 20.
  The upper limit is arbitrary, and is `MOST` in `roots.rs`.
* **Changing `n`.**<br>
  A node cannot receive more numbers than it was drawn with, and the library cannot remove a node.
  So a change of `n` draws the diagram and its two nodes, rectangular and polar, again from nothing.
  The new scene keeps the zoom and pan of the old one.
  The slider is an ordinary HTML control outside the scene, so the redraw leaves it alone and a drag of it continues unbroken.
  In the polar node every modulus is exactly 1, and the angles go up in steps of 360°/`n`.
  Its arrows are all the same length and turn round in `n`ths.

The arithmetic of a diagram is pure and runs natively under test.
That covers where a number lands on the page, which way an arc runs, and how a label is placed.
The arithmetic of a slider is tested the same way, namely its length against its axis and which end is the minimum.
The drawing is not tested natively.

See [Complex Numbers](../library/complex-numbers.md) for the types and formats themselves.
