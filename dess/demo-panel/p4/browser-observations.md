# Browser observations

Chrome is the release gate for the gallery panel, and the other browsers are observations.
Nothing on this page is a pass claim, and no CI job depends on it (design D5).
What was not seen is said so.

## Matrix

| Browser | Version | What was observed | Where |
| --- | --- | --- | --- |
| Chrome | 157.0.8103.39 | Everything: the whole test script, in CI and on this machine | `dess/demo-panel/test-gallery.mjs` |
| Firefox | 159.0 | The surface and controller tests, and a short visit to the panel | `evidence/firefox-observations.json` |
| Safari | not observed | Nothing | see below |

## Firefox 159.0, on macOS

- **The `service-sim` browser tests.** `wasm-pack test --headless --firefox service-sim` ran all 25 tests, and they pass.
  The first run found a real difference.
  The captions under the data node are wider in Firefox's default type than in Chrome's, and one ran past the edge of the scene.
  The captions now carry their size on the element, and not only in the page's style, so they are the same width in both.
- **A short visit.** `dess/demo-panel/p4/firefox-observe.mjs` drives a headless Firefox through geckodriver, and the page did the following.
  - The panel is built when it is selected, with the controls in a `foreignObject` of the scene's `<svg>`.
  - Four clicks of Next gave snapshot 4 of the walkthrough, and the data node held 4, 4, 3, 1, 0, 0, 0 and 1.
  - The server boxes were the colour of their state: red for the server that is down, and green for the one that runs a job.
    So `:has()` works in this version.
  - Play for 2.6 seconds at 1 Hz reached snapshot 6, and Pause stopped it with the button back at Play.
  - Switches to the no-op scenario, the five-server scenario and back each left one scene, the right number of servers and no refusal.
  - The controls, the scene and the inspector were laid out together, as in Chrome.
    A screenshot is kept beside the record.
- **Not looked at in Firefox.** The keyboard path, a failed switch, the hide and show of the panel, the long scenarios, and a screen reader.
  The visit is not a test suite, and it checks no snapshot against the independent engine.

## Safari

Not observed.
To do it, open the staged gallery (`cargo demo`, or `cargo run -p demo-server -- --build-only` and serve `target/demo-stage`) at `#panel-service-simulator`, and note the following.

| Look at | Why |
| --- | --- |
| The server boxes' colours | They need CSS `:has()`, which Safari has had since 15.4 |
| Controls inside the scene | They are HTML in a `foreignObject`, which Safari has handled differently in the past |
| The range and the scenario list from the keyboard | Native controls in a `foreignObject` |
| A hide and a show of the panel while Play runs | The page lifecycle events differ |

## Not seen in any browser

- **A screen reader.** Chrome's accessibility tree reports the Snapshot range as its number and ignores `aria-valuetext`.
  So a reading in VoiceOver or NVDA is still needed to know whether "Snapshot N of M" is announced.
- **A phone.** The layout was tested at 640 px and looked at in screenshots at 900 and 1500 px.
