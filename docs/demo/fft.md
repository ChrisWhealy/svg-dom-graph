# Fast Fourier Transform

Demonstrates a forward radix-2 decimation-in-time fast Fourier transform, one stage at a time.
It uses the existing complex-number formats and `Scene::add_butterfly_node`, and adds no library API.
The panel is in the "Signal processing" group.

The panel opens with a short explanation for a reader who has not met the transform.
It says what problem the transform solves, why the fast version is fast, and what each part of the demo shows.

## The waves

The input is not typed in as numbers.
It is the sum of three sine waves, which the reader shapes with sliders.
Wave A completes one cycle in the window, wave B two, and wave C three.
Each has an amplitude from 0 to 1, and a phase from 0° to 345° in steps of 15°.

* **The graph.**<br>
  It is a plot node of the library, in a scene of its own.
  See [Plot Nodes](../library/plot-nodes.md).
  The demo configures it once.
  It is a real plane with a legend.
  Its horizontal axis runs from 0 to 1, with ticks named as quarters, and its vertical axis runs from -3 to 3.
  Only what is plotted changes as a slider moves, with `Scene::set_plot_items`.
  It plots the three waves and their sum.
  The vertical axis stands at the left edge and shows positive and negative values.
  Time runs from 0 to 1 along the horizontal axis, so it shows positive values only.
  The scale never changes, so the axes stay put as a wave grows.
  The lines differ by more than colour: A is solid, B is dashed, C is dotted, and the sum is thick.
  Orange dots mark the samples, which are the only thing the transform sees.
* **The sliders.**<br>
  They are native range inputs, arranged as an amplitude and a phase for each wave, with the number beside each.
  The graph, the numbers and the table follow a slider as it moves.
  The two scenes are drawn again when it is let go, because drawing them takes longer than a slider takes to move.
* **The table.**<br>
  It shows what the transform found, beside what was set: amplitude, phase, and whether they agree.
  A wave of amplitude 0 has no phase, so it shows a dash.
  A last sentence gives the largest amplitude in any frequency that none of the waves uses, which should be zero.

A wave `a·sin(2πkn/N + φ)` has `X[k] = (N·a/2)·e^(i(φ − 90°))`.
So the amplitude found is `2·|X[k]|/N`, and the phase found is the angle of `X[k]` plus 90°.
Each of these is in `fft::waves`, with tests that set every phase and read it back.

## The two scenes

* **The overview.**<br>
  One node holds one complex number.
  There is a column of lane nodes for the bit-reversed input `S0`, and one for the result of each stage `S1` onwards.
  So eight samples draw four columns of eight nodes, and sixteen samples draw five columns of sixteen.
  Each butterfly joins two lanes of one column to the same two lanes of the next by four straight connectors.
  They are ordinary edges, so they share the scene's zoom and pan.
  The count is 48 connectors at eight samples and 128 at sixteen.
* **The inspector.**<br>
  A second scene holds one butterfly node, fed by `a`, `b` and `w`, with its sum and difference on the right.
  It shows the butterfly chosen in the list under the overview.
  Choosing another gives the same nodes new numbers.
  The frame can be dragged.

## The controls

* **Samples.**<br>
  A list of 8 and 16.
* **The stepping toolbar.**<br>
  Each press of `Next` completes one whole stage.
  `Previous` steps back, and shows the stored state again.
  `Previous` from the first stage and `Restart` both return to the start, with no stage complete.
* **Butterfly.**<br>
  A list of the `N/2` butterflies of the stage being shown.
  Each entry names its lanes.
  Before the first stage, the inspector previews stage 1.

The default butterfly is the first whose `a` or `b` is not zero.
If every one is zero, it is the first.
A butterfly chosen at a stage is remembered for that stage until `Restart`.

## What a stage shows

A stage that is complete shows its stored values.
A stage that is not yet complete holds a zero, drawn faint, and is marked as not computed.
So no result is on the page before it exists.
The four lanes and four connectors of the chosen butterfly are emphasised.

At the last stage, the lanes hold the transform `X[0]` onwards, in natural order.
The input alone is reordered.
The forward transform is not scaled, so a constant of 1 gives `X[0] = N`.

## The arithmetic

All of it is worked out in plain Rust by `fft::algorithm::trace`.
It calls `ComplexBuf::butterfly` once for each whole block, with the twiddle factors conjugated for the forward direction.
It also records each butterfly from the state before its stage, using the scalar complex operations.
Those are then checked against the buffer.
The independent unit tests compare the result with a transform from the definition, using `sin` and `cos`, for 2 to 32 samples.
The fixed test signals, such as an impulse, live in the tests only.

## Accessibility

* The nodes are named, and each says what feeds it and what it feeds.
  The connectors have no name and are not tab stops, so sixteen samples do not add 128 items to read.
  A test checks that Chrome's own accessibility tree agrees.
* The keyboard stops are the six sliders, the toolbars and the two native lists.
* The graph has a text description of every wave and of the range of the sum.
* One status line outside both scenes says what changed.
  It is a live region, and a scene being replaced never removes it.
* The sentences name the stage, the block width, the butterfly, its lanes and its twiddle factor.
  Choosing another butterfly speaks its numbers, and never lists every lane.

## Changing the size or the waves

The new transform is worked out first.
Then both scenes are drawn out of sight, and only a complete pair replaces the old one.
So a transform that cannot be drawn leaves the old one in place and working.
The zoom of each scene is kept, and so is the place the butterfly was dragged to.
If the new canvas has no room for it there, it is moved inside, and the status line says so.

## When a step fails

A step writes many nodes, and is not atomic.
If it fails, the toolbar is put back and the last good state is drawn again.
If that fails too, the panel shows an error and a `Retry display` button, and refuses further steps until a retry works.
The logic is in `fft::cursor`, and its native tests inject each failure.

## Reading the diagram at sixteen samples

The last stage joins lanes that are eight rows apart, so its connectors are long.
The columns are therefore further apart than at eight samples.
The canvas is wider than the panel, and scrolls.
Zoom out with the toolbar to see all of it.
