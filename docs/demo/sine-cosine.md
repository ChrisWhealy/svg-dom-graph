# Sine and Cosine

Shows that sine and cosine are the height and the width of a point going round the unit circle.
The panel is in the "Signal processing" group, before the Fast Fourier Transform.
It adds no library API: each scene holds one plot node, and `Scene::set_plot_items` draws what changes.

## The three scenes

* **The circle.**<br>
  Point C moves round the unit circle in an anti-clockwise direction.
  The button that stops C and starts it again is in the bottom right corner of this scene, where the circle does not reach.
  A dotted line drops from C to the horizontal axis, where it is labelled W for width.
  Another dotted line runs across to the vertical axis, where it is labelled H for height.
* **The sine.**<br>
  To the right of the circle is a plot showing how the height of point C varies as it moves around the unit circle.
  The horizontal axis is the angle in degrees, from 0 to 360, with a tick every 30.
  The curve also shows a dotted line that drops from C to the horizontal axis, labelled H.
* **The cosine.**<br>
  Below the circle is a plot showing how the horizontal position of point C varies as it moves around the unit circle.
  The curve is plotted downwards in degrees with 0 at the top.
  The curve also has a dotted line that runs across from C to the vertical axis, labelled W.

## The scales

All three scenes are drawn to one scale.
On the circle, 0.2 units is 33.3 pixels, and 30 degrees is the same number of pixels on the angle axes of the sine and the cosine.
So the angle axes are as long as the side of the circle, 400 pixels for 360 degrees.
The sine is 430 pixels wide and 400 high, and the cosine is 400 wide and 430 high.
The extra 30 pixels is a margin at the end of the angle axis, so that the label of 360° is not cut off.
The circle has no margin, and the margins of the other two are not along an axis that is shared with the circle, so a height or a width is the same distance in all three.
The axes that show a size are labelled the same way in all three scenes, and stop at 1, the radius.

## The titles

The sine and the cosine each have a title written in the plot, in a place that nothing else is ever drawn.
The curve of the sine is never below a height of -1, so its title is in the strip below that, at the left.
The curve of the cosine is to the right of the vertical axis for the first quarter of the turn, and so is the dotted line from C.
So its title, in three lines, is at the top left, to the left of the axis and below the labels of the horizontal axis.
The unit tests check that, by trying every whole angle against the box that holds each title.

## What drives what

The angle is the only state.
Each frame moves it on by the time since the last frame, at 30 degrees a second, and all three plots are drawn again from it.
A turn takes twelve seconds, and the curves start again at 0.
The grids, axes and labels are drawn once, and are never touched by a frame.

## Controls

Drag C round the circle to put it at an angle, forwards or backwards.
The pointer has to start within 18 pixels of C, and the pointer shows a hand when it is that near.
While C is held it follows the direction of the pointer from the middle of the circle, in whole degrees, and the sine and the cosine follow it.
Holding C stops it going round by itself, and the button then says "Play".

C can also be reached with the Tab key, and moved with the keyboard.
A handle that is not drawn stands over C, with the role of a slider from 0 to 359, and moves with it.
The handle has the label "Point C", and says its angle as its value.
While it has the focus, a blue ring is drawn round C.
Right and Up move C anti-clockwise by a degree, and Left and Down move it clockwise, as the keys of a slider do.
Page Up and Page Down move it by 15 degrees, and Home and End take it to 0 and to 359.
The angle wraps in both directions, and a key, like a drag, stops C going round by itself.

"Pause" and "Play" stop and start C.
The button is drawn in the circle scene, and works from the keyboard.
Text under the heading gives the angle, W and H as numbers, one to a line.
It is plain text with line breaks and spaces in it, which the page shows as they are, so that the names line up, and it is never read as markup.
Someone who has asked for less motion starts with C still, and can still drag it or move it with the keys.
A gap between frames of more than 100 ms is not counted, so a page that was in the background does not jump.

## How it is checked

`sine_cosine/plots/unit_tests.rs` checks what is drawn for an angle, and that the plots share their scales, in pixels.
`sine_cosine.rs` in the CDP tests checks the sizes of the three scenes, the labels of the circle, dragging C forwards and backwards, the keys that move it, the dotted lines, and the button.
