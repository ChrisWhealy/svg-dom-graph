# `svg-dom-graph`

## Table of Contents

- [Modules](#modules)
- [`Scene`'s Public API](#scenes-public-api)

## Modules

| Module | Description | Visibility | Tested Using |
|---|---|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics providing<ul><li>`boundary_point`</li><li>`snapped_anchor`</li><li>`clamp_to_bounds`</li><li>elbow-corner routing</li><li>zoom and pan arithmetic (`ViewTransform`)</li></ul> | public: `Side` and `ViewTransform` are re-exported from `scene`, and `geometry` itself is public. | `cargo test`
| `src/model/`  | DOM-free graph topology and node content<ul><li>`Graph`, `Node` and `Edge`</li><li>`DataNodeContent`, `NodeValues`, `DataFormat`, `GridLayout`, `ByteOrder` and `LabellingStyle`</li><li>`Selection` and the three operator enums</li></ul> | crate-private: exposes opaque `NodeId`/`EdgeId` handles, and re-exports the content types through `scene` | `cargo test`
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants | public: `Error` only | `cargo test`
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle. It also owns<ul><li>the nodes of every kind and their connectors</li><li>dragging</li><li>the zoom toolbar and the pan and wheel-zoom gestures</li><li>the selection toolbar</li><li>the scene title</li><li>nested scenes</li></ul> | public | `cargo test`, `wasm-pack test` and the CDP tests (see [Testing](../testing/README.md))

## `Scene`'s Public API

`Scene` is a cheap, cloneable handle, and its public API is grouped by area:

| Page | What it covers |
|---|---|
| [Nodes](nodes.md) | Plain, data, named data and operator nodes, and measuring, placing and changing them |
| [Floating Point Values](floating-point.md) | `F32` and `F64`, `FloatFormat`, decimal places against significant digits, special values and widths |
| [128-bit Values and UUIDs](uuids-and-128-bit-values.md) | `U128`, `I128`, `Uuid`, `UuidFormat`, the byte layouts of a UUID and why no operator accepts one |
| [Row Labels and Text Measurement](labels-and-text.md) | `LabellingStyle`, and how this crate sizes anything that contains text |
| [Selection and Marking Cells](selection.md) | `Selection`, secondary cells, unreached cells, a ring around a whole node, and the selection toolbar |
| [Connectors](connectors.md) | `add_edge`, connector types, forced sides, fixing points, and `Side` |
| [Dragging](dragging.md) | `make_draggable`, bounds and collision handling |
| [The Toolbar, Zoom and the View](toolbar-and-zoom.md) | The zoom toolbar, `zoom_in` and friends, and `view` and `set_view` |
| [Pan and Wheel Zoom](pan-and-wheel-zoom.md) | The pan and wheel-zoom gestures, their keyboard handling, and `InputMode` |
| [Responsive Layouts](responsive-layouts.md) | `refresh_layout`, and how the visible area is found |
| [How View Updates Are Written](view-updates.md) | Why zooming and panning are cheap, and how a failed or interrupted write is handled |
| [The Scene Title](scene-title.md) | `show_scene_title` and `SceneTitleOptions` |
| [Nested Scenes](nested-scenes.md) | Container nodes, `enter` and `exit` |
