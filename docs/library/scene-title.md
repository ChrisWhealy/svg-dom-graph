# The Scene Title

A scene title is a heading-style label for the whole scene.

- `show_scene_title`/`hide_scene_title` draw a heading-style title for the whole scene, as a sibling of the content layer, so zooming and panning never move it.
  `has_scene_title` reports whether one is shown.
  `SceneTitleOptions` sets its edge, margin, font size, weight, underline and `aria-level`, the last of which lets a host match the heading structure of the page around the scene.
