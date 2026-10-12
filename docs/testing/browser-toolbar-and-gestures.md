# Browser Tests: The Toolbar and Gestures

These tests belong to the browser integration tests in `tests/drag/`; see [Browser Integration Tests](browser-tests.md) for how they are run.

- the scene title (`Scene::show_scene_title`):
  - the drawn attributes, bold and underlined by default, and placement against each edge
  - replacing an existing title, hiding it, and rejecting invalid options

- the toolbar and view controls (`Scene::show_toolbar`, `zoom_in`/`zoom_out`/`reset_view`):
  - the bar existing only while shown, and sitting after the content layer as a sibling, never inside it
  - placement against each of the four `Side`s, and the bar's `aria-orientation` following the edge
  - zooming changing the content layer's `transform` while the bar's own position and every button's size stay unchanged
  - zoom about the centre of the visible area keeping that point fixed
  - click, Enter, and Space activating a button, and other keys not doing so
  - each button being focusable and named, and a button at its limit reporting `aria-disabled`
  - rejecting invalid `ToolbarOptions` while leaving an existing toolbar alone
  - a button doing nothing, rather than panicking, once every `Scene` handle has been dropped
  - dragging a node under zoom moving it by the pointer delta divided by the scale
  - dragging empty background panning the content at any zoom, following the pointer and stopping on release
  - a pan ignoring a second pointer and a non-primary button, ending on `pointercancel`, and not starting when a node is dragged
  - the 100% button being enabled by a pan and undoing it
  - a pan's DOM write being deferred to one animation frame while it runs, and settled immediately on release
  - Ctrl+wheel and Cmd+wheel zooming about the pointer, the wheel's direction and size setting the zoom's direction and amount, and the zoom working over a node
  - a wheel without a modifier neither zooming nor being cancelled
  - a burst of small wheel events composing to the same zoom as one large event, with the DOM written once, on the next frame
  - a button pressed while a wheel burst is still waiting for its frame winning over the pending write
  - by default, pan and wheel handling being present only while the toolbar is shown, and not stacking after it is shown again
- pan and wheel zoom modes (`InputMode`), independent of the toolbar:
  - both defaulting to `WithToolbar`, and following the toolbar on show and hide
  - both being forced `On` with no toolbar at all, for an application that supplies its own controls
  - a forced-on gesture surviving `hide_toolbar` while a following one does not
  - a gesture being forced `Off` while the toolbar is shown, without affecting the other
  - no surface existing at all when both gestures are off
  - only the requested gesture being wired, so forcing pan on does not enable wheel zoom
  - switching a gesture back off removing its surface and its wheel listener from the content layer
  - setting the same mode again not rebuilding the surface
  - `refresh_layout` resizing the surface with no toolbar shown

- pan and node dragging never being confused for each other:
  - dragging a selected node moving only that node, leaving its selection text and cell colours untouched, and not panning
  - a toolbar button never starting a pan, but still clicking
  - the pan cursor returning to idle after a release and after a cancel, and a new pan starting cleanly after either
  - showing and hiding the toolbar, and toggling the input modes, repeatedly leaving exactly one surface and no leftover handler
    A leaked wheel listener cannot zoom, because it holds only a weak reference to a surface that is gone, but it would still cancel the event.
    So the tests tear everything down at the end and check that a modified wheel is no longer cancelled by anything.
