# Phase 3: the gallery adapter

`gallery-sweep.mjs` selects every panel with a real click on its menu entry, so it works for every panel without a change to any of them.
The report (`schemas/gallery-report.schema.json`) states its own completion.
In the recorded sweep (`evidence/phase3/results/gallery/`):

| | |
|---|---|
| Panels in the gallery, and swept | 54 of 54 |
| Loaded, failed to load, with page errors | 54, 0, 0 |
| Walked with Next, Previous or Restart | 16 |
| Downgraded to a load and a revisit | 37 (no such control by accessible name) |
| Skipped by an override | 1 (Tarjan, whose panel only opens the demo in its own window) |
| First visit and revisit timed | 54 and 54 |
| Walk verdicts: pass, inconclusive, fail | 16, 38, 0 |
| Checks of those walks: pass, inconclusive, fail | 57, 77, 0 |

A first visit has a median of 29 ms, and a revisit 38 ms.
They are close because the time is from the click to the end of the frame that shows the panel, and showing a large panel costs more than building it.
The sweep says plainly that none of this is a teardown, since the gallery builds each panel once and only hides it afterwards.

## The verdict of a walk

A walk is not a diagnostic with a green label.
Each walked panel gets a verdict, and each verdict is made of checks, and a check comes out one of three ways.
An observation that nothing defines is written down and is never counted as success.

- **pass:** the control exists, its expected effect was seen, and the invariants that are defined for the panel hold.
- **inconclusive:** a control is absent, or the panel's own navigation has no defined expectation to hold it to, or it declares that it has none.
  The panel is not asked to behave like another.
- **fail:** a defined expectation was broken.
  The sweep exits 1.

What is defined for every panel is small.
An enabled Next or Previous changes the panel.
An enabled Restart changes a panel that has been moved off its start.
The page throws nothing during the walk.
Two things differ from demo to demo and are held only where a panel declares them in `gallery-overrides.json`, with the place in its source that says so:

| Declared | Held to | Declared by |
|---|---|---|
| `reversible` | From each step, Previous lands on the state that Next left from | The four selection panels (`selection.rs`) and Theta (`xor_loop.rs`) |
| `restart_to_start` | Restart returns the panel to the state it was first shown in | Dijkstra declares `false`, since its Restart draws new random weights |

Writing it corrected two expectations of mine, which the first version of the check would have counted as failures:

- Previous from the first step reaches the position before it, which a demo need not treat as its unstarted state.
  A walk of Next then Previous the same number of times does not have to end where it began.
  So the invariant is stepwise, and the last Previous is not judged.
  The five declared panels retrace every step they can be compared at.
- A fingerprint of a panel's attributes drifts in the fifth decimal place of positions that come from measured text widths.
  That made Shell sort's Restart look as if it did nothing, and Theta-C's walk look as if it had not come back.
  Every number in the fingerprint is now rounded to a tenth of a pixel, and a mismatch is reported with the first few elements that differ.

The other 11 walked panels have nothing declared, so their invariants are inconclusive, with what was seen written beside it.
`selftest-gallery.mjs` shows that a verdict can come out as a failure.
Dijkstra, declared to restart to its start against its source, fails and names the check.
Declared as its source says, it is inconclusive for that check.
Selection-1D with nothing declared passes on its Next and Previous and says that the retrace it saw is an observation and not a pass.

Of the 174 clicks of the 16 walked panels, 24 could not be placed in a frame (`STATE_DROPPED`), which is most of the steps of the heaviest panels.
Every click changed the panel in all 16 walks.
