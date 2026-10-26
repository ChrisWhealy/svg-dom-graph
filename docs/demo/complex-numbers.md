# Complex Numbers

Demonstrates `NodeValues::Complex64` and every `ComplexNumberFormat`, with the complex plane drawn as Argand diagrams.
The real part of a number runs along the horizontal axis and the imaginary part up the vertical one, so every number is a point, and every form is a different way of saying where it is.
The diagrams are SVG drawn by the demo, in the scene's own group, so they zoom and pan with the nodes.
Each is described as a whole to assistive technology.

* **One number in each quadrant.**<br>
  Four diagrams, one for `3 + 2i`, `-2 + 3i`, `-3 - 2i` and `2 - 3i`.
  Each marks the modulus as a solid line from the origin and the angle as an arc from the real axis, and writes both on the diagram: `r = 3.61` and `θ = 33.69°`.
  The dot is labelled with the number in rectangular form.
  Under each diagram is a node holding the same number in polar form, which begins with a small arrow in the direction of the number, drawn twice as high as a line of text with `ComplexStyle::with_icon_scale(2.0)`, so the labels, the digits and the arrow can be read against one another.
  The angle of a number in the third or fourth quadrant is negative, because it is measured clockwise.
* **One number, seven ways.**<br>
  `3 + 4i`, whose modulus is exactly 5, drawn large, with its real part and imaginary part dashed to the axes as well.
  Beside it is a node for each way of writing it: rectangular, polar in degrees, trigonometric, polar in radians, exponential, log-polar and matrix.
  The rectangular node begins with a picture of the two legs, the adjacent along the real axis in green and the opposite up or down in purple, and the polar node with a picture of the hypotenuse.
  The exponential node writes its exponent raised and smaller, an `i` and then the angle in radians.
  The matrix is the one node whose cell is two rows high.
* **The eighth roots of unity.**<br>
  The eight points of the unit circle that, raised to the eighth power, give 1, which are the twiddle factors of an eight-point Fourier transform.
  Each is 45° further round than the one before.
  A rectangular node and a polar node hold the same eight numbers in two rows of four.
  In the polar node every modulus is exactly 1 and the angles go up in steps of 45°, and its eight arrows are all the same length and turn round in eighths.
  In the rectangular node the legs of the pictures shrink to nothing on the axes and are equal at 45°.

The arithmetic of a diagram (where a number lands on the page, which way an arc runs, and how a label is placed) is pure and is tested natively. The drawing is not.

See [Complex Numbers](../library/complex-numbers.md) for the types and formats themselves.
