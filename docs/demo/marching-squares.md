# Marching squares

The marching squares algorithm on a sampled scalar field, one cell at a time, at three levels.
The panel is in the group "Computational Geometry".
It adds no library API.
Everything is demo-local, in `demo-app/src/marching_squares/`.

## The algorithm

A field is a grid of samples, row by row, with row 0 at the top.
The cell `(r, c)` has the corners TL, TR, BR and BL, and `g` is a sample minus the level.
A corner is set when `g > 0`, with TL = 1, TR = 2, BR = 4 and BL = 8, which gives a mask from 0 to 15.
Each mask says which sides of the cell the contour crosses and how the crossings are joined.
A crossing is calculated once for each grid edge, in the same direction every time, and both cells that share the edge use it, so the coordinates are the same bits.
A sample exactly at a level is refused with `VertexOnLevel`, since that is a plateau and not a saddle.

The masks 5 and 10 are ambiguous: the two corners above the level are opposite each other.
`Q = gTL·gBR − gTR·gBL` settles it.
For `Q > 0` the pairs are T–R and B–L, for `Q < 0` they are T–L and R–B, and the same rule serves both masks.
`rho = |Q| / max(|gTL·gBR|, |gTR·gBL|)` is how big `Q` is next to the terms it is made of.
Both products are positive in an ambiguous cell, so `rho` is always in `[0, 1)`.
A cell with `0 < rho < 0.10` is noted as near-degenerate, which never changes the branch.
For `Q = 0` exactly, the two strands cross at the saddle, `sx = (gTL − gBL)/d` and `sy = (gTL − gTR)/d` with `d = gTL − gTR + gBR − gBL`.
Each strand has a virtual key of its own, so the graph has two strands that cross, and never a node of degree four.

`stitch` joins segments into contours, and it works on any set of them.
The open chains come first, each from its lowest end, and then the cycles, each from its lowest key.
Keys are ordered by kind (`H`, `V`, `SaddleV`, `SaddleH`), then row, then column.
A cycle is rotated to its lowest key and goes the way with the lower sequence.
The result does not depend on the order of the segments.
For the first `k` cells, the same stitcher runs on the segments of those cells.
A crossing on the edge of the cells visited has one segment, so it is a provisional open end, and nothing is joined across a cell that has not been visited.
A loop is closed only once all its cells have been visited.
`Trace::digest` hashes the keys and the bits of every coordinate.

## The fields

| Field | Samples | Levels |
|---|---|---|
| Ripple plane (default) | 25 × 19 of `sin(1.15x)·cos(1.35y) + 0.16x − 0.08y` | −0.53, +0.101, +0.52 |
| Twin hills | 25 × 19 of two Gaussian hills | −0.12, +0.24, +0.66 |
| Coarse saddle waves | 9 × 7 of `sin(x)·sin(y)` | −0.10, +0.10, +0.20 |
| Saddle lab | five 2 × 2 cells at threshold zero | −0.5, 0, +0.5 |

The three Ripple levels each have one ambiguous cell: case 10, `Q` negative at (4, 1), case 10, `Q` negative at (14, 12), and case 5, `Q` positive at (13, 22).
The middle level is +0.101 and not +0.100, because +0.100 is exactly the sample at row 14, column 12.
Their `rho` is 0.892, 0.425 and 0.416, and the two segments of each are 0.85, 0.71 and 0.76 of a grid unit apart, so the decision can be seen.
Moving the middle level to +0.102 keeps case 10, but `Q` becomes positive and the pairing turns to T–R and B–L.
The coarse preset has 6, 6 and 2 ambiguous cells, with both masks at the first two levels, and its `Q` is never zero.
The saddle lab has the five named cells: case 5 with `Q` positive and negative, case 10 with both, and the exact tie.
The same solver makes all of them.

## The page

The stage is one `<svg>`.
The controls are in it, above the scene, and they never zoom: five buttons from the library's `svg_buttons`, the range from its `sliders`, and the selectors and the number as the browser's own controls in a `<foreignObject>`.
Below them is a group that each scene is swapped into.
A scene is a `Scene` with one `PlotNode`, drawn at the grid's own coordinates on axes padded by 0.25, with a toolbar for the zoom and the pan.
The plot items are the samples, an outline for each ambiguous cell that has been visited, the contours with a pattern for each level, and an outline for the inspected cell.
A loop repeats its first point, which leaves a notch at the seam, so a dot as wide as the line goes there.
The inspector beside the plot draws the inspected cell enlarged: its samples, its crossings and its segments.

The scene is built in a hidden `<svg>` that is in the document, so that it can be measured, and it is swapped in with one call to `replaceChildren` only when all of it has been built.
The zoom and the pan are carried over to the new scene before the swap if the field has the same axes, and a view that cannot be set aborts the swap.
Every scene has an ID of its own, so a retry never reuses one.
Focus in the scene is put back by the name of the control.

## State and the controller

The state is the field, the three levels, the number of cells visited, which levels are shown, which level is inspected, and the chosen cell.
`project(trace, state)` is a pure function of them, so going forward, going back and jumping give the same picture.
The inspector follows the last cell visited unless one is chosen.
A cell that has not been visited says "Not yet evaluated", and reveals no mask, `Q`, crossing or pairing.
Switching field shows its last step, and keeps the view only between fields with the same axes, which are Ripple and Twin hills.
A threshold edit builds a new trace before anything changes.
It must stay finite and strictly ascending, with no sample exactly at a level, and a refused value leaves everything and puts the number back.

The controller has the same `Surface` and `Scheduler` as the other demos.
A draw that fails before the page is touched keeps the state.
One that fails after, or a rollback that was not whole, marks the page dirty, and the next action draws it again first, even one with nowhere to go.
Range input is coalesced to one draw for each frame.
Each frame carries the generation, which counts the traces and scenes, and a token of its own, so a frame that was taken back but arrives cannot overwrite a newer move.

## Tests

`?manual-clock` in the address gives a clock that runs a frame only when `window.__msAdvanceFrame(n)` is called, and `window.__msPendingFrames()` says how many wait.
The hooks come off the window when the page is left.
A browser test makes a write fail by overriding `Element.prototype.setAttribute` for a name, and makes the swap throw by overriding `replaceChildren`, before it changes the host or after.

An independent check reads the page: it learns the plot's own mapping from the outline of a cell, whose corners are whole numbers, and turns each vertex of each contour back into the grid's coordinates.
Each must be on a grid edge, at the place where the interpolation of the two samples gives the level, which is worked out in the test from the field's formula.
It does not use the solver.
The solver is also checked in native tests against a second reading of the same grids, which does not use the case table or the stitcher.

### Phase 0: what was measured

| Probe | Result |
|---|---|
| The plot's DOM | Lines are `<polyline>` elements with a `points` attribute that holds each number as text. There is no `<path>` |
| Precision | The vertices read from `points` are right to about 1e−13 grid units, and the renderer's `getPointAtLength` to about 1e−6, which is 32-bit floating point. The tests use 1e−9 and 1e−5 |
| Clipping | With the axes padded by 0.25, every open contour has both ends on the boundary, at every level |
| Closed seam | A loop is drawn as a polyline that repeats its first point, with a dot of radius half the line width at the seam |
| Legend | Three entries, each with its name and pattern |
| Zoom and pan | The toolbar and the keyboard work in a `<svg>` nested in the stage. The label says the zoom, the geometry is the same at 25%, 100% and 400% and after a pan, and the painted box of each contour is within 0.5 pixels of its vertices through the transform |
| Stage controls | The size of the buttons and the range is the same at every zoom |
| Hidden candidate | The scene is built attached and hidden, view carried over, and swapped. A failed build, set of the view or swap leaves the old scene, its view and its focus |

Not checked: how the seam dot looks on a screen reader, since the plot is read as an image from its description, and a screenshot of every zoom limit.
Selectors were not tested with the arrow keys on a closed `<select>`, which headless Chrome on macOS does not change, and so they are reached by Tab and changed by their events.
