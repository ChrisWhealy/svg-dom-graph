# Gallery panel, P1: the session coordinator

The specification is `design-specs/dess_demo.pdf`, revision 4, section 7.
P1 is the reusable part of the cross-size scenario switch, in `service-sim` and with no browser in it.
It decides when a new scene replaces the one on the page, and when it does not.

The code is `service-sim/src/session/mod.rs`, and its tests are in `service-sim/src/session/unit_tests.rs`.
Nothing in `demo-app` changes in this phase, and no simulator rule is added to the gallery.

## What P1 builds

| Piece | What it is |
|---|---|
| `Spec` | A scenario: its identity, its server count, how to make its engine and what it is known to come to |
| `Spec::preflight` | Runs the engine to its end, counting slices and pops, against exact totals and per-scenario ceilings |
| `SessionHost` | The one trait that touches the page: build a candidate, verify it, hand the page over, read the handoff back, undo it, discard and finalize |
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
   A handoff that reports success is then read back from the page, by `SessionHost::verify_published`, and the candidate has to be live and clean again afterwards.
5. **Commit.** Only now does the target point at the new controller, and the old one is disposed and its page removed, once.

Any failure before the commit rolls back.
The candidate is disposed and removed, the old session is where it was, and its page is repaired if the handoff may have disturbed it.
The old cursor, selection and scenario are unchanged.

The coordinator's state is the result of the latest request that was acted on, whether it was a replacement or a switch in place.
It is `Committed` if the scenario is now the one on the page, and `RolledBack` if the one that was there still is.
A request that is rejected before anything was done changes nothing, and so leaves the state of the request before it.

The old playback is not promised.
A request invalidates it at the start, and a cancelled timer is not restored.

A scenario with the same number of servers uses the controller's own in-place switch, which is a transaction already.

### Why the handoff is read back

The controller cannot tell a staged host from a shown one.
P0 showed that an `opacity:0` host that is `inert` is as `Live` to it as a visible host is.
So a `publish` that returned `Ok` without removing the staging, showing the page or moving the focus would pass every check that the coordinator can make itself.
Only the page knows, so `verify_published` has to read back the complete postconditions.

- The new page is shown and no longer staged.
- It is the only scene exposed to the keyboard, the pointer and the accessibility tree.
- The old page is hidden.
- The focus is on a valid target in the new page.

The method is required, with no default, so that an adapter cannot omit it.
A failure undoes the handoff and rolls back, as a failed `publish` does.

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

There are 22 tests, all native, with a fake host and a manual scheduler.

- **Preflight:** E0 gives 12 slices and 19 pops, E3-NOOP gives 9 and 9, and the scaled run gives 487 and 1,009.
  A bad server count, a passed ceiling, a total that differs and a failing engine are each refused.
- **Replacement:** the same-size path builds nothing.
  A different size replaces the session, keeps the speed, does not resume and clears the selection.
- **Failures before the handoff:** a host that cannot be staged, a first draw that fails and leaves the candidate dirty, and a failed readback each leave the old session untouched.
- **The handoff:** a failure at each of its three steps is undone and the old page is repaired.
  A handoff that cannot be undone is reported, and the old session stays active.
  A candidate that is not live after the handoff is rolled back.
  A handoff that returns success and leaves the page not shown, the old page still shown, or the focus not moved is not committed.
  The candidate is clean, at slice 0 and live to the controller throughout, so only the readback sees it.
- **Hooks:** one made before a replacement reaches the new controller, and one still reaches the old controller after a rollback.
  One tells the current controller that the panel was hidden and shown.
- **Ownership:** the old session is disposed and removed exactly once, no host is kept alive in a circle, and a timer and a frame that were on their way do nothing.

`node dess/demo-panel/p1/mutation-check.mjs` breaks the coordinator in 14 ways, one at a time, in a copy of the tree, and a test has to fail for each.
All 14 are caught, and the result is kept in `evidence/mutation-check.json` with the commit that it measured.
Each mutation is built on its own first, so one that does not compile is never counted as caught.
The script exits with 2 if the coordinator's source file is not where it expects, so a move of the file cannot make it pass vacuously.
CI runs it as one of the entries of the `perf-selftests` job.

## What P1 does not do

- **No DOM.** The real host, with its staged element, its handoff of pointer and focus and its readback, is P3.
- **No layout test.** P0 found that a staged host in the flow of the page pushes what follows it down.
  The P3 binding must stage the host out of the flow, and its browser test must show that the live scene and the scroll position do not move.
  A native test cannot show that.
- **No panel owner.** The object that holds the hooks, the controls and the coordinator is the gallery's, and is P3.
- **No catalogue.** The four scenarios and their labels are P2.
