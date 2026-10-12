# Browser Tests: View Robustness

These tests belong to the browser integration tests in `tests/drag/`; see [Browser Integration Tests](browser-tests.md) for how they are run.

- teardown between an input and its animation frame, checked by watching the page for uncaught errors:
  - a wheel zoom, then the toolbar hidden before its frame
  - a pan move, then panning switched off before its frame, which rebuilds the gestures
  - a wheel zoom, then wheel zoom switched off before its frame
  - a wheel zoom, then every `Scene` handle dropped before its frame
  - a node drag move, then the scene dropped before its frame
  - in each, nothing throwing, and where there is a view to check, what is drawn agreeing with `zoom_scale()` or the pan made so far
- the toolbar's lifecycle, so that showing and hiding it repeatedly leaves no stale handler:
  - the sequence show, hide, show, hide, show, after which each kind of input does exactly one thing: one button click is one zoom step, one arrow key press is exactly 40 units, one `+` key is one step, one wheel notch is one step, and a 40 pixel drag is 40 units
  - the same after many more cycles that also move the toolbar to every edge and replace it while it is shown
  - the same after the input modes are switched off, on, and back again many times
  - a scene with every handle dropped answering no input of any kind, checked in the rendered DOM because no handle is left to read the zoom from
- listener lifetime, checked inside the crate by taking a `Weak` reference to the scene's shared state and asserting it cannot be upgraded once every handle is dropped:
  - a scene with draggable nodes, the toolbar, and both gestures switched on
  - a clone keeping the state alive until the last handle goes
  - repeated show, hide, and mode changes leaving nothing holding the state
  - a pending animation frame not keeping the state alive
  - a scene whose toolbar has been hidden
- changing the view in the middle of a pointer gesture, which must compose with it:
  - a node drag, then a wheel zoom at the pointer, then more dragging: 25 pixels at 1.25× is 20 units, so the node ends where it should and not 5 units further
  - the same with the application calling `zoom_in`, which zooms about the centre and so moves the grabbed point on screen, so the node has to follow the pointer to it
  - a pan, then a wheel zoom, then more panning, and the same for `zoom_in` and for the keyboard, so the zoom is kept and not overwritten by the next move
  - a node drag with the "100%" button pressed in the middle, so the pointer is over a different point of content
  - a drag beginning in the same frame as a wheel zoom that has not yet been written to the DOM, which needs the pending write settled first
- a view change being a transaction, with `Element.setAttribute` made to throw for chosen attributes so that failures which almost never happen for real are exercised:
  - a zoom whose `transform` cannot be written reporting the error and leaving `zoom_scale()` as it was, with nothing drawn, and the next zoom being one step and not two
  - the same for a reset, which keeps the zoom that is drawn
  - a failed change falling back to a view that is still waiting for its frame, which the waiting frame then draws
  - a failure writing only the accessible name, or only a button's state, neither failing nor undoing the zoom, and the next change putting it right
  - a frame whose write fails leaving the view waiting, and the next write catching up
- performance of zoom and pan:
  - a pan frame, keyboard panning, a wheel burst, and every kind of zoom step making no `getAttribute` call at all, checked by counting calls to `Element.prototype.getAttribute` with a counter that is itself tested
  - this needs its own test because reading an attribute back costs a call across the WASM and JavaScript boundary and a `String`, which a `MutationObserver` cannot see, since it reports only writes
  - every route into the view — buttons, wheel, drag, and keyboard — changing only the content layer's own `transform`, checked with a `MutationObserver` that watches the content layer and everything beneath it, so no node, connector, label, or marker is ever rewritten
  - a burst of wheel and pan events inside one frame writing nothing until that frame, and then exactly once
  - every node's position and every connector's path being exactly what they were after zooming and panning
