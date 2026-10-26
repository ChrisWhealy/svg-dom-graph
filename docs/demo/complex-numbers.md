# Complex Numbers

Demonstrates `NodeValues::Complex64` and every `ComplexNumberFormat`, with the complex plane drawn as Argand diagrams.
The real part of a number runs along the horizontal axis and the imaginary part up the vertical one, so every number is a point, and every form is a different way of saying where it is.
The diagrams are SVG drawn by the demo, in the scene's own group, so they zoom and pan with the nodes.
Each is described as a whole to assistive technology.

* **One number, seven ways.**<br>
  A diagram of `3 + 4i`, whose modulus is exactly 5, with its real part and imaginary part dashed to the axes, and its modulus and angle, all written on the diagram.
  A slider runs along the bottom of the diagram and sets the real part. A second runs up its left side and sets the imaginary part.
  Each is **exactly as long as the axis it lies along**, from -5 to 5, so its thumb is directly under, or beside, the point of the axis whose value it stands for.
  That is done by making the slider one thumb wider than the axis, since a thumb's centre travels from half a thumb in at one end to half a thumb in at the other. The thumb is given the same width in every browser, and the track is drawn as exactly that much shorter than the input, by a style that the demo puts in the page itself with the width that `THUMB` says, and not by `style.css`: the length and the style that makes it true must come from one place, and a stylesheet that a browser has kept from an earlier visit cannot change them. The geometry was measured in Chrome and Firefox, at the default zoom and zoomed in three times: the input is always the axis plus one thumb, scaled.
  The vertical slider is the same element turned a quarter turn anticlockwise, so its minimum is at the bottom.
  Both are range inputs inside the scene (an HTML `<input>` in a `<foreignObject>`), so they zoom and pan with the diagram. They have the keyboard, and each is named, and its value is read as a number to one place.
  As either moves, the diagram is drawn again, and each of the seven nodes is given the new number with `Scene::set_data_values`, which rewrites its cell, its picture and its exponent, and nothing else:
  rectangular, polar in degrees, trigonometric, polar in radians, exponential, log-polar and matrix.
  A node is never resized by `set_data_values`, so each is first drawn for whichever number the sliders can reach that makes its form widest (the corners, a very small number whose logarithm is long, and zero), found by measuring each in the page, and is then given `3 + 4i`. No number the sliders reach can be wider than its cell.
* **The roots of unity.**<br>
  The points of the unit circle that, raised to the power `n`, give 1, which are the twiddle factors of an `n`-point Fourier transform.
  A slider above the diagram sets `n` from 1 to 20. The upper limit is arbitrary, and is `MOST` in `roots.rs`.
  A node cannot be given more numbers than it was drawn with, and the library cannot remove one, so a change of `n` draws the diagram and its two nodes, rectangular and polar, again from nothing, and carries the zoom and pan over to the new one.
  The slider is an ordinary HTML control outside the scene, so that it is not drawn again as the scene is, and a drag of it is not broken.
  In the polar node every modulus is exactly 1 and the angles go up in steps of 360°/`n`, and its arrows are all the same length and turn round in `n`ths.

The arithmetic of a diagram (where a number lands on the page, which way an arc runs, and how a label is placed) and of a slider (its length against its axis, and which end of it is the minimum) is pure and is tested natively. The drawing is not.

See [Complex Numbers](../library/complex-numbers.md) for the types and formats themselves.
