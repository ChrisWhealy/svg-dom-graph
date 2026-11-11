# Gallery panel, P4: scale and release

The specification is `design-specs/dess_demo.pdf`, revision 4, sections 12 to 15.
P4 adds the long paths and a way to take the panel down and show that nothing is left.
It also adds the cost in the demo's wasm, the browser observations and the user-facing page.

## Taking the panel down

The gallery builds a panel once and only hides it afterwards, so it never drops one.
`dispose_panel(id)` is for a page that unloads a panel for good, and for the tests that show that nothing is left behind.
It is a wasm export of the demo, it does nothing for a panel that has no owner, and it can be called twice.

For the simulator it does the following, in this order.

1. It removes the listeners of the controls, the watch on the gallery's stage and the page's lifecycle listeners.
2. It disposes the controller, which removes the scene and puts the controls back where the markup wrote them.
3. It clears what the build wrote into the markup, removes the source frames and marks the panel as not built.

A later `init_panel` then builds the panel again, as if for the first time.

## G18 to G20

| Test | Runs | What it shows |
| --- | --- | --- |
| G18 | nightly | The scaled scenario. A walk of all 487 snapshots by Next, 30 seeks in a fixed order, and six rates with four ticks at each. The page matches the independent engine and its own full redraw, and no element is added by 487 draws |
| G19 | nightly | The advanced scenario. A thousand commits by Play, one timer at a time, and 14 seeks across the 5,464 snapshots including the first and the last. Nothing in it is a timing, and it is not a benchmark |
| G20 | every commit with 3 cycles, nightly with 40 | Cycles of switching through every size, hiding and showing, a seek and a timer left waiting, and then taking the panel down and building it again. The elements, listeners, observers, held and lost externref slots, table capacity and waiting callbacks are exactly what they were after the first cycle |

The random seeks use a seeded generator, so a run is the same every time and a failure can be repeated.
G18 and G19 run with `--nightly`, and so does the long G20, which `--cycles N` can set.

G20 reads the table of externref slots that wasm-bindgen keeps, from the wasm instance, which the test page captures when it is made.
Nothing in the gallery is changed for that.
The length of the table is its capacity, which stays the same while a slot is free, so it cannot show a leak by itself.
So G20 counts the slots that hold an object, and the slots that hold `undefined`, which a failed import leaves for good.
A free slot holds `null`, and the first four hold constants.
After the first cycle the table held 532 objects in 2,052 slots, with 2 lost, and it was exactly that after each of 40 further cycles.

Callbacks that were kept and called after the panel was taken down do not run.
The page's scheduler frees a closure that was taken back, so a browser that still held one gets a refusal and not a call.
The test shows that calling them changes nothing in the page.

`dess/demo-panel/p4/mutation-check.mjs` breaks the taking down in six ways, leaks one object in each, and swaps the stale and no-op counters, and a test has to fail for each.
All eight are caught.
The leak is the case that matters: the table's capacity stays at 2,052, and only the count of held slots shows it.
It runs the tests with `--nightly --cycles 3`.

## The nightly job

`dess-gallery-nightly` in `.github/workflows/perf-nightly.yml` builds the demo and runs the whole script with `--nightly`.
It is apart from W6 Layer C and its timings, and it keeps its results for 30 days.
The per-commit `dess-gallery` job keeps the captured feature tree (`features.txt`) beside its results, as the design asks.

## The cost in the wasm

`dess/demo-panel/p4/size-ledger.mjs` builds the demo twice by the same command, with the same `Cargo.lock`, once at the commit before the panel and once as the tree is now.
It records the wasm in bytes and gzipped, the difference in bytes and per cent, the dependency tree of both and what the panel added to it.
No threshold is imposed.
The record is `evidence/size-ledger.json`, and the two trees are kept beside it.

| | Wasm | Gzip |
| --- | --- | --- |
| Before the panel | 9,183,623 bytes | 2,414,830 bytes |
| With the panel | 9,915,071 bytes | 2,611,905 bytes |
| Difference | 731,448 bytes, 7.96% | 197,075 bytes, 8.16% |

The panel added two crates to the demo's wasm dependency tree, `service-sim` and `service-sim-engine`, and nothing else.
`sha2` was already in the demo and is there once, at version 0.11.0.
The build is the demo server's own, which does not run `wasm-opt`.
The record was made with changes that were not committed, so it should be made again from a clean tree.

## Browser observations

See `browser-observations.md`.
Chrome is the gate.
Firefox 159 passes the `service-sim` browser tests, after a caption that was too wide in its type was fixed, and a short visit looked right.
Safari and a screen reader have not been tried.

## Other things that P4 changed

- The preflight time of a scenario is written to the console and is never a reason to refuse a run (design D2).
- The captions under the data node carry their size on the element, so they are the same width in every browser.
- `docs/demo/service-simulator.md` is the user-facing page, and it is linked from `docs/demo/README.md`.
- `dess/gather-evidence.mjs` has steps for the long paths, the Firefox observations, the P4 mutation check and the size ledger.

## What is not done

- **Safari.** Not observed.
- **A screen reader.** The range's `aria-valuetext` is checked at the attribute, since Chrome's accessibility tree does not show it.
- **The nightly job has not run on GitHub.** It runs the same script that was run here, but not on the runner.
