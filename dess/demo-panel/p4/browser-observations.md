# Browser observations

Chrome is the release gate for the gallery panel, and the other browsers are observations.
Nothing on this page is a pass claim, and no CI job depends on it (design D5).
What was not seen is said so.

## Matrix

| Browser                                    | Version            | What was observed                                                                                | Where                                                                     |
| ------------------------------------------ | ------------------ | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------- |
| Chrome                                     | 157.0.8103.39      | Everything: the whole test script, in CI and on this machine                                     | `dess/demo-panel/test-gallery.mjs`                                        |
| Firefox                                    | 159.0              | The surface and controller tests, a short visit, and the longer visit of `webdriver-observe.mjs` | `evidence/firefox-observations.json`, `evidence/firefox-interaction.json` |
| Safari                                     | 27.2 on macOS 27.2 | The longer visit                                                                                 | `evidence/safari-interaction.json`                                        |
| Firefox and Safari, a window of 390 pixels | as above           | The same visit, narrow                                                                           | `evidence/*-interaction-narrow.json`                                      |
| A screen reader, a phone                   | not observed       | Nothing                                                                                          | [the manual checklist](manual-checklist.md)                               |

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

## The longer visit, in Firefox 159 and Safari 27.2

`dess/demo-panel/p4/webdriver-observe.mjs --browser firefox|safari [--viewport WxH]` drives the browser's own WebDriver and records what it saw.
Safari needs "Allow Remote Automation" in its Develop menu, and opens a visible window.
It is a look and not a test suite, and no CI job runs it.
The two browsers gave the same results for everything below unless a difference is named.

- **Controls in the `foreignObject`.** All seven are inside it, and Safari draws and handles them.
- **Colours of the server boxes.** The `:has()` rules give the colour of each server's state in both, so Safari 27.2 has them.
- **The computed role and name of each control.** Button Previous, Next, Play, Restart and Repair display, slider Snapshot.
  The two lists are a combobox in Firefox and a button in Safari, which is how each reports a closed list.
- **The pointer.** Clicks on Next and Previous, and a real move, press and release through the driver's actions, advanced and went back as they should.
  A click of Previous takes about 60 ms in Safari, since a step back is a seek, so a reading straight after it sees the old snapshot.
- **The keyboard.** Enter and Space on Next each took a step, and the arrow keys on the range took it to the number asked for.
  A closed list does not change for the Down key in either.
  It does for the first letter of another option, which also switched the scenario and kept the focus on the list.
- **Tab.** In Firefox it went from Next to Restart, Play, Speed, Repair display and the range.
  In Safari it went to the speed list and then out of the scene, since Safari does not Tab to buttons unless the setting for it is on.
  That is the browser's habit and not the panel's.
- **Playback interrupted.** A press of Next while Play ran stopped it, took one step, and nothing ticked afterwards.
- **A hidden panel.** Routing to another panel while Play ran stopped it, nothing ticked while the panel was away, and it did not resume on return.
- **Focus.** After routing away and back the focus is on the body in both, since the panel was hidden.
  The focus was not put back on the control it had left.
  Whether it should be is a decision for the design, and Chrome was not compared.
  After a switch from the list it stays on the list.
- **Switching.** The five-server scenario drew five servers, and the walkthrough drew two, each in one scene and with no refusal.
- **Failures.** An injected failure of the first write of a step gave "The simulator could not draw" and left the picture at its snapshot.
  The Repair button then made the display up to date.
  An injected failure during a switch gave "The scenario could not be changed" with the previous scenario still shown and the list back on it, and a second try worked.
  In Safari the focus was on the body after a failed step, since a click does not focus a button there.
- **A narrow window.** The gallery's menu takes 272 pixels, so the scene gets what is left of the window and is scaled into it.
  At 500 pixels (Firefox's smallest) the scene is 181 pixels wide and its smallest control 3 pixels high.
  At 390 pixels (Safari) the scene is 69 pixels wide, its smallest control 1 pixel high, and the page scrolls sideways.
  At 1500 pixels the smallest control is 14 to 18 pixels high.
  The controls still work with a pointer or a key, but nobody could hit them on a phone.
  The panel is usable on a desktop window and not on a phone.

## Not seen

Safari was not driven by a real person's hands, and it was not run in CI.
The visit never changed the window's zoom, and it used a mouse and no touch.

## Not seen in any browser

- **A screen reader.** Chrome's accessibility tree reports the Snapshot range as its number and ignores `aria-valuetext`.
  So a reading in VoiceOver or NVDA is still needed to know whether "Snapshot N of M" is announced.
- **A phone.** The layout was tested at 640 px and looked at in screenshots at 900 and 1500 px, and the longer visit measured it at 390 and 500 px: see above.
