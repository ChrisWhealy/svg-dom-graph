# Browser Tests: Accessibility and Layout

These tests belong to the browser integration tests in `tests/drag/`; see [Browser Integration Tests](browser-tests.md) for how they are run.

- accessibility of the toolbar and the view:
  - every button having the `button` role, an explicit name that does not depend on its visible "100%", and a place in the Tab order
  - a focused button drawing a thicker, differently coloured border, and blur restoring it
  - activating an `aria-disabled` button, by click, Enter, or Space, doing nothing at all, and writing nothing to the DOM
  - the scene adding a keyboard focus target of its own: a `<rect>` with the `application` role
  - the focus target having a name that reports its zoom, and a description of its keys
  - none of those going on the application's `<svg>`
  - the application's own `<svg>` role, `tabindex`, name and description being untouched
  - the same holding throughout a sequence of showing, zooming, panning, changing every mode, moving, and hiding the toolbar
  - a `MutationObserver` on the `<svg>`'s attributes checking this, and recording nothing
  - a key sent to the `<svg>` itself being neither handled nor cancelled
  - the focus target drawing nothing until it has focus, then outlining the visible area, and blur removing the outline
  - the focus target taking no pointer events, lying between the pan surface and the content layer, covering the same area, and leaving panning working
  - that name following a zoom made by a button and one made by the wheel
  - the arrow keys panning like scrolling, 40 units a press at any zoom, and Shift making it five times further
  - plus, minus, and zero zooming from the keyboard
  - unrelated keys, Tab, and keys held with Ctrl, Cmd, or Alt being neither handled nor cancelled, so browser page zoom and focus movement still work
  - a key pressed on a focused toolbar button not panning the view, although the event bubbles through the `<svg>`
  - the arrow keys following `pan_mode` and the zoom keys following `wheel_zoom_mode`
  - hiding the toolbar removing the whole focus target, with its tab stop, role, name, description, and key listener, and leaving the `<svg>` as it was
  - zooming adding no live region, and rewriting no button state attribute that did not change
- responsive layouts:
  - an `<svg>` sized purely by CSS, with no `viewBox` and no size attributes, being laid out against its rendered size
    - not the `0 × 0` that `svg-dom` caches for it
  - a resize not being picked up until `refresh_layout` is called, which pins down the documented requirement so it is not mistaken for a bug
  - an `<svg>` with a `viewBox` needing no refresh when only its CSS size changes
- the visible area under different `viewBox` origins, aspect ratios, and CSS scales:
  - a `viewBox` centred on the origin (`-500 -300 1000 600`), where zooming about the centre must leave the translation at zero rather than zooming about `(width / 2, height / 2)`
  - a `viewBox` with a positive origin (`100 50 400 300`) on both axes
  - the toolbar's position and the surface's origin and size under both of those
  - pointer-centred wheel zoom holding its point under a non-zero origin and a CSS scale together
  - `preserveAspectRatio` `meet`, where more than the `viewBox` is visible, and `slice`, where less is, with the toolbar staying on the visible edge in both
  - panning under a CSS scale moving the content by user units, not pixels
  - reset restoring the identity
