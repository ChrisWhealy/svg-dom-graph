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
It starts at 3 Hz, where there is none of either wave and the areas cancel.

## The four scenes

* **The signal.**<br>
  800 by 330 pixels.
  The top 70 pixels are a strip that holds the slider.
  Below it, the signal is drawn in black over the cosine (violet) and the sine (teal) of the probe, which are thinner.
  The three are named along the bottom, and the names of the probes say the frequency.
* **The two products.**<br>
  400 by 250 pixels each, side by side: the signal times the cosine, and the signal times the sine.
  They are drawn to the same scale of height as the signal, from -1.8 to 1.8.
  The area between each product and the axis is shaded blue where the product is above the axis and orange where it is below.
  The shading is at 40% opacity, so that the grid shows through.
  The product's line is drawn over the shading.
  Three numbers are written at the top: the area above, the area below, and the integral.
  The integral is the one plus the other, since the area below is written as a negative number.
* **The spectrum.**<br>
  800 by 260 pixels.
  It has the amplitude that is found at every frequency of the slider.
  A dotted line comes down from the frequency of the probe, with a dot at the amplitude found there.
  Under the axis, in two lines, it says what was found.
  The first line has `a` and `b`, as twice the integrals that are written in the products.
  The second has the amplitude and the phase.

## What the numbers mean

For a wave `A · sin(2π · f · t + φ)` that is looked at for a whole number of cycles, twice the integral of its product with the cosine is `A · sin φ`.
Twice the integral of its product with the sine is `A · cos φ`.
Those are `a` and `b` in the writing, so the amplitude is `√(a² + b²)`, and the phase is the angle `atan2(a, b)`.
At 2 Hz the page says `a = 2 × 0.32 = +0.64`, `b = 2 × 0.38 = +0.77`, an amplitude of 1.00 and a phase of 40°, which are the numbers of the first wave.
At 5 Hz it says an amplitude of 0.60 and a phase of 110°, which are those of the second.
At a whole frequency that is in neither, such as 3 Hz, both integrals are nothing, because the shading above the axis is as large as the shading below it.
The page then writes no phase, since a wave that is not there has none.

A frequency that does not make a whole number of cycles in the second finds a little of each wave.
This is spectral leakage.
It comes from looking at the signal for a window that does not hold a whole number of cycles of the probe.
It is why the spectrum has small bumps between its peaks.
It is also why a peak can be a step away from the frequency of its wave, as the leakage of each wave adds to the other.

## Correlation, normalisation and the conventions

**What the sums are.**
Multiplying the signal by a probe and adding up the result is a correlation, or a projection of the signal onto the probe.
It is not a convolution in the usual sense.
A convolution flips one function in time and slides it across the other, but here nothing is slid and each frequency gives one number.
The panel keeps the name "Convolution", and its text says so.

**Raw integrals and recovered amplitudes.**
The three numbers written in a product plot are raw: the area above, the area below, and the integral.
For a wave `A · sin(2π · f · t + φ)` over whole cycles, the raw integral with the cosine is `A/2 · sin φ`, since the average of a sine squared is a half.
So `a` and `b` in the spectrum are twice the raw integrals, which is `2/T` for a window of `T` = 1 second.
The spectrum says so in its first line: `a = 2 × 0.32 = +0.64`.
Without the 2 the amplitudes found would be half as tall as the waves that made the signal.

**Zero frequency.**
This is a special case.
The cosine probe is always 1 and the sine probe is always 0.
So the integral with the cosine is the mean of the signal, and the integral with the sine is nothing.
The amplitude is the size of the constant that the signal is offset by.
That is the mean itself and not twice it, so the factor is 1 and not `2/T`.
A constant is not a wave, so the page says no phase for it, and `Finding::has_phase` is false.
In the transform the bin for 0 Hz is `X = N · a`, where the others are `X = N/2 · (a − i·b)`.
The signal of the demo has a mean of nothing, so the page shows `a = 1 × 0.00` at 0 Hz.
A constant that is added to it is found at 0 Hz and nowhere else.
The same is true of a phase at no amplitude: the page does not say one, since a wave that is not there has none.

**The conventions of the FFT demo.**
The frequency is in cycles per second and the angle is `2π · f · t`.
A wave is `A · sin(2π · f · t + φ)`, so a positive phase moves it to the left, as in Phase Shift and Amplitude, Frequency and Phase.
The forward transform has `e^(−2πi·f·t)` in it, with no scaling.
So the bin of the transform for the frequency of the probe is `X = N/2 · (a − i·b)`, its size is `N/2 · A`, and its angle plus 90° is the phase.
That is the amplitude `2|X|/N` and the phase `angle(X) + 90°` of the FFT demo.
`convolution/analysis/unit_tests.rs` checks it against a transform worked out from its definition, for both waves.

## The integral and the shading

`PlotArea` shows an integral and does not work it out, so the demo works it out itself, and the two agree because they use the same shape.
The curve is drawn as straight segments through the 513 samples, so the area between it and the axis is a run of trapezoids.
The demo adds them up by the trapezoid rule, and counts a segment that crosses the axis as a triangle on each side, which is where the library cuts it.
`convolution/analysis/unit_tests.rs` checks that the net of that is the signed area of the polygon that the library shades, worked out by the shoelace formula, to 1e-12.
The CDP test checks the same against the shading that the browser draws, to within six thousandths.

## The shading

`PlotArea` takes a line, a baseline, a colour for the part above and one for the part below, and an opacity.
It cuts the line where it crosses the baseline, so that each shaded piece is wholly on one side.
It cuts the shading to the plotted area as a line is cut.
A point that is not finite breaks the area in two, as it breaks a line.
It is drawn in the order that it is given, so the demo gives it before the product's line.

## When a scene cannot be drawn

Each scene is drawn on its own, and the library makes each one all or nothing, so a scene that fails keeps what it showed before.
The four together are not all or nothing.
If the second fails, the signal shows the new frequency, and the products and the spectrum still show the old one.
The demo accepts that as an error and does not roll back, since a failed write has not been seen in practice.
It writes in the error banner, above the scenes, that they may disagree with each other and with the slider, and that moving the slider again draws them all again.
The frequency that the demo remembers is only changed once all four are drawn, and a successful move hides the banner.

## How it is checked

`convolution/analysis/unit_tests.rs` checks the areas, including a segment that crosses the axis.
It checks that a whole cycle of a sine is as much below as above, and that the probe of each wave's own frequency finds its amplitude and phase.
It checks that a whole frequency that is not there finds nothing, and the peaks of the spectrum.
`convolution/plots/unit_tests.rs` checks what is drawn: the order of the lines, the colours and opacity of the shading, and the writing.
It also checks that the three plots of time have one scale of height.
The library's own tests check `PlotArea`: where the line is cut, and the area of each piece against that of a sine.
They also check the cutting to the plotted area, and that the browser draws a polygon of each colour.
`convolution/unit_tests.rs` in the CDP tests checks the sizes of the scenes, and the slider with its keys and pointer.
It checks that every scene follows the slider, and that the area of the shading in the browser is the area that is written.
