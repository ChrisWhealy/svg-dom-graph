# Demo changelog

Changes to the demo gallery (`demo-app`, `demo-common`, `demo/`), the demo server, the CDP tests and the docs.
These have no version of their own, and do not change the version of the library. Changes to the library itself are in [`changelog.md`](../changelog.md).
Entries before the library's 0.2.47 are in that file, mixed with the library's own.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Conway's Game of Life demo panel, in a new group, "Cellular Automata": a 40 by 30 board that is drawn on by click, drag or keys, with Play, Step, Random, Clear, wrapping or stopping edges, five patterns including the Gosper glider gun, and a speed slider
- Game of Life demo: a second row of buttons under the board that adds a light, medium or heavy weight spaceship at the left edge, level with the cursor, flying to the right
- Convolution demo panel, after Phase Shift: a signal of two sine waves probed with a cosine and a sine, with the area under each product shaded above and below the axis, and the amplitude found at every frequency
- Angle curve, labelled &theta;, in the Pythagoras demo
- Pythagoras demo text beside the scene
- Conway's Game of Life demo (`eb3dcd8`)

### Changed

- Convolution demo: the spectrum says that `a` and `b` are twice the raw integrals, and its text and docs say that the sums are a correlation, give the normalisation, and show that the conventions are those of the FFT demo
- Demo panels are grouped and ordered by the folders under `demo/panels/` (`NN-group-name/NN-demo-name.html`), not by `data-group` and `data-order`
- `svg_buttons` and `sliders` moved from `demo-common` to the library, as `svg_dom_graph::controls`

### Fixed

- Game of Life demo: a write to the page that fails leaves the board right and the page out of date, which is remembered and drawn again by the next button, key, drag, frame or slider move; the failure puts the Play button back to Play; and a change to the error waits until a drag ends, so the board does not move under the pointer
- Convolution demo: zero frequency is a special case, in which the amplitude is the mean of the signal and not twice it, the factor written is 1, and there is no phase
- Convolution demo: a scene that cannot be drawn is reported as an error that says the four scenes may disagree, the remembered frequency changes only when all four are drawn, and a later success hides the error
- Pythagoras drag test rounds the mouse position to whole pixels, so that CI reads back the angle that was dragged to
- Don't skip cells during rapid pointer drag movements (`468e1ee`)
- Drag ends when the pointer is let go or at `pointercancel` (`68755cb`)
- Prevent model and presentation from diverging after a DOM write failure (``)