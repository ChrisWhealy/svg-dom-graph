# Gallery panel, P2: scaffold and controls

The specification is `design-specs/dess_demo.pdf`, revision 4, sections 2 to 6 and 11.
P2 puts the simulator in the gallery as a real route with real controls, and proves it with G01 to G10.
It offers the walkthrough only.
Switching to another scenario is P3.

## What P2 adds

| Piece                                            | Where                                                                                 |
| ------------------------------------------------ | ------------------------------------------------------------------------------------- |
| The fragment, numbered after Petri Net           | `demo/panels/16-concurrency/02-service-simulator.html`                                |
| The registry entry and the source frame          | `demo_gallery!` in `demo-app/src/lib.rs`, `service_sim::build_service_simulator_demo` |
| The controls, the inspector and the page's hooks | `demo-app/src/service_sim/controls.rs` and `mod.rs`                                   |
| The page side of a session                       | `demo-app/src/service_sim/dom.rs`, which implements `SessionHost`                     |
| The catalogue and its preflight                  | `service-sim/src/catalogue.rs`                                                        |
| The acceptance tests                             | `dess/demo-panel/test-gallery.mjs`                                                    |
| The CI job                                       | `dess-gallery` in `.github/workflows/ci.yml`                                          |

No simulator rule is in `demo-app`.
The engine is `service-sim-engine`, and the projection, the surface, the controller and the coordinator are `service-sim`.

## The catalogue and the preflight

`service_sim::catalogue::catalogue()` lists the four scenarios of design D1 in order.
Each has the totals that it is known to come to, and its ceilings are those totals.

| Id      | Servers | Slices | Pops   |
| ------- | ------- | ------ | ------ |
| E0      | 2       | 12     | 19     |
| E3-NOOP | 1       | 9      | 9      |
| scaled  | 3       | 487    | 1,009  |
| W6      | 3       | 5,464  | 11,214 |

A native test replays all four to their ends and finds exactly these totals.
The gallery replays the first one before it builds anything, and the range's maximum is the number that the replay found.
Time is never a reason to refuse a run.

## The page

The first scene is built by the P1 coordinator, through `DomSessions`, into a staged host, read back, and then published.
That is the path that P3 extends to another size, so the first build already takes it.

Each control is one request to the controller, and the page shows what the controller committed once the call has returned.
Nothing is shown from the request, and no second state is kept.

- **Previous and Next** are `aria-disabled` at a boundary, with a reason that is read with them, and are never `disabled`.
  A click on one still reaches the controller.
- **Play and Pause** are one toggle.
  At the end that the preflight found, it asks for a step and not for playback, so no timer is made and the button never reads Pause.
- **Seek** listens for `input` only, so each arrow key is one request and a drag's last `change` does not repeat it.
  The range says what was asked for beside the snapshot that is still committed.
- **Repair display** redraws the committed snapshot and moves nothing.
- **Inspect a job** is a native list of the jobs in the picture.
  A job is selected by the controller, and the outline, the inspector and the accessible names follow its committed selection.

The page installs one watch on the gallery's stage and one set of lifecycle listeners, and both reach the current controller through the coordinator's weak target.

## G01 to G10

`node dess/demo-panel/test-gallery.mjs` serves the staged gallery, and drives it in one Chrome with real clicks and keys.
The oracle is the independent JavaScript engine, which shares no code with the page's.
Timers and frames are held by the page-level injector, so that no test waits for an interval, and a DOM call is made to fail by the same injector.
One test lets the browser's own timer run once at 1 Hz, and says how long that took without judging it.

| Test | What it shows                                                                                                                                                                                    |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| G01  | The menu, the deep link and the source frames agree with the registry                                                                                                                            |
| G02  | A revisit adds no SVG, host, observer, listener or id                                                                                                                                            |
| G03  | The first draw is snapshot 0 of the walkthrough                                                                                                                                                  |
| G04  | Twelve Next clicks match the engine at every snapshot, and the data node's ordinal advances by one                                                                                               |
| G05  | Previous, Restart and seeks equal a full redraw, and five arrow keys are five requests with the last one winning                                                                                 |
| G06  | One slice per timer at 1, 2, 5 and 10 Hz, rate replacement, and Play at the end of a clean and of a dirty page                                                                                   |
| G07  | A timer or frame that was taken back is gone, and the last request wins                                                                                                                          |
| G08  | Repair restores the picture, the overlay and the Play control, after a failure at each of five writes after a failed write of the Play control, and after a failed write of a boundary attribute |
| G09  | Selecting and clearing a job agree across the scene, the inspector and the accessible names                                                                                                      |
| G10  | Names, roles, aria-disabled, Tab order, Enter, Space, arrow keys, focus retention and no shortcut                                                                                                |

`dess/demo-panel/p2/mutation-check.mjs` breaks the page in 15 ways, one at a time, in a copy of the tree, and a test has to fail for each.
Its result is in `evidence/mutation-check.json`, and the run of the ten tests that was measured is in `evidence/gallery.json`.
It rebuilds the demo for each mutation and takes about half an hour, so CI does not run it on every commit.

## What P2 does not do, and why

- **W6 on the arrow keys.** G05 asks for rapid arrow keys on the 5,464 snapshots of W6.
  That scenario has three servers, and at P2 the page could not yet switch from two, so the keys were run on the walkthrough's 12.
  The W6 run is in P3.
- **A stale callback that the browser still calls.** G07 cannot call one, since the page's scheduler frees a closure that was taken back, so the real page has none to call.
  The native controller tests do it, with a scheduler that keeps cancelled tasks.
- **Pause with a seek waiting.** The page has no way to pause while a seek is waiting, since a seek stops playback.
  That case is native.
- **What a screen reader says for the range.** Chrome's DevTools accessibility tree reports a range as its number and ignores `aria-valuetext`.
  A custom slider is reported the same way, so the attribute is checked, and a reading by a screen reader is a manual observation for P4.
- **Another browser.** Chrome is the gate.
  Firefox and Safari are observations, and none has been made.
