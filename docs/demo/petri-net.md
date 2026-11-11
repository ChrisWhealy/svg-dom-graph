# Petri net

A bounded buffer that four producers fill and three consumers empty, as an ordinary place/transition Petri net.
The panel is in the group "Concurrency".
It adds no library API.
Everything is demo-local, in `demo-app/src/petri_net/`.

## The net

There are 20 places, 18 transitions and 50 arcs, and every arc has weight 1.
A transition is enabled when every input place holds a token.
Firing it takes one token from each input place and adds one to each output place, all at once.
A producer has the places `Idle`, `Busy` and `Ready` and the transitions `Start`, `Finish` and `Put`.
A consumer has the places `Idle` and `Processing` and the transitions `Take` and `Complete`.
The buffer is the two places `Free` and `Used`, which always add up to 8.

| Transition | Inputs | Outputs |
| --- | --- | --- |
| `Start[i]` | `P[i].Idle` | `P[i].Busy` |
| `Finish[i]` | `P[i].Busy` | `P[i].Ready` |
| `Put[i]` | `P[i].Ready`, `Free` | `P[i].Idle`, `Used` |
| `Take[j]` | `C[j].Idle`, `Used` | `C[j].Processing`, `Free` |
| `Complete[j]` | `C[j].Processing` | `C[j].Idle` |

A `Take` frees its slot at once, so `Processing` is not counted in `Used`.
Nothing in the page stops a full buffer from taking an item.
A `Put` is simply not enabled while `Free` is 0.
A producer that waits is `Ready` with `Free` at 0, and a consumer that waits is `Idle` with `Used` at 0.
These are worked out from a marking and are never kept.

The net checks every marking it makes.
A worker is in exactly one state, and `Free + Used = 8`.
The history also counts Puts, Takes and Completes, and checks that Puts minus Takes is what the buffer holds, and that Takes minus Completes is how many consumers are processing.
A search of the whole net finds exactly 5,832 reachable markings, which is 3⁴ × 2³ × 9.
The initial marking can be reached again from every one of them, and none is dead.
A test-build-only interpreter that forgets to take the `Free` token on a `Put` shows that these checks notice a fault.

## The scripts

A script is only a list of transitions.
Every snapshot comes from firing them through the net, so a script cannot write a marking or wait.
Snapshot 0 is the initial marking, and event k makes snapshot k.

| Script | Events | Snapshots | Shows |
| --- | --- | --- | --- |
| Walkthrough | 50 | 51 | The buffer fills, a producer is blocked, a `Take` releases it, the buffer drains and the consumers are blocked |
| Contention | 28 | 29 | Two `Put`s are enabled for the one free slot, and firing one disables the other |
| Interleaving | 30 | 31 | Workers that are busy, ready and processing at the same time, and a return to the initial marking |

The named checkpoints are indices of snapshots, not of events.
Walkthrough has Full 24, Blocked 26, Released 27 and Empty 45.
Contention has Both ready 25, Last slot taken 26 and Re-enabled 27.
Interleaving has Two busy 2, Mixed work 8, Two processing 11, Three processing 14 and Home again 30.

In Interleaving the buffer holds at most two items, at snapshots 21, 23, 24 and 25.
It tests overlapping work, and not a full or an empty buffer.
The design specification says the buffer never holds more than one, which the script does not bear out.

## Manual mode

Entering manual mode forks a branch from the snapshot that is shown.
The branch has its own index 0, and inherits the marking and the counters.
Selecting never fires.
A transition fires from the button, from a double click, or from Enter or Space while it has the focus.
All three use the same checks, and a transition that is not enabled says which input has no token and changes nothing.
A firing from an earlier snapshot of the branch cuts off the snapshots after it.
A branch holds at most 10,000 events, and the limit is checked before anything changes.
Return to script restores the script at the snapshot the branch was forked from.
Choosing a scenario discards the branch and says so, in the page and aloud.
Checkpoints and Play are not available in manual mode.

## How the page is built

There is one scene for all three scripts and for the manual branch.
Changing the scenario or the history draws another snapshot on that scene, and never builds another.
The places are circle nodes, the transitions are narrow box nodes, and every arc is a straight connector.
The arcs at a `Put` or a `Take` are fixed with `with_from_side`/`with_to_side` and a position along the side, and the arcs at a hub with a clock position.
A gate is a rectangle, so it is never given a clock position, which the library refuses.
The `Free` arc is always the upper one on a gate's side, as `Free` is the upper hub.
The library's own nodes and connectors are the picture and are hidden from assistive technology.
An overlay in the same transformed group holds everything that changes or can be selected.

A snapshot is projected to a list of slots and the fields of the text panel.
The page writes the whole projection, or only the slots that differ from the one before, and a test checks that the two give the same page for every pair of snapshots.
A write that fails marks the page dirty, and the next action draws the committed snapshot again in full.
Every tick and every seek carries the playback epoch and the history generation, so a callback that was asked for before a stop or a change of history does nothing.

## Tests

The native tests are in `unit_tests.rs` beside each module and cover the net, the scripts, the history, the layout, the projection and the controller.
The browser tests are in `cdp-integration-test/tests/cdp/petri_net/unit_tests.rs`.
They measure the real geometry, including the smallest clearance between a spoke and the hub it does not belong to, and drive the keyboard, the pointer, the playback, a failed write and the comparison of a diffed page with a full one.
