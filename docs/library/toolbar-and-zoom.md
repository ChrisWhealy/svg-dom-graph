# The Toolbar, Zoom and the View

The zoom toolbar is a fixed-size bar of buttons along one edge of the scene.
The pan and wheel-zoom gestures, which the toolbar also switches on, are described in [Pan and Wheel Zoom](pan-and-wheel-zoom.md).
How the toolbar is kept in the right place when its `<svg>` changes size is described in [Responsive Layouts](responsive-layouts.md).

- `show_toolbar`/`hide_toolbar` add and remove a fixed-size button bar along one edge of the scene — see `ToolbarOptions`.
  `has_toolbar` reports whether one is shown.

   The bar holds three zoom buttons: "+", "−", and "100%".
   "100%" returns the view to its original scale and position, and its accessible name is "Reset zoom".
   `ToolbarOptions::edge` takes a `Side` and fixes the bar to the north, south, east, or west edge.

   The bar is a sibling of the content layer, not a child.
   So it stays the same size however far the content is zoomed, and draws on top of it.
   Tab reaches each button and Enter or Space activates it.
    A button that could currently do nothing is dimmed and marked `aria-disabled`, but stays focusable.
    So keyboard focus is never lost from under someone who has just pressed into a limit.
   Activating a disabled button does nothing.
   A focused button draws a thicker, differently coloured border.
   So keyboard focus is obvious in every browser, rather than depending on the browser's own outline for an SVG element.
    Each button has the `button` role and an explicit name: "Zoom in", "Zoom out" and "Reset zoom".
    So a screen reader does not read the "100%" that the button shows.

   `set_toolbar_edge` moves the bar to another edge.
   `refresh_layout` repositions it after the `<svg>`'s size or `viewBox` changes — see [Responsive Layouts](responsive-layouts.md), since forgetting this fails silently.

   Showing the bar also switches on two further gestures that work anywhere in the scene.
   They are not part of the toolbar, and are described in [Pan and Wheel Zoom](pan-and-wheel-zoom.md).

   The 100% button undoes a pan as well as a zoom.
   Hiding the bar switches off whichever gestures are set to follow it, but leaves any forced on.

- `zoom_in`, `zoom_out`, `reset_view`, and `zoom_scale` drive the same zoom the buttons do, with or without a toolbar.
  Each step scales by 1.25 about the centre of the visible area, between a scale of 0.25 and 4.0.

   Every node, connector, and port marker lives in one content `<g class="svg-dom-graph-content">`, and zooming and panning set that group's `transform`.

- `view` and `set_view` read and restore the zoom and pan as a `ViewTransform`.
  A host that draws a scene again, because a step cannot update it in place, carries the view over.
  So the zoom and pan are not reset.
  `set_view` is a transaction, like `zoom_in`: if the `transform` cannot be written, nothing changes.
