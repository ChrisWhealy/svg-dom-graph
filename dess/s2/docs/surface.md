# The projection and the surface

The design asks S2 for a long-lived fixed `Scene`, an overlay, a reserved data node and an exact incremental projection.
This page says what each is, what a draw writes, and what the tests prove.

## The slots

The projection is a list of slots with keys that never change.
For a given number of servers the list is the same in every projection, and a slot that has nothing to say is blank and hidden.

| Key | Count | Text | Hidden when |
|---|---|---|---|
| `clock` | 1 | The tick and the snapshot number | Never |
| `queue` | 1 | Waiting jobs of capacity | Never |
| `S0` to `S3` | 2 to 4 | The running job's key, or `idle`, or `down` | Never |
| `Q0` to `Q11` | 12 | The job's key and `H` or `L` | The position is empty |
| `more` | 1 | `+N more` | No more than twelve wait |
| ten counters | 10 | The value | Never |
| `selected` | 1 | The selected job and its state, or `none` | Never |

Each slot carries four fields: the text, the class, the accessible name and whether it is hidden.
The class says the state, so a stylesheet can colour a server that is busy or down, and a job that is selected.
The accessible name says the same in words, for example `Server S0: running J0, attempt 1, due tick 4`.

The data node has eight cells: the snapshot number, the tick, the waiting count, the busy count, and the completed, timed-out, cancelled and rejected counts.
The first cell is the snapshot number, so that every displayed slice changes the node, including a slice that pops only stale events.

## What a draw writes

- **A delta.** The fields that differ from the last draw.
  A slice that pops only stale events writes four: the clock's text and name, and the stale counter's text and name.
- **A full redraw.** Every field of every slot, which is four for each of the 28 to 30 slots.
- **The data node.** `Scene::set_data_values` on every draw, a delta or a full one.
- **Nothing else.** A draw adds no element, and does not touch the view or the focus.

A draw that fails, or is given a projection of another shape, makes the surface forget what it believes is drawn.
The next draw is then a full one, and cannot be a delta from something untrue.
Whether a draw may be committed is the controller's decision, in stage S3.

## A host that has left the document

A detached `<svg>` still accepts DOM writes, so the absence of an error from a write says nothing about whether the page was written.
S0 showed this, and the surface owns the guarantee that follows from it.

- **Checked, and not discovered.** The surface asks whether the host is connected before every write, before the data node's setter, and after the setter.
- **A distinct error.** The draw fails with `DrawError::Detached`, so that the controller can tell it from a failed write and from a projection of the wrong shape.
- **Nothing is trusted.** After `Detached` the surface holds no belief about the page, and the next draw, once the host is back, is a full one.
- **No scene operation on a detached root.** If the host leaves in the middle of the overlay's writes, the setter is not begun.

The controller adds what the surface cannot know: whether the gallery panel is visible, and whether to commit a slice at all.
The surface refuses a detached host in every case, so the controller cannot forget to ask.

The case of a host that leaves between two writes needs a hook that no real build has.
The `fault-hook` feature adds `FaultPoint` and `Surface::set_fault_hook`, which call a function after each overlay write and after the setter.
Only the tests enable it, through a dev-dependency of the crate on itself, so a normal build contains neither.

## What the tests prove

| Test | Proves |
|---|---|
| `e0_projects_exactly_what_the_ledger_says` | After each of the 12 slices and before the first, every slot says what the ledger says. The ledger is derived from the table in section 4 of the design, and not from the engine |
| `e0_is_on_the_page_exactly_after_every_slice` | The same on a real page, read back from the elements, with the data node holding the snapshot number |
| `a_delta_between_any_two_snapshots_equals_a_full_redraw` | A delta between any two snapshots, forwards or backwards, reaches what a full redraw writes. This is a seek |
| `a_page_reached_by_deltas_equals_a_page_drawn_in_full` | The same on the page, against a fresh surface drawn in full at each step |
| `a_delta_writes_only_what_differs` | A stale-only slice writes four fields and calls the setter |
| `a_thousand_displayed_slices_add_no_element_and_each_calls_the_setter` | 1,000 consecutive commits through the one data node change its content every time, keep the page equal to its projection and add no element |
| `drawing_leaves_the_view_and_the_focus_alone` | A set view and a focused slot are where they were after every draw, including draws that change the selection |
| `the_overlay_is_in_the_one_content_group_and_follows_the_view` | There is one content group, the overlay is its last child, and a view of scale 2 scales the overlay as it scales a node |
| `a_refused_draw_makes_the_next_one_a_full_redraw` | A projection of the wrong shape is refused, and the next draw is a full one |
| `a_detached_host_is_refused_before_anything_is_written` | A draw and a full redraw on a detached host fail with `Detached` and write nothing. After reattaching, the next draw is a full one and restores the projection |
| `a_host_that_leaves_in_the_middle_of_a_draw_never_leaves_the_surface_trusted` | The host leaves after each of the 30 overlay writes of a draw in turn, and after the setter. Every time the draw fails, no write follows the departure, the setter is not begun on a detached root, the surface does not trust the page, and a full redraw restores it |
| `ids_and_slot_keys_are_unique_and_do_not_change` | Every id on the page is unique, and the slot keys are those of the projection before and after drawing |
| `nothing_is_drawn_outside_the_view_or_over_anything_else` | Nothing is outside the view box, and nothing overlaps, for E0 and for four servers with a full window and a "more" line |

The content group test is the compatibility test that the design asks for.
It rests on the six library tests that S0 added in `tests/drag/content_group_hook.rs`, and it checks the same facts on the simulator's own scene.

## What the tests found

The geometry test found two defects that no other test could have, since all the others read the elements' content and not their place.

- The heading of the counters was 12 units above the first counter, which is less than a line of text.
  They overlapped.
- The "+N more" text was placed at the top of its row, and not at its baseline, so it was printed over the last queue slot.

After the first fix the test found that the counters overlapped the data node, which is 86 units tall.
Both are fixed, and the second is one of the mutations that the sensitivity check makes.

## The sensitivity check

`dess/s2/mutation-check.mjs` breaks the code in fourteen ways, one at a time, and runs the tests.
The record is `dess/s2/evidence/mutation-check.json`, and all fourteen are caught.

| Mutation | First test to fail |
|---|---|
| A hidden slot is not marked hidden | E0 on the page |
| The data node is given zeros | E0 on the page |
| A delta writes a blank class | E0 on the page |
| A failed draw keeps trusting the page | The refused draw |
| The host is not checked between writes | The host that leaves mid-draw |
| The host is not checked before the setter | The host that leaves mid-draw |
| The host is not checked after the setter | The host that leaves mid-draw |
| The "more" line is drawn at the top of its row | The layout |
| The queue window is shifted by one | The empty slots |
| A delta never writes a changed label | The stale-only slice |
| The stale count is not shown | The E0 ledger |
| The retry-wait count is not lowered when a retry re-queues a job, or when a job is finished | The recount after every slice |
| A model copies every job that has arrived | The model's size |

## Left open

- **The setter after a failed write.** A failed `set_data_values` leaves some cells written, and asking for the old values does not heal them (finding F1 of S0).
  S2 does not meet a failure, so it does not meet this.
  Stage S4 does.
- **Keyboard and selection.** Each slot can take the focus from a script, and nothing moves it by key.
  The controller adds the keys in S3.
- **A stylesheet and a look at the page.** The classes are in place and the CSS is not.
- **The panel.** No gallery panel hosts the surface, so the hidden-panel predicate of S0 is not exercised.
- **Firefox.**
