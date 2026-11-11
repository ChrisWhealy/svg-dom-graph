# Gallery panel, P1: the session coordinator

The specification is `design-specs/dess_demo.pdf`, revision 4, section 7.
P1 is the reusable part of the cross-size scenario switch, in `service-sim` and with no browser in it.
It decides when a new scene replaces the one on the page, and when it does not.

The code is `service-sim/src/session.rs`, and its tests are in `service-sim/src/session/unit_tests.rs`.
Nothing in `demo-app` changes in this phase, and no simulator rule is added to the gallery.

## What P1 builds

| Piece | What it is |
|---|---|
| `Spec` | A scenario: its identity, its server count, how to make its engine and what it is known to come to |
| `Spec::preflight` | Runs the engine to its end, counting slices and pops, against exact totals and per-scenario ceilings |
| `SessionHost` | The one trait that touches the page: build a candidate, verify it, hand the page over, undo that, discard and finalize |
| `Coordinator` | The state machine: Idle, Staging, Publishing, then Committed or RolledBack |
| `Target` and `TargetHook` | The one place the current controller is kept, and the weak handle that a page's hooks hold |

A gallery binds `SessionHost` to its DOM in P3, and the tests bind it to a fake.

## The transaction

1. **Preflight.** A request that is not valid is rejected with nothing changed, not even the old playback.
2. **Stage.** The old playback and any pending seek are invalidated, since a request owns navigation.
   A candidate is built at its first slice, and the saved speed is applied while it is paused.
3. **Validate.** A controller that was made is not a controller that drew.
   The candidate has to be clean, at slice 0, on a live host, and its page has to pass the host's own readback.
4. **Publish.** The host hands the page over, and a failed handoff is undone.
   The candidate has to be live and clean again afterwards.
5. **Commit.** Only now does the target point at the new controller, and the old one is disposed and its page removed, once.

Any failure before the commit rolls back.
The candidate is disposed and removed, the old session is where it was, and its page is repaired if the handoff may have disturbed it.
The old cursor, selection and scenario are unchanged.

The old playback is not promised.
A request invalidates it at the start, and a cancelled timer is not restored.

A scenario with the same number of servers uses the controller's own in-place switch, which is a transaction already.

## The hooks

A mutation observer and the page's lifecycle listeners are installed once, and they have to reach whichever controller is current.
They hold a `TargetHook`, which holds the `Target` weakly.
After a replacement the same hook reaches the new controller, after a rollback it still reaches the old one, and after the coordinator is gone it reaches nothing.
Nothing owns anything in a circle, which a test shows by counting the hosts that are dropped.

## Changes to the controller

Three small additions, with no change to what the controller does.

| Addition | Why |
|---|---|
| `Factory` is public | A scenario's engine maker is part of a `Spec` |
| `Controller::speed_ms()` | The coordinator carries the speed to the new controller |
| `Controller::is_disposed()` | A test, and a caller, can tell that a replaced controller is gone |

## Tests

There are 20 tests, all native, with a fake host and a manual scheduler.

- **Preflight:** E0 gives 12 slices and 19 pops, E3-NOOP gives 9 and 9, and the scaled run gives 487 and 1,009.
  A bad server count, a passed ceiling, a total that differs and a failing engine are each refused.
- **Replacement:** the same-size path builds nothing.
  A different size replaces the session, keeps the speed, does not resume and clears the selection.
- **Failures before the handoff:** a host that cannot be staged, a first draw that fails and leaves the candidate dirty, and a failed readback each leave the old session untouched.
- **The handoff:** a failure at each of its three steps is undone and the old page is repaired.
  A handoff that cannot be undone is reported, and the old session stays active.
  A candidate that is not live after the handoff is rolled back.
- **Hooks:** one made before a replacement reaches the new controller, and one still reaches the old controller after a rollback.
  One tells the current controller that the panel was hidden and shown.
- **Ownership:** the old session is disposed and removed exactly once, no host is kept alive in a circle, and a timer and a frame that were on their way do nothing.

`node dess/demo-panel/p1/mutation-check.mjs` breaks the coordinator in 12 ways, one at a time, in a copy of the tree, and a test has to fail for each.
All 12 are caught.
Each mutation is built on its own first, so one that does not compile is never counted as caught.

## What P1 does not do

- **No DOM.** The real host, with its staged element, its handoff of pointer and focus and its readback, is P3.
- **No layout test.** P0 found that a staged host in the flow of the page pushes what follows it down.
  The P3 binding must stage the host out of the flow, and its browser test must show that the live scene and the scroll position do not move.
  A native test cannot show that.
- **No panel owner.** The object that holds the hooks, the controls and the coordinator is the gallery's, and is P3.
- **No catalogue.** The four scenarios and their labels are P2.
