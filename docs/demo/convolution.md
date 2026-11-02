# Convolution

Shows how a cosine and a sine of one frequency find out how much of that frequency is in a signal.
The panel is in the "Signal processing" group, after Phase Shift and before the Fast Fourier Transform.
It adds one library item, `PlotArea`, which shades the area between a line and a baseline in one colour above the baseline and another below it.
Everything else is built from what the library already has: four scenes with one plot node each, `Scene::set_plot_items` to draw what changes, and a slider from `svg_dom_graph::controls::sliders`.

## The signal and the probe

The signal is two sine waves added together: `1.0 · sin(2π · 2 · t + 40°)` and `0.6 · sin(2π · 5 · t + 110°)`.
It is looked at for one second, as 512 straight segments.
Both frequencies are whole numbers, so each wave makes a whole number of cycles in that second.

The probe is a cosine and a sine of one frequency, which the slider sets from 0 Hz to 10 Hz in steps of 0.1 Hz.
It starts at 2 Hz.

## The four scenes

* **The signal.**<br>
  800 by 330 pixels.
  The top 70 pixels are a strip that holds the slider.
  Below it, the signal is drawn in black over the cosine (violet) and the sine (teal) of the probe, which are thinner.
  The three are named along the bottom, and the names of the probes say the frequency.
* **The two products.**<br>
  400 by 250 pixels each, side by side: the signal times the cosine, and the signal times the sine.
  They are drawn to the same scale of height as the signal, from -1.8 to 1.8.
  The area between each product and the axis is shaded blue where the product is above the axis and orange where it is below, at 40% opacity so that the grid shows through.
  The product's line is drawn over the shading.
  Three numbers are written at the top: the area above, the area below, and the integral, which is the one plus the other, since the area below is written as a negative number.
* **The spectrum.**<br>
  800 by 240 pixels.
  It has the amplitude that is found at every frequency of the slider, a dotted line down from the frequency of the probe, and a dot at the amplitude found there.
  Under the axis it says what was found: `a`, `b`, the amplitude and the phase.

## What the numbers mean

For a wave `A · sin(2π · f · t + φ)` that is looked at for a whole number of cycles, twice the integral of its product with the cosine is `A · sin φ`, and twice the integral of its product with the sine is `A · cos φ`.
Those are `a` and `b` in the writing, so the amplitude is `√(a² + b²)`, and the phase is the angle `atan2(a, b)`.
At 2 Hz the page says `a = +0.64`, `b = +0.77`, an amplitude of 1.00 and a phase of 40°, which are the numbers of the first wave.
At 5 Hz it says an amplitude of 0.60 and a phase of 110°, which are those of the second.
At a whole frequency that is in neither, such as 3 Hz, both integrals are nothing, because the shading above the axis is as large as the shading below it.
The page then writes no phase, since a wave that is not there has none.

A frequency that does not make a whole number of cycles in the second finds a little of each wave.
This is spectral leakage, and it is why the spectrum has small bumps between its peaks, and why a peak can be a step away from the frequency of its wave, as the leakage of each wave adds to the other.

## The shading

`PlotArea` takes a line, a baseline, a colour for the part above and one for the part below, and an opacity.
It cuts the line where it crosses the baseline, so that each shaded piece is wholly on one side, and it cuts the shading to the plotted area as a line is cut.
A point that is not finite breaks the area in two, as it breaks a line.
It is drawn in the order that it is given, so the demo gives it before the product's line.

## How it is checked

`convolution/analysis/unit_tests.rs` checks the areas, including a segment that crosses the axis, that a whole cycle of a sine is as much below as above, that the probe of each wave's own frequency finds its amplitude and phase, that a whole frequency that is not there finds nothing, and the peaks of the spectrum.
`convolution/plots/unit_tests.rs` checks what is drawn: the order of the lines, the colours and opacity of the shading, the writing, and that the three plots of time have one scale of height.
The library's own tests check `PlotArea`: where the line is cut, the area of each piece against that of a sine, the cutting to the plotted area, and that the browser draws a polygon of each colour.
`convolution.rs` in the CDP tests checks the sizes of the scenes, the slider and its keys and pointer, that every scene follows it, and that the area of the shading in the browser is the area that is written.
