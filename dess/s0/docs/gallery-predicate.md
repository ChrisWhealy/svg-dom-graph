# How a hidden panel is known

The design needs two things from the gallery.
It needs a way to be told that a panel was hidden, and a synchronous test of whether the panel is hidden now.
S0 read the gallery's markup and measured the candidates in Chrome.

## How the gallery hides a panel

The gallery keeps every panel in one document, as a `<div class="section">` inside `#stage`.
The rules in `demo/style.css` are `.section { display: none }` and `.section.active { display: block }`.
`selectDemo` in `demo/index.template.html` toggles the class in the handler of a click.
Nothing else is told, no event is dispatched, and a panel built once is never rebuilt.
Seven demos stop their clocks on `pagehide`, and the springs demo also uses `visibilitychange`.
Neither says anything about a panel being hidden while the page stays visible.

## What was measured

`dess/s0/spikes/predicate-bench.mjs` builds a page with the same markup and the same two rules, with an SVG of 60 rects in each panel.
The results are in `dess/s0/evidence/predicate-bench.json`.
They are for Chrome 157 on one machine, with a timer that resolves 5 µs.

### Every candidate is right

All seven candidates gave the right answer for a shown panel and for a hidden one.
`isConnected` is true for a hidden panel, as it should be, since a hidden panel is still in the document.
So it cannot be the whole test.

### What a call costs

| Predicate | Clean, µs | After an SVG write, µs |
|---|---|---|
| A cached flag the hint sets | 0.002 | none |
| `isConnected` | 0.031 | 0.003 |
| The ancestor has class `active` | 0.066 | 0.051 |
| `closest('.section:not(.active)')` is null | 0.125 | 0.115 |
| `checkVisibility()` | 0.078 | 0.684 |
| `getClientRects().length > 0` | 0.208 | 2.011 |
| `getBoundingClientRect().width > 0` | 0.189 | 1.994 |

A call after a write has to recalculate style, or layout, first.
`checkVisibility()` needs only style, and costs about a third as much as the layout-reading calls.
The class tests are cheap, but they name the gallery's own class, so they would break when the gallery changes.

### The order of events

After a class is toggled in one task, a MutationObserver runs before a timer that was already queued.
An IntersectionObserver runs after that timer and after the next animation frame.
So a mutation hint reaches the controller before any timer callback can run.
An intersection hint does not, and one tick could commit after the hide.

## Decision

| Need | Choice |
|---|---|
| The hint that a panel was hidden | A MutationObserver on `#stage`, observing the `class` attribute with `subtree: true` |
| The test at callback entry, before scheduling, and before a commit | `checkVisibility()`, outside every timed interval |
| The test inside a timed draw | The cached flag the hint sets, and `isConnected` |

The mutation hint is delivered as a microtask at the end of the task that hid the panel.
So in the gallery a callback that was already queued finds the flag already cleared.
The test at callback entry still runs, since the hint cannot see a hide that is not a class change on the observed subtree.

The design says that one callback may run after a hide and must be inert.
That remains true as a rule, and for this gallery the mutation hint makes it hold without relying on it.

## Limits

- Chrome 157 only.
  Firefox and Safari were not run, and `checkVisibility()` needs Chrome 105, Firefox 106 and Safari 17.4.
- The page copies the gallery's markup and rules, and is not the gallery.
  Stage S3 repeats the order test against the real gallery in a CDP test.
