# Demo changelog

Changes to the demo gallery (`demo-app`, `demo-common`, `demo/`), the demo server, the CDP tests and the docs.
These have no version of their own, and do not change the version of the library. Changes to the library itself are in [`changelog.md`](../changelog.md).
Entries before the library's 0.2.47 are in that file, mixed with the library's own.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Angle curve, labelled &theta;, in the Pythagoras demo
- Pythagoras demo text beside the scene

### Changed

- Demo panels are grouped and ordered by the folders under `demo/panels/` (`NN-group-name/NN-demo-name.html`), not by `data-group` and `data-order`
- `svg_buttons` and `sliders` moved from `demo-common` to the library, as `svg_dom_graph::controls`

### Fixed

- Pythagoras drag test rounds the mouse position to whole pixels, so that CI reads back the angle that was dragged to
