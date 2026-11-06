# Phase 0 instruments: timer, counters and listener ownership

## 5. Timer resolution

| Page                      | `performance.now()` step |
| ------------------------- | ------------------------ |
| Not cross-origin isolated | 100 µs                   |
| Served with COOP and COEP | 5 µs                     |

The harness serves its own pages, so it can send those headers.
The resolution is recorded in every result, and an operation shorter than 50 steps is timed in a batch.

## 6. Chrome's counters

`Performance.getMetrics` gives `Nodes`, `JSEventListeners`, `JSHeapUsedSize`, `LayoutCount`, `RecalcStyleCount` and `ScriptDuration`.

`JSEventListeners` is not a count of live registrations until a collection has run.
After a second scene was built it read 139, and 37 after `HeapProfiler.collectGarbage`, which is what the page's own tally (`addEventListener` calls minus `removeEventListener` calls) also gave.
The first reading, before any collection, was 51 against a tally of 37.
So Chrome's count is a diagnostic that needs a forced collection, and a raw call count is only a cross-check.

## 7. Who owns the listeners

The unexplained result is explained, using the effective-registration tracker (`page/listener-tracker.js`).
The fixture keeps its only `Scene` handle in a `thread_local`, and `run()` replaces it (`cdp-test-fixture/src/lib.rs`, the line `*slot = Some(scene)`).
So the second build dropped the first `Scene`.

| What the tracker saw | First build | Second build |
|---|---|---|
| Listeners added | 44 | 44 |
| Listeners removed | 7 | 44 |
| of which, registered by the same build | 7 | 7 |
| of which, registered by the first build | 0 | 37 |
| Registrations alive afterwards | 37 | 37 |

- **Scene-owned listeners are released when the last `Scene` handle drops**, even though the DOM it drew stays where it is.
  The 37 are pointer, focus, keyboard, click and wheel listeners on the scene's `<g>` and `<rect>` elements.
  The element count doubled because the first scene's elements were left behind, with no listeners.
- **Seven registrations are transient.** Each build puts 7 listeners (`pointerdown`, `pointermove`, `pointerup`, `pointercancel`, `focus`, `blur`, `keydown`) on one `<rect>` and removes them again a few calls later.
  I did not establish why.
  They are self-cancelling and bounded, so they do not affect a count taken after a build.

What this means for a listener-count gate:

- The count is only meaningful once the last `Scene` handle has dropped, not when the host `<svg>` is removed.
- The expected count after a drop is the count before the scene was built, which is a checkable gate.
- A registration that survives a drop is a candidate leak.
  One on a node that is no longer in the document shows in the tracker's `detached` figure.
- The tally is the cross-browser check.
  Chrome's count is its cross-check, and only after a forced collection.

The tracker is tested in `page/test-listener-tracker.mjs`, with 25 checks in a real Chrome.
It covers duplicates, capture flags, `once`, `AbortSignal`, `handleEvent` objects, null callbacks and detached nodes.
It then compares the tally with Chrome's count after a collection, for 50 listeners added twice, and for `once` listeners that stop propagation and listeners on an aborted signal.

An earlier version missed a `once` listener that calls `stopImmediatePropagation` on itself, because it relied on a helper listener that never ran.
That is removed.
A `once` listener now goes to the browser through a thin wrapper.
The wrapper records its own firing whenever the original would run.
A signal is checked when the tally is read, so nothing depends on a helper firing.
The only trace left on a listener is that DevTools shows the wrapper in place of the original.
The library, `demo-app`, `demo-common` and the fixture use neither `once` nor `stopImmediatePropagation`, so the case could not have arisen from them.
A listener-count gate no longer assumes that.

## Running the spike

```sh
# From the repository root, with cdp-test-fixture/pkg built.
node perf/spike/phase0-spike.mjs cdp-test-fixture
node perf/spike/phase0-spike.mjs cdp-test-fixture isolated
```
