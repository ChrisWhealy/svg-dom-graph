# Observing a Drag

`Scene::make_draggable_with_observer(node, options, on_drag)` makes a node draggable exactly as `make_draggable_with` does, and also tells `on_drag` what the drag does.
It is for a host that keeps something else in step with a dragged node, such as a physics simulation.
It follows the library's own drag, which already gets the pointer, the zoom, the bounds and the collision push right.
It adds no second set of pointer listeners.

The drag itself is unchanged: the same listeners, the same pointer capture, and the same coalescing of moves to one a frame.

## The events

`on_drag` is called with the node and a `DragPhase`.
Every `origin` is the top-left corner of the node's rectangle in the scene's user space: the same unit that `move_node` takes and `node_rect` gives.
A host that wants the centre adds half of `node_rect(id).size`, and never assumes the size.

| Phase | When | `origin` |
|---|---|---|
| `Start` | The drag was established. | Where the node is. |
| `Applied` | A move was written to the page **without an error**. | Where the node now is. |
| `End` | The pointer was released, after the last pending move and any push clear of an overlapping node. | Where the node ended up, read from the scene. |
| `Cancel` | The drag was cancelled, for example by the system. | The last committed position. A position that was never drawn is discarded. |

A drag is a `Start`, any number of `Applied`, and then exactly one of `End` or `Cancel`.
Events for one node arrive in that order.
Events for two nodes can interleave, since two pointers can drag two nodes at once.

`DragPhase` is `#[non_exhaustive]`, so a `match` needs a wildcard arm.

### Applied is only for a move that succeeded

A move that returns an error sends nothing, and its error is dropped exactly as it is for an unobserved drag.
So a host never sees an `Applied` for a position the page does not have.

A move to the origin the node already has still succeeds, and still sends `Applied`.
It can retry an edge label that failed to place earlier.
A duplicate `Applied` is therefore possible and harmless.

After a failed write the host can find out where the node really is.
`End` and `Cancel` carry the scene's own origin, which is authoritative, and `node_rect` gives it at any time.

### A push clear is reported

When the collision policy pushes a dropped node back clear of another, that is one more move.
So it is one more `Applied`, and then `End` carries the corrected origin.

## The handle owns the observer

The method returns a `DragObserverHandle`.
It is the only strong owner of the callback.
The drag listeners and the coalescer hold only a weak reference to the slot the callback sits in.
So the callback can capture whatever it likes, a host controller included, without a cycle through the scene.

* **Keep the handle for as long as the events are wanted.**<br>
  The observer stops the moment the handle is dropped.
  A handle that is not bound to anything is dropped at once, which `#[must_use]` warns about.
* **Dropping the handle is final for that node.**<br>
  It ends observation of that node for good.
  It does not remove the drag listeners, and it does not make the node eligible for another call.
  So a later `make_draggable`, or a second `make_draggable_with_observer`, returns `Error::AlreadyDraggable`.
  No other observer can attach to a node that is already draggable.
* **A running notification finishes.**<br>
  A notification already running when the handle is dropped finishes.
  None is delivered after it.
  The callback may even drop its own handle.
* The callback's captures are released when the handle is dropped.

The handle is not `Clone`.

## Re-entrancy and timing

Events are delivered synchronously, from inside the pointer event handler or the animation frame that caused them.
They are delivered with **no borrow of the scene held**, so the callback may call any method of the scene, `node_rect` and `move_node` included.
It must not assume it runs at a frame boundary, and it should do little.
A host that has more to do queues the event and does it later.

## Failed registration

The method fails in the same ways as `make_draggable_with`: `UnknownNode`, `AlreadyDraggable`, `InvalidCollisionPadding`, `InvalidDragBounds`, and a wrapped `Svg` error if a listener cannot be registered.
Each is checked, or rolled back, before the call returns, so a failed call installs nothing, makes no node draggable and returns no handle.

## Commit order of `move_node`

The events rely on a small change to the order in which `move_node` commits a move.
It writes the node's transform first, and only then changes the node's position in the model.
So if the transform cannot be written, `node_rect` still says where the node really is, and no `Applied` is sent.
The move is committed once the transform has been written, and it is **not rolled back**.
If a connector or an edge label then fails to update, the call returns an error and the node stays moved.
`End` and `Cancel` report the new position.
Recovery is eventual rerouting and not rollback.
See `Scene::move_node`.

## Example

The caller must keep the handle.
A helper that installs the observer and returns without handing the handle back ends the notifications as it returns.
So this one returns the handle, together with the log it fills.

```rust,no_run
use std::{cell::RefCell, rc::Rc};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DragObserverHandle, DragOptions, DragPhase, Scene};

/// Returns the handle, and with it the log of where each drag ended.
/// The caller keeps the handle for as long as the events are wanted, and the notifications stop when it is dropped.
fn watch(scene: &Scene) -> Result<(DragObserverHandle, Rc<RefCell<Vec<Point>>>), svg_dom_graph::Error> {
    let node = scene.add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "A")?;
    let log = Rc::new(RefCell::new(Vec::new()));
    let sink = log.clone();
    let handle = scene.make_draggable_with_observer(node, DragOptions::default(), move |_id, phase| {
        if let DragPhase::End { origin } = phase {
            sink.borrow_mut().push(origin);
        }
    })?;
    Ok((handle, log))
}
```

The weighted springs demo is the first user: see [Force-directed Graphs (Weighted springs)](../demo/springs.md).
