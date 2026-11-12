# Red–black tree

A red-black tree built by six insertions, and then the deletion of a node that has two children, one event at a time.
The panel is in the group "Graph Algorithms", after Edmonds–Karp.
It adds no library API.
Everything is demo-local, in `demo-app/src/red_black_tree/`.

## The run

The run inserts 10, 5, 1, 15, 20 and 12, and then deletes 15.
Inserting 1 right-rotates at 10.
Inserting 15 recolours and moves the conflict up, and inserting 20 left-rotates at 10.
Deleting 15, which has the children 10 and 20, chooses 20 as the successor.
The original node 20 is moved into the place of 15 and takes its red colour.
Node 15 is removed, and its key is never copied.
The black that 20 supplied is missing from the right of 20, which the deletion fix-up then repairs with a left rotation at 10 and a right rotation at 20.
The final tree has 5 as the root, with 1 and 12 below it, and 10 and 20 below 12.

## The algorithm

`solver/` is plain Rust and knows nothing of the browser.
It is the classic bottom-up tree, with `Option` children.
An absent child is a black NIL leaf that is never allocated, so the deletion fix-up carries its own `FixupCursor`: `x`, `x_parent` and `x_side`.
An absent `x` has no parent pointer, and `None` cannot say which side it is the absence of when both children are absent.
The identity of a node is its key.
A transplant moves the successor node and never copies a key.

`Trace::build` turns a list of operations into a list of snapshots, each the whole tree after one event.
The events are an insertion, a recolouring, a left or right rotation, and the choice of a successor.
They are also a transplant, a splice, a case of the fix-up, and the end of an insertion or a deletion.
A snapshot says whether the tree satisfies every rule there, which a frame in the middle of a repair does not.
It also carries `replaced_by`, which says which key took the place of each deleted key so far.

Native tests check the rules at every finished operation and the links at every frame.
They check the golden run by the meaning of each event and not by its number.
They also check the cases that the run does not reach: a successor deeper than the right child, both sides of a missing child, the root, and an empty tree.
They also compare 300 random runs of inserts and deletes with a set.

## The picture and the tree

The stage is one `<svg>`.
The controls are in it, above the tree, and below them is a group that each step is swapped into.
The controls are never part of the swap.
A board is two layers.
The lower is a real `Scene`, whose circles and straight connectors are the picture, drawn at fixed places.
A key's column is its rank among the six keys of the run, and its row is its depth.
So a key never moves sideways because another is gone, and the column of 15 stays empty after the deletion.
Above it is a transparent semantic tree, a `role="tree"` of one `treeitem` for each key, nested to match the tree, with a hit circle on each circle of the picture.
The picture is `aria-hidden`, so the tree is the only thing that a screen reader or the keyboard sees.
A treeitem's ring is drawn on its own circle, since the browser's outline would box the whole subtree.

A board is built in a hidden `<svg>` that is in the page, so that the label of every node can be measured.
It is swapped in with one call to `replaceChildren` only when all of it has been built.
Pan and zoom are off, since either would move the picture away from the tree and add a tab stop of its own.
Nothing can be dragged, for the same reason.

## Choice, tab stop and focus

A person's choice of a key is kept as an intent (`chosen`), and the tab stop is another (`roving`).
Each snapshot resolves them again.
A key that is in the tree is itself, and a key that a deletion replaced is the key that took its place.
Any other key gives the root and then the smallest key.
So choosing 15 and playing past its deletion shows 20 as selected and says why, and going back shows 15 again.
Choosing 20 explicitly after the deletion stops that.

The focus is captured by key before a swap and put back on the item that the tab stop resolves to after it, and the swap never touches the buttons.
A focus event that the page caused itself is not taken for a person's.
Tab landing on the tab stop does not change it, but a pointer or an arrow key that lands somewhere else does.

## Keyboard

Tab enters the tree once, at the tab stop, which starts at the root, and Tab again leaves it.
Up and Down move in sorted key order, Home and End go to the first and the last key, and Left goes to the parent.
Right goes to the left child, and if there is none, to the right child.
So the right child of a node with two children is reached with Down.
Enter and Space select the focused key.
Moving never selects, and never changes the step.

## Controls and playback

Restart, Previous, Next, Play and a range go through the 30 snapshots.
Every button is drawn in the stage by the library's `svg_buttons`, as a group with the role of a button and a place in the tab order.
Enter and Space press it.
The range is the browser's own `<input type="range">` in a `<foreignObject>`, from the library's `sliders`, and its arrow keys move it one step.
Three jump buttons go to the state just after the right rotation at 10, and just after the left rotation at 10 while building.
The third goes to just after the choice of 20 as the successor of 15.
Each is found by the meaning of its event and its phase.
A button at an end is dimmed with `aria-disabled` and keeps its place in the tab order, and a press of it says so.
Play steps twice a second.
While it plays, only a finished insertion or deletion, a rotation, a transplant or a case of the fix-up is announced, and pausing says where the run is.
A step in the middle of a repair says "Repair in progress, not yet a valid red-black tree".

The controller is the one that the Edmonds–Karp demo uses, with a `Surface` and a `Scheduler`.
A commit that fails before the page is touched keeps the step and leaves the old tree.
One that fails after, or a focus that cannot be put back, marks the page dirty.
The next action commits it again before anything else, even one that has nowhere to go.
Playback has an epoch, so a tick from before a stop does nothing.

## Tests

`?manual-clock` in the address gives a clock that runs a tick only when `window.__rbtAdvance(n)` is called, and `window.__rbtPending()` says how many wait.
The hooks come off the window when the page is left.
A browser test makes a write fail by overriding `Element.prototype.setAttribute` for a name, and makes the swap throw by overriding `Element.prototype.replaceChildren`, before it changes the stage or after.

The browser tests read the real paths.
For each connector they take the start and the end with `getPointAtLength`, in screen space, and compare them with where the line between the two centres leaves each circle.
The error is never more than 1.5 units.
