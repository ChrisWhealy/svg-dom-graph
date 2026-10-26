# How View Updates Are Written

Zooming and panning change one group's `transform`.
This page describes why that is cheap, what happens when the write fails, and what happens on teardown.
Teardown is when the handling behind a pending animation frame is dropped.
The zoom API itself is described in [The Toolbar, Zoom and the View](toolbar-and-zoom.md), and the gestures in [Pan and Wheel Zoom](pan-and-wheel-zoom.md).

**Performance.**<br>
Because zoom and pan change one group's `transform`, the work does not grow with the graph.
No node is moved, no connector is rerouted, and no label or marker is rewritten, so zoom and pan never invalidate any routing geometry.
A graph of a thousand nodes costs the same to zoom as one of two.

Three things keep the cost of that one write down:

- The attribute value is built in a buffer the scene reuses.
  So a run of zoom or pan updates allocates nothing for it once the buffer has grown to fit.
  The accessible name that reports the zoom is built the same way when it changes.
- Wheel and pan events update the view at once, but write the DOM at most once per animation frame.
  So a trackpad pinch that fires more events than the browser paints frames still costs one write per frame.
  Releasing a pan writes its final position immediately.
- A frame never reads from the DOM to decide whether there is anything to write.
  Reading an attribute back crosses the WASM and JavaScript boundary and allocates a `String` for the answer.
  So the scene keeps what it needs on the Rust side.
  Each toolbar button remembers whether it was last drawn enabled and writes its two attributes only when that changes.
  The scene remembers the zoom percentage its accessible name shows, so a pan, which never changes it, does not even build the text.

So a pure pan frame writes only the content layer's `transform`, and reads nothing.
A zoom frame also writes the accessible name, and the toolbar buttons only if one has just been enabled or disabled.
Every zoom step works out the centre of the visible area without reading anything back from the DOM either.
That holds whether the step comes from a button, the keyboard, the wheel or the API.

Keyboard presses are written immediately, since a held key repeats far more slowly than a pointer moves.

**When a view change cannot be written.**<br>
A view change is a transaction.
`zoom_in`, `zoom_out` and `reset_view` return an error if the `transform` cannot be written.
Nothing has then changed.
The previous zoom is kept, and `zoom_scale` still agrees with what is drawn.
A view that was already waiting for its frame is still waiting.
Once the `transform` is written the operation has succeeded.
The accessible name and the toolbar buttons that follow are bookkeeping.
A failure to update either is not reported, since the graph has visibly zoomed.
Neither is treated as up to date until its write succeeds, so the next change puts it right.

A change made by the wheel or a pan is drawn by an animation frame, which has nobody to report an error to.
If that write fails the view stays waiting.
The next thing that draws the view draws it, so the scene and the drawing agree again.

**Teardown while a frame is pending.**<br>
A wheel or pan event can leave its DOM write to the next animation frame.
The handling that asked for it can be torn down before that frame arrives.
The toolbar may be hidden, an input mode changed, or every `Scene` handle dropped.
Two things keep that safe:

- The pending frame is cancelled when the callback behind it goes.
  Otherwise the browser would call a freed callback, which throws "closure invoked recursively or after being dropped".
  The node-drag coalescer uses the same mechanism.
- Teardown writes the view first, so the graph is drawn as the last input left it and agrees with `zoom_scale()`.
  Dropping the last `Scene` handle does the same.
  The exception is a node position pushed by a drag but not yet applied when the scene is dropped, which is simply not applied.
