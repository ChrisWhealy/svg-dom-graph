# Phase Shift

Shows a phase shift as the angle of a point on the unit circle.
The panel is in the "Signal processing" group, after Sine and Cosine and before the Fast Fourier Transform.
It adds no library API: each scene holds one plot node, and `Scene::set_plot_items` draws what changes.

## The two scenes

The scenes are the circle and the sine of [Sine and Cosine](sine-cosine.md), without the cosine and without the animation.
They are drawn to the same scale as there, so 30 degrees on the angle axis is as many pixels as 0.2 units on the circle.

* **The circle.**<br>
  400 by 400 pixels, with point C on it.
  C starts at 45 degrees and is not animated, so it moves only when a person moves it.
  It is dragged with the pointer, or reached with the Tab key and moved with the arrow keys.
  The dotted lines to the axes, with W and H, are the ones of the sine and cosine demo.
  A small orange arc near the middle shows the angle that the radius makes with the positive horizontal axis, and the angle is written beside it as `φ=45°`.
  The arc always starts on the positive horizontal axis and runs anti-clockwise to C.
  Beyond 180 degrees it keeps going, so it shows the outside angle that C has been through and not the smaller angle that is left.
  The text is on the line that splits the angle in two, so for an angle of more than 180 degrees it is on the far side of the middle from C.
* **The sine.**<br>
  To the right of the circle, 430 pixels wide and 400 high, with two sine waves in it.
  The first is `sin(x)`, in blue, and never moves.
  The second is `sin(x + φ)`, in orange, where φ is the angle of C.
  Each wave is named in its own colour at the bottom left, and the name of the second says the shift.

## What the shift is

A positive shift moves the second wave to the left, so it is ahead of the first and reaches each crest sooner.
At the left edge, where x is 0, the second wave is at the height of C, which is where a wave that C drew would begin if it had set out from that angle.
A dot for C marks that place.

The distance between the places where the two waves rise through zero is the shift.
The first rises through zero at 360 degrees, and the second at 360 - φ, so a dotted orange line joins them along the axis, with φ written over it.
A shift of nothing has no such line, since the waves are one on the other.
A shift of 90 degrees makes the second wave a cosine, and a shift of 180 degrees turns it upside down.

## Controls

C is dragged and moved with the keys in the same way as in Sine and Cosine, and the code is shared.
The pointer has to start within 18 pixels of C, and the angle is a whole number of degrees.
Right and Up move C anti-clockwise by a degree, Left and Down clockwise.
Page Up and Page Down move it by 15 degrees, and Home and End take it to 0 and to 359.
The angle wraps in both directions, and is never written as -0.

There is no text outside the scenes: the angle is written beside the arc, and the names of the waves are written in the sine plot.
The handle that stands for C says its angle to assistive technology, as "45 degrees".

## The shared code

`circle_control/` holds what the two demos have in common: the plot that holds a scene, the handle that the keyboard and assistive technology use, the dragging, and the pure functions that say which angle a direction is and which key moves C where.
Each demo is told the angle that a person has moved C to, draws what it draws for that angle, and tells the control where C now is.
Sine and Cosine also stops C going round by itself when it is told.

## How it is checked

`phase_shift/plots/unit_tests.rs` checks the waves, that the first never moves and that the second is the first with the shift added, that the second begins at the height of C, the arc and its text, including the long way round beyond 180 degrees, the dotted line between the places where the waves rise through zero, the colours and the names, and that the names are below where either wave ever goes.
`circle_control/angles/unit_tests.rs` checks the angles and the keys.
`phase_shift.rs` in the CDP tests checks the sizes of the scenes, the start at 45 degrees, dragging, the keys, that the first wave never moves while the second does, the dotted line, and the arc and its text.
