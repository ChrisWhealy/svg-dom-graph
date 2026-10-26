# Nested Scenes

A container node is a box that owns a whole nested `Scene`.
So a step that is genuinely complex gets a full-size view of its own.

- `add_container_node`/`add_container_node_with` draw a box that owns a whole nested `Scene`.
  `enter` and `exit` move between a scene and its nested child.
  They hide one `<svg>` root and show the other.
  So a step that is genuinely complex gets a full-size view of its own.
  `make_enterable` makes the container clickable, and `make_unenterable` undoes it.
  `parent`, `is_nested` and `is_focused` report where a scene sits in the tree, and `replace_container_child` grafts a new child into an existing container.

   ***IMPORTANT***<br>
   The root `<svg>` of a nested scene is hidden through its `visibility` attribute.
   A CSS `visibility` rule that reaches it always wins over an attribute, and silently defeats `enter` and `exit`.
