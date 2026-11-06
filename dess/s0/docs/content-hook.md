# The content group hook

The design puts the queue window in an overlay of application-owned SVG elements, as the Petri-net demo does.
The overlay is appended to the group that `Scene::new` creates, so that it pans and zooms with the nodes.
The group is found by its class, `svg-dom-graph-content`, which the library does not document.
S0 pinned what the design relies on, and decided what to promise.

## What the tests pin

The tests are in `tests/drag/content_group_hook.rs`.

| Test | Pins |
|---|---|
| `a_scene_has_one_content_group_and_its_nodes_are_inside_it` | One `<g class="svg-dom-graph-content">` is a direct child of the `<svg>`, and a node is inside it |
| `an_element_in_the_content_group_follows_the_view_as_a_node_does` | After `set_view`, an overlay element and a node scale by the same factor and keep their distance in proportion |
| `using_the_scene_leaves_an_application_element_in_the_group_alone` | Drawing nodes and edges, moving a node, setting data values and changing the view leave the overlay alone |
| `a_node_drawn_after_the_overlay_is_painted_over_it` | The group paints in append order, so the overlay is added once the scene's nodes are drawn |
| `a_second_scene_on_the_same_svg_adds_a_second_content_group` | Dropping a scene leaves its group, and a second scene on the same `<svg>` adds a second one |
| `a_thousand_commits_through_one_data_node_add_no_elements` | 1,000 commits through one data node, with a write to each of 12 overlay slots beside each, change no element count |

The view test was checked to be able to fail.
With the overlay appended to the `<svg>` and not to the group, it reports that the overlay was not scaled.

## What the tests found

The hook is one group for each scene, and not one for each `<svg>`.
A scene is not rebuilt in place, because the second scene does not replace the first scene's group.
A dashboard that rebuilds its scene must clear or replace the host first.
This matches the design, which rebuilds only for a legitimate scenario change.

## Decision

Support the class as part of the DOM, and pin it with these tests.
An application may append elements to the group, and the library leaves them alone through the operations tested.
A change to the class name is a breaking change.

The decision is not applied yet.
CI fails any change to `src/` while the version in `Cargo.toml` is the published one, and 0.2.48 is published.
So the documentation and its changelog entry are prepared here, for the next version.
The tests live in `tests/`, which the version check does not gate, and they are in place now.

### Prepared documentation

Add this section to the documentation of `Scene`, after "Keep the host `<svg>` in the document".

```rust
/// # Drawing next to the scene
///
/// `Scene::new` adds one `<g class="svg-dom-graph-content">` as a direct child of the `<svg>`. Every node, connector
/// and label is inside it, and the pan and zoom transform is written on it.
///
/// An application may append its own elements to that group, and they then pan and zoom with the scene. Drawing, moving
/// and connecting nodes, setting data values and changing the view leave such an element alone.
///
/// The group paints in the order its children were appended, so a node drawn later is painted over an earlier element.
/// Draw the nodes first and the application's elements after them.
///
/// Two scenes on one `<svg>` have a group each. The class name is part of the supported DOM, and changing it is a
/// breaking change.
```

### Prepared changelog entry

Add this under the library's next version in `changelog.md`.

```markdown
## Documented

- `Scene` now documents the content group, `svg-dom-graph-content`, as a supported hook for drawing next to the scene
```

Applying both needs the version in `Cargo.toml` to move to 0.2.49.
That is a release decision, and it is not made here.
