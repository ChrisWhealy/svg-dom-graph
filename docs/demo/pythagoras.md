# The Theorem of Pythagoras

Shows that in a right triangle the square on the longest side has the same area as the other two squares together.
It does so by drawing the squares for a triangle that a person changes.
The panel is in the "Signal processing" group, after Amplitude, Frequency and Phase and before the Fast Fourier Transform.
It adds no library API, and it has no plot node: the scene is drawn directly as SVG.

## The scene

The scene is 560 pixels square, with no axes, grid or ticks.
It has the unit circle, with a radius of 140 pixels, and point C on it, which starts at 45 degrees.
C is dragged round the circle with the pointer, or reached with the Tab key and moved with the keys.
This is the same as in Sine and Cosine and in Phase Shift.

The centre of the circle O, the point W under C on the horizontal axis (or over it, when C is below), and C make a right triangle.
The right angle is at W, and is marked with a small square.
The hypotenuse is the radius OC, so it is always 1 long.
The other two sides are the width OW and the height WC of C, which are the absolute values of its cosine and its sine.

A square is drawn on each of the three sides, on the side of it that is away from the triangle, so that no square covers the triangle.

* The square on the hypotenuse is blue.
* The squares on the width and on the height are green.

Each is a fill that lets what is under it be seen, with an outline of the same colour that does not, so that the circle shows through.
The area of each is written in it, as `c² = 1.00`, `a² = 0.50` and `b² = 0.50`, and the sum is written at the top: `a² + b² = c²`, and under it `0.50 + 0.50 = 1.00`.

## What happens as C moves

Each move of C draws the triangle and the three squares again.
The blue square is always 1, since the radius is.
The green squares are the cosine squared and the sine squared, so they grow and shrink as C goes round, and the two always add up to the blue one.
That is the identity that `cos² θ + sin² θ = 1`, which is the theorem for this triangle.

At 0 and 180 degrees the triangle is flat, so the square on the height has no area.
At 90 and 270 degrees the square on the width has none.
A flat triangle has no away, so the squares go to the side that they go to just before it, and flip when C crosses the axis.

## What is in the scene

The shapes are in a group of their own that does not take the pointer, since the circle control listens on the `<svg>` for a person taking hold of C.
The handle that stands for C in the keyboard's tab order is added after the shapes, so it is over them.
The `<svg>` has the role of an image, and a label that says the angle and the sum, which is written again as C moves.

`circle_control` is shared with the sine, cosine and phase shift demos.
Those draw their circle as a plot node, and this one does not.
So the circle control can be made either for a plot (`CircleControl::new`) or for a circle that the demo has drawn itself (`CircleControl::bare`).
The second is told where the middle of the circle is and how long the radius is.

## How it is checked

`pythagoras/figure/unit_tests.rs` checks the triangle and the squares for every whole angle.
Each is a square on its own side, the area on the hypotenuse is 1, and the others are the cosine squared and the sine squared.
It checks that they add up, that no square covers the triangle, that the flat triangles go the way that they should, and how the areas are written.
`pythagoras/unit_tests.rs` in the CDP tests checks the size of the scene, and that nothing but the scene is in the panel.
It checks the colours and the transparency, the start at 45 degrees, and dragging and the keys.
It checks that the green squares always add up to the blue one, the writing, that the handle is over the shapes, and the label.
