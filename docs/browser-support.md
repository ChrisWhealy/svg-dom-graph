# Browser support

This is a proposal, not a support contract.
It says what the test evidence currently supports, so that the decision can be made against it.
A browser that is outside the contract is not a release blocker, and one that is inside it is.

## The tiers

| Tier                | Browsers                                       | What it means                                                                                                                                                |
| ------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1, the release gate | Current stable Chrome                          | Every automated test passes on it before a release: the CDP suite, the browser tests, the gallery script and the harness                                     |
| 2, observed         | Current Firefox and current Safari             | Each release is looked at with `webdriver-observe.mjs`. A defect is fixed, and it does not block a release unless it makes the library or the panel unusable |
| Not supported       | Older than the floor below, and phone browsers | Nothing is promised, and nothing is tested                                                                                                                   |

Moving Safari to tier 1 means running the gallery script and the browser tests on it in CI, which needs a macOS runner.
Moving it out of the contract means saying so here, and the Safari observations stop being kept.

## The floor

These are the oldest versions that have the features the library and the demo use.
They are from MDN's browser compatibility data, which was read on 2026-11-12.
The table is to be checked again when the contract is published.

| Feature                                   | Used by                                   | Chrome | Firefox | Safari                     |
| ----------------------------------------- | ----------------------------------------- | ------ | ------- | -------------------------- |
| WebAssembly reference types (`externref`) | every wasm build                          | 96     | 79      | 15.1                       |
| `foreignObject` controls                  | the gallery panels                        | any    | any     | any, with the checks below |
| `inert`                                   | the staged scene of the service simulator | 102    | 112     | 15.5                       |
| `:has()`                                  | the colours of the server boxes           | 105    | 121     | 15.4                       |

The highest of each column is the floor of the whole gallery: Chrome 105, Firefox 121, Safari 15.5.
The library itself, without the gallery, needs less.

## What was observed

The details are in [the observations](../dess/demo-panel/p4/browser-observations.md).

| Browser         | Version     | Observed                                                                                                                                        |
| --------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Chrome          | 157         | Everything, automated                                                                                                                           |
| Firefox         | 159, macOS  | The surface and controller tests, and a scripted visit with pointer, keyboard, playback, hide and show, switching, failures and a narrow window |
| Safari          | 27.2, macOS | The same visit                                                                                                                                  |
| A screen reader | none        | Not yet: see [the manual checklist](../dess/demo-panel/p4/manual-checklist.md)                                                                  |
| A phone         | none        | Not yet, and the layout does not fit a narrow window                                                                                            |

## What is not covered

- **A screen reader** announcing "Snapshot N of M".
- **A touch screen.** The pointer path was driven with a mouse.
- **A narrow window.** The gallery's menu takes 272 pixels, and the scene is scaled into what is left, so its controls are a few pixels high below about 800 pixels.
- **Versions at the floor.** The browsers observed are the current ones.
