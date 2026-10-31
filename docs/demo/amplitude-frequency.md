# Amplitude, Frequency and Phase

Shows that one sine wave is described by three numbers, which the page lets a person set.
The panel is in the "Signal processing" group, after Phase Shift and before the Fast Fourier Transform.
It adds no library API: the scene holds one plot node, `Scene::set_plot_items` draws what changes, and the sliders are the browser's own range inputs, put in the scene by `demo-common`.

## The wave

The wave is `y = A · sin(2π · f · t + φ)`.

* **Amplitude, A.**<br>
  From 0 to 1, in steps of 0.01.
  How tall the wave is.
  Two dotted lines, at A and at -A, mark it, and there are none when the amplitude is 0, since they would be on the axis.
* **Frequency, f.**<br>
  From 1 Hz to 8 Hz, in steps of 0.1 Hz.
  How many cycles the wave makes in a second.
* **Phase, φ.**<br>
  From 0 to 360 degrees, in steps of 1.
  How far along its cycle the wave is at time 0.
  A positive phase moves the wave to the left.

The formula, with the three numbers in it, is written at the bottom left of the plot, below the lowest that a wave of amplitude 1 goes.
The wave starts at an amplitude of 1, a frequency of 2 Hz and a phase of 0.

## The scene

The scene is 800 pixels wide and 400 high.
The top 90 pixels are a strip for the three sliders, side by side, so that they are never over the wave.
The plot is below it, with 40 pixels at the left for the labels of the height and 30 pixels at the right for the label of the last tick.

The vertical axis is the height, from -1.2 to 1.2, labelled in steps of 0.2 as far as 1, as in the circle demos.
The horizontal axis is time, from 0 to 1 second, with a tick every tenth of a second.
It does not change when the frequency does, so a higher frequency packs more cycles into the same second, from one at 1 Hz to eight at 8 Hz.
The wave is 512 straight segments, which is 64 for each cycle at 8 Hz.

## The sliders

A slider is the browser's own `<input type="range">`, in a `<foreignObject>` of the scene's `<svg>`, with its name and its value written above it as SVG text.
The browser does the dragging with a pointer or a finger, the keys, and the accessibility, so assistive technology says its name, its range and its value, such as "2.0 Hz".
The page's own code only says where a slider goes and how it looks.

* Right and Up move it a step, and Left and Down move it back a step.
* Page Up and Page Down move it on further than a step, by as much as the browser chooses.
* Home and End take it to the least and the greatest value.
* It stops at the ends and does not wrap.

A value is always a whole number of steps from the least, so that it can be written exactly.

## The sliders of `demo-common`

The sliders are `demo-common/src/sliders`, and do not know what they are for.
The function given to `sliders::watch` is told, from the `input` event, which slider moved and to what, draws that, and tells the slider what to write with `sliders::set_value`.
The connector routing, fixing points, ellipses and complex number panels have sliders too, and all use the same module.

What makes a slider as long as an axis is the arithmetic in `sliders::frame`, which touches no DOM.
A native slider's thumb travels from half its own width inside one end of the element to half its own width inside the other, so the element is made one thumb wider than the track.
The thumb is given a fixed width by a style that the module puts in the page, since the browsers' own differ.
A vertical slider is the same element turned a quarter turn.

A demo that draws its scene again for every move of a slider is also served.
`sliders::add_slider` puts the slider in a group of its own, directly in the `<svg>`, and `sliders::clear_scene` takes everything out of the `<svg>` but those groups.
So the slider is the same element before the scene is drawn again and after, a drag carries on across the scenes, and the keyboard focus is kept.
`set_range` changes the greatest value of a slider whose room changes, `set_disabled` dims one, and `value` reads where one is.

## How it is checked

`amplitude_frequency/plots/unit_tests.rs` checks the configuration, the wave, that it never goes beyond its amplitude, that a wave of n cycles has the right number of turns, the dotted lines, and the formula, and that it is below where the wave goes.
`demo-common/src/sliders/unit_tests.rs` checks where a slider goes, how a value is rounded, and the style that the thumbs are given.
`amplitude_frequency.rs` in the CDP tests checks the size of the scene, the three sliders, the pointer and the keys, and that the wave changes.
