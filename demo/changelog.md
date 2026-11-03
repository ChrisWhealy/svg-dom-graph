# Demo changelog

Changes to the demo gallery (`demo-app`, `demo-common`, `demo/`), the demo server, the CDP tests and the docs.
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

### Changed

- Convolution demo: Show that the conventions are those of the FFT demo
- Demo panels are grouped and ordered by the folders under `demo/panels/` not by `data-group` and `data-order` tags
- `svg_buttons` and `sliders` moved from `demo-common` to the library, as `svg_dom_graph::controls`
- Red Black: Fully account for DOM rollback failures (`dd71128`)

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
- Red Black: Prevent delete fix-up descriptions from preceding their actions (``)
