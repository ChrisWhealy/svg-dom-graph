# Dragging

Dragging is the only way this crate offers to move a node by pointer.
To place a node from code, see `move_node` in [Nodes](nodes.md).

- `make_draggable`/`make_draggable_with` wire up pointer-based dragging for a node, with configurable drop-collision handling and an optional drag-bounding rectangle — see `DragOptions`, `CollisionPolicy`, and `DragOptions::bounds`.

   Pointer dragging is the only way this crate offers to move a *node*; a keyboard or non-drag alternative for that is not provided.
   The view is different: the toolbar's buttons, and the keyboard handling of `set_pan_mode` and `set_wheel_zoom_mode` (see [Pan and Wheel Zoom](pan-and-wheel-zoom.md)), let a keyboard user zoom and pan it.

Dragging a node keeps working under zoom, because it converts pointer positions through the node's own screen matrix.
