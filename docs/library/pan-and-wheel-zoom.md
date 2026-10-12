# Pan and Wheel Zoom

Two gestures change the view without the toolbar: dragging empty background pans it, and holding Ctrl or Cmd while turning the mouse wheel zooms it.
A keyboard user can do both too.
The toolbar, and the zoom API that drives the same zoom, are described in [The Toolbar, Zoom and the View](toolbar-and-zoom.md).

- `set_pan_mode` and `set_wheel_zoom_mode` control two gestures, each with its own `InputMode`, that a mouse, trackpad, or keyboard can perform.
  `pan_mode`, `wheel_zoom_mode`, `pan_enabled`, and `wheel_zoom_enabled` report the settings and whether each gesture is active right now.

   - Dragging empty background pans the content, so content zoomed past the edge of the view can always be brought back.
     This works through a transparent surface behind the content layer, so dragging a node still drags only that node.
   - Holding Ctrl or Cmd while turning the mouse wheel zooms about the pointer.
     Cmd is the Mac convention and Ctrl is the Windows and Linux one.
     Browsers report a trackpad pinch as ctrl+wheel, so pinch-to-zoom works too.
     A wheel without a modifier is left alone, so the page still scrolls.

   **Keyboard**<br>
   Zooming from the toolbar can push content out of view, so the same gestures can be reached without a pointer.
   While either is active, the scene adds a keyboard focus target of its own: a transparent `<rect>` inside the `<svg>` that joins the Tab order and has the `application` role.
   The role tells a screen reader to pass keys through to it instead of keeping them for reading.

   The application's own `<svg>` is never touched.
   Whatever role, name, description, or `tabindex` it was given, often a description of the whole graph, is left exactly as it was, throughout and afterwards, so the default `InputMode::WithToolbar` is safe to use with an `<svg>` that has its own.
   The `application` role is confined to that one small control, so the nodes inside the `<svg>` keep their ordinary accessible descriptions and are read as normal.

   - The arrow keys pan the view, like scrolling: right moves the view right, so the content moves left.
     One press moves 40 units of the `<svg>`'s own space, which is the same distance on screen at any zoom, and Shift moves five times as far.
     They belong to `pan_mode`.
   - `+` (or `=`), `-`, and `0` zoom in, zoom out, and restore the original zoom, about the centre of the visible area.
     They belong to `wheel_zoom_mode`.

   Only a key pressed while the focus target has focus is handled, so an arrow key on a focused toolbar button is left to that button, and a key sent to the `<svg>` itself is ignored.
   Keys held with Ctrl, Cmd, or Alt are left alone, so the browser's own page zoom keeps working.
   Everything that is handled has its default action cancelled, so the page does not also scroll.

   The focus target's accessible name says how far it is zoomed, such as "Graph view, zoom 125%", and its description lists the keys that are active.
   The zoom is part of the name and not a live region, so a run of zoom steps never interrupts a screen reader with an announcement.
   It is read the next time the scene takes focus.

   The focus target draws nothing until it has keyboard focus.
   Then it outlines the visible area, so it is obvious where focus is.
   It takes no pointer events, so it never gets in the way of panning.
   It is removed, with its role, name, description, tab stop, and key listener, when no gesture needs it.

   `InputMode::WithToolbar`, the default, makes a gesture active only while a toolbar is shown.
   `InputMode::On` makes it active whether or not a toolbar is shown, and `InputMode::Off` never activates it.
   So an application that supplies its own controls can call `hide_toolbar` and still keep both gestures, or switch either one off with the stock toolbar showing.
   The two gestures are independent of each other.

   A trackpad pinch delivers many small wheel events rather than one notch.
   Each event zooms in proportion to its `deltaY`, converted from lines or pages to pixels where the browser reports them that way.
   One 100 pixel notch is one 1.25 step, and ten 10 pixel events compose to exactly the same zoom, so a pinch never jumps straight to a limit.

   Wheel and pan events update the view at once, but write the DOM once per animation frame, the same way a dragged node's moves are coalesced.
   Releasing a pan writes its final position immediately.

   `refresh_layout` resizes the gestures' surface, and repositions the toolbar if there is one, after the `<svg>`'s size or `viewBox` changes.
   `refresh_toolbar_layout` does the same and is kept for compatibility.

## Changing the view during a gesture

The view can change at any moment: the wheel, the keyboard, a toolbar button, or the application calling `zoom_in` or `reset_view` can all change it while a node is being dragged or the background is being panned.
These compose with the gesture rather than being blocked or corrupting it, so whatever changes the view, the gesture carries on tracking the pointer.

- A node drag holds the point it was grabbed at under the pointer.
  After a zoom that point is over a different part of the content, so on each move the pointer's position is re-read under the view as it is now.
  Zooming with the wheel at the pointer holds the grabbed point still, so 25 pixels of further movement at 1.25× is 20 units.
  A zoom about the centre of the view, such as from `zoom_in`, moves the grabbed point on screen, and the node follows the pointer to it.
- A pan applies each move to the view as it is now, by how far the pointer has moved since the last move.
  A zoom made in the middle of it is kept, and the pan carries on from the new view.

A zoom from the wheel, keyboard, or a button is written to the DOM one animation frame after it is made.
A drag that begins inside that frame settles it first, so the node's screen matrix is never read from a picture of the view that is one frame old.
