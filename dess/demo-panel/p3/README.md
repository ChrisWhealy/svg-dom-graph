# Gallery panel, P3: lifecycle and switching

The specification is `design-specs/dess_demo.pdf`, revision 4, sections 6, 7, 9 and 12.
P3 lets a person choose any of the four scenarios, and makes the panel behave when it is hidden, left and shown again.
Its acceptance tests are G11 to G17, and G05's run on the advanced scenario that P2 could not make.

## Switching

The scenario list is a native `<select>`, with the advanced scenario in a group of its own.
Choosing one is a request to the coordinator that P1 built, and `switching.rs` writes what it decided.

- **The same number of servers** is shown in the same scene.
  Nothing is replaced, and the scene's elements keep their identity.
- **Another number** is built into a staged host, read back, and handed over.
  The old scene is removed only after that.
- **Anything that fails** leaves the old scenario on the page, at the snapshot it was at, with its selection and its picture.
  The list goes back to it, the error area says which stage failed, and a repair restores the picture exactly.

The exact end of the run is found before anything is built, and it is what the range runs to.
A person's zoom and pan are carried to the new scene, and a focus that was inside the old scene moves to the scenario list.
Pan and zoom are on for the scene, since it has no toolbar: dragging the background pans, and the wheel with Ctrl or Cmd held zooms.

## Hiding, leaving and returning

The page installs one watch on the gallery's stage and one set of lifecycle listeners, once.
Each reaches the current controller through the coordinator's weak target, so a replacement does not need new ones.

- **A panel that is hidden** is seen by the observer, and the controller stops and takes back what it had scheduled.
- **Coming back** never resumes.
  The page says "Paused while hidden.
  Press Play to resume." and keeps saying it until a person acts.
- **A document that is hidden** stops Play by the same path, and a later visible event does not resume it.
- **A hide and a show of the section in one task** are one batch to the observer, which sees only a panel that is shown, so playback may go on.
  That is permitted, and the tests only require that there is no extra tick, observer, listener or host.

## Layout

The controls and the inspector are `foreignObject`s of the scene's own `<svg>`, outside the content group, so they do not zoom or pan with the scene.
The view box is the surface's own, with a band above it for the controls and a column beside it for the inspector, so the surface's coordinates do not move.
Nothing can wrap below the scene, since everything scales together.

The controls are written once in the panel's markup, and are moved into each scene as it is shown, which keeps their listeners, their state and their focus.
A scene that is removed takes nothing with it that the page needs, since the controls are put back first.

The scene's nodes are placed by the number of servers.
The dispatch node's midline is the common midline of the column of servers, whatever their number.
Every connector leaves the dispatch node on its east side and enters its server on the west side.
Each counter's value has room for the longest name.

The eight cells of the data node are two to a row, each wide enough for six digits, and each row is named beside it.
The library sizes a numeric cell by the number that it is made with.
So the cells are made with a six-digit number, and the first draw writes the real ones.

## G11 to G17

`node dess/demo-panel/test-gallery.mjs` runs them with G01 to G10, in one Chrome, as P2 did.

| Test | What it shows |
|---|---|
| G05W6 | Five arrow keys on the 5,464 snapshots are five requests with the last one winning, and a seek to the end arrives |
| G11 | Routing away with a held Play timer stops it, routing back stays paused, and the observer, listeners and host are unchanged |
| G12 | A hidden document stops Play and a visible one does not resume it, and a hide and a show in one batch add nothing |
| G13 | Hiding during a seek that is waiting leaves no frame, and a later seek succeeds |
| G14 | A host detached in the middle of a draw, or before one, commits nothing, and a full repair follows |
| G15 | A switch to the same size is atomic and in place, and a failure at three writes leaves the old scenario, selection and picture |
| G16 | The sizes 2, 1, 3 and 2 each keep one scene in the document and the accessibility tree, and 38 failures through the whole switch roll back |
| G17 | The no-op scenario's nine slices match the independent engine, and its end commits nothing |
| L01 | The controls and inspector are in the scene at any width, the nodes and connectors are laid out as designed, and the counters have room |

G16 finds the number of DOM writes in a switch from a switch that succeeds, and fails writes spread through it.
Every failure that fell in the switch rolled back to the old scene, and the failures reached the build, the first draw and the handoff.
The writes that did not roll back fell after the commit, in the page's own refresh.

The native coordinator tests add a matrix.
Each way a replacement can go wrong, at each transition on the way 2, 1, 3, 2, leaves the old session on the page, shown and focused.

`dess/demo-panel/p3/mutation-check.mjs` breaks the page in 14 ways, one at a time, in a copy of the tree, and a test has to fail for each.
Its result is in `evidence/mutation-check.json`, and the run of the tests is in `evidence/gallery.json`.

## What P3 does not do

- **G18 to G20.** The scaled and advanced runs over many commits, and the endurance of repeated switches, are P4.
- **Disposing the panel.** The gallery builds a panel once and only hides it, so the owner is never dropped.
  Its disposal is exercised by G20.
- **A stale callback after a hide.** The page's scheduler frees a closure that was taken back, so the real page cannot call one.
  The native controller tests do.
- **Another browser.** Chrome is the gate, and no other has been run.
