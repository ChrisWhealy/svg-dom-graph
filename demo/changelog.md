# Demo changelog

Changes to the demo gallery (`demo-app`, `demo-common`, `demo/`), the demo server, the CDP tests, the docs and the performance harness.

These have no version of their own, and do not change the version of the library. Changes to the library itself are in [`changelog.md`](../changelog.md).
Entries before the library's 0.2.47 are in that file, mixed with the library's own.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Game of Life demo: a second row of spaceship buttons under the board
- Convolution demo panel, after Phase Shift: a signal of two sine waves probed with a cosine and a sine
- Angle curve, labelled &theta;, in the Pythagoras demo
- Pythagoras demo text beside the scene
- Conway's Game of Life demo (`eb3dcd8`)
- N-Queens demo panel, in a new "Backtracking" group (`572ecd9`)
- Implement a demo of the Edmonds-Karp algorithm (`dcdba3e`)
- Cleanup module to remove old Chrome processes after CDP tests (`ca48276`)
- Red-black tree demo panel, after Edmonds–Karp: six insertions and a two-child deletion, as a real Scene with a semantic tree above it
- Implement Red-Black Tree demo (`3a900bd`)
- Implement marching squares demo panel, in a new "Computational Geometry" group (`421dfb1`)
- Add a Petri Net demo (`eb06cfa`)
- Add a Tarjan's Strongly Connected Components demo (`78e3e2c`)
- Performance harness design spec and Phase 0 spike, schemas and findings, in `perf/` (`84a9599`)
- Perf Harness: Implement effective-registration accounting in listener-tally spike (`2fc8997`) 
- Perf Harness: Retain spike results as reproducible evidence (`89a03af`) 
- Perf Harness Phase 1: Deterministic workload engine `perf-harness` (`a2c35b3`)
- Perf Harness Phase 2: Layer A and Layer B runners `perf-harness-wasm` (`78eacc4`)
- Perf Harness Phase 3: Layer C browser runner and gallery adapter (`038b041`)
- Perf Harness: Provide completion evidence of gallery sweep (`1d2c34f`)
- Perf Harness: Implement accessibility checks to cover Chrome AX tree (`be3b0b1`)
- Perf Harness: Gallery walk tests have "pass", "inconclusive" and "fail" outcomes (`4857001`)
- Perf Harness: Phase 4: Recovery and endurance testing under deliberate failures (`778ccbc`)
- Perf Harness: Phase 5: CI integration, baseline promotion and statistical regression reporting (`ae4c4b1`)
- Discrete-Event Service Simulator (DESS): S0 implementation (`fe0c7b5`)
- DESS S1: Pure Rust `service-sim-engine` (`72adf2b`)
- DESS S2: Long-lived fixed Scene, overlay, reserved data node; exact incremental projection; class compatibility test (`1c599ba`)

### Changed

- Convolution demo: Show that the conventions are those of the FFT demo
- Demo panels are grouped and ordered by the folders under `demo/panels/` not by `data-group` and `data-order` tags
- `svg_buttons` and `sliders` moved from `demo-common` to the library, as `svg_dom_graph::controls`
- Red Black: Fully account for DOM rollback failures (`dd71128`)
- Update top level README (`c12dfd9`)
- Update top level README again (`a312666`)
- Update top level README again (`ef4e9fc`)
- Perf Harness: Tighten JSON Schema validation (`e54ca79`)
- Perf Harness: Treat WASM memory usage as a high water mark that does not need to shrink after teardown (`4365961`)
- Perf Harness: Close gap caused by a helper listener not running when a once listener stops propagation on itself (`a6beaff`)
- Refactor `perf/README` into multiple chapters (`65511d1`)

### Fixed

- Game of Life demo: Improve test coverage
- Convolution demo: Treat zero frequency as a special case
- Convolution demo: a scene that cannot be drawn is reported as an error
- Pythagoras drag test rounds the mouse position to whole pixels, so that CI reads back the angle that was dragged to
- Don't skip cells during rapid pointer drag movements (`468e1ee`)
- Drag ends when the pointer is let go or at `pointercancel` (`68755cb`)
- Prevent model and presentation from diverging after a DOM write failure (`1090132`)
- Prevent `Board::new` from panicing for certain board sizes (`a6fcb7c`)
- Prevent simulation timing from behaving unexpectedly at invalid inputs (`4376156`)
- N-Queens: Make board replacement fully transactional (`81dbca9`)
- N-Queens: Prevent real-time scheduler from silently accepting timer creation failures (`9204f7a`)
- N-Queens: Release manual-clock browser hooks (`6c92cdd`)
- N-Queens: Associate seek requests with a trace generation number. Ignore earlier generation requests (`82542bf`)
- Edmonds-Karp: Implement 5 seperate review fixes (`7076d8e`)
- Edmonds-Karp: Initial Play state failure stops playback from starting (`16c5a72`)
- Prevent keyboard focus movement from failing silently (`e000303`)
- Prevent pointer selection and actual focus from diverging (`24f0f6e`)
- Red Black: Prevent delete fix-up descriptions from preceding their actions (`6adb2a6`)
- Red Black: Correct review findings (`7052aed`)
- Marching Squares: Repair after a successful scene swap can restore the wrong field geometry (`2c49afd`)
- Marching Squares: Pointer-centred zoom and pointer panning need regression coverage (`885af74`)
- Marching Squares: Reconstruction generation identifiers are reused after failure (`9e638c5`)
- Petri Net: Stop playback on failed selection or wiring update (`1bcc0f4`)
- Petri Net: Quick-fire can proceed after its selection draw fails (`71b617b`)
- Petri Net: Teardown does not fully release listeners (`413be15`)
- Perf Harness: Result schema fully represents statistical contract (`1d6a367`)
- Perf Harness: Wait for timing segment to close before `step()` updates reference model (`56c2522`)
- Perf Harness: Include cycles that run a checkpoint in the endurance series (`0d5d634`)
- Perf Harness: Explain retained allocations in endurance test results (`cb03518`)
- Perf Harness: Performance-mode correctness checks cover the measured work (`8736347`)
- Perf Harness: Record attribution strength of the cause of a screen repaints (`80e9916`)
- Perf Harness: Prevent failed DOM calls from retaining externref slots (`61499c9`)
- Perf Harness: Prevent failed DOM calls from retaining externref slots (`e30bb44`)
- Perf Harness: Remove grab cursor for a node whose scene is dropped mid-drag (`19f3ac4`)
- Perf Harness: Correct CI test step order (`9630405`)
- Perf Harness: Correct CI test that tried to delete a non-empty Chrome temp profile directory (`fc1bfab`)
- Perf Harness: Ensure all test processes terminate after tests complete (`133d529`)
- DESS: Prevent stale timer callbacks from erasing the current timer handle (`d8d63b6`)
- DESS: Prevent layer C protocol from accepting incomplete measurements (`2bb307c`)
- DESS: Ensure frozen generator parameters are independently pinned (`883bcae`)
- DESS: Classify late cancellation events as stale (`1003933`)
- DESS S1: Prevent duplicate job keys from redirecting cancellation (`9be023b`)
- DESS S1: Prevent generator parameters and engine configuration from disagreeing (`62e2798`)
- DESS S1: Ensure coverage report accurately states path-specific checks (`3794f8a`)
- DESS S2: Prevent detached-host writes from being accepted as successful (`ad02b75`)
- DESS S2: Projection preparation has an unbounded per-slice cost (``)