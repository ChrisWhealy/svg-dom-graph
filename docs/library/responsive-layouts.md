# Responsive Layouts

The toolbar, the scene title and the surface for the gestures are laid out against the visible area of the `<svg>`.
The layout happens at the moment they are created.

***IMPORTANT***<br>
**Responsive layouts.**<br>
The scene cannot observe its `<svg>` being resized.
The toolbar and the gestures' surface are laid out against the visible area at the moment they are created.
They stay there until `refresh_layout` is called.
Nothing reports an error when they go stale, so the failure is easy to miss.

Whether a refresh is needed depends on how the `<svg>` is sized:

| The `<svg>` | When its size changes | Call `refresh_layout`? |
|---|---|---|
| has a `viewBox`, and its CSS size changes but not its shape | The browser scales the whole `<svg>`, toolbar included | No |
| has a `viewBox`, and its CSS size changes shape | The visible area changes, and the toolbar keeps its old place | **Yes** |
| has a `viewBox`, and the `viewBox` itself changes | The toolbar keeps its old place | **Yes** |
| has no `viewBox`, and its size changes | The toolbar keeps its old place | **Yes** |

So a responsive page whose `<svg>` keeps its aspect ratio, and has a `viewBox`, needs no refresh as its CSS size changes.

A stale layout is not only cosmetic.
The surface for panning and wheel zoom is sized the same way.
So if the `<svg>` grows and the layout is not refreshed, those gestures stop working in the new area.

**The visible area.**<br>
The toolbar, the gestures' surface, and the centre that `zoom_in` and `zoom_out` zoom about are all placed together.
Each goes against the part of the `<svg>`'s user space that is actually on screen.
That is not always the `viewBox`, and is found by mapping the rendered box back into user space through the `<svg>`'s own screen matrix:

- A `viewBox` origin other than `(0, 0)`, such as `-500 -300 1000 600`, is honoured, so the centre is the true centre and not `(width / 2, height / 2)`.
- `preserveAspectRatio="meet"`, the default, shows *more* than the `viewBox` when the `<svg>` is a different shape, so the toolbar sits on the real edge.
- `preserveAspectRatio="slice"` shows *less*, so the toolbar stays inside the cropped area instead of falling outside it.
- CSS scaling is accounted for, so a pan or wheel zoom moves the content by user units, not pixels.
- An `<svg>` sized purely by CSS, with no `viewBox` and no size attributes, is measured by its rendered size.
  `svg-dom` alone would report `0 × 0` for it, since it only reads the `width` and `height` attributes.
