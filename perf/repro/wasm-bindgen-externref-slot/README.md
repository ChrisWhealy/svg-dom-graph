# wasm-bindgen: a failed import that returns an object loses an externref slot

A minimal reproduction with nothing of this repository in it: no DOM, no web-sys, no svg-dom, no svg-dom-graph.
It has three imports written in JavaScript that throw on request, one that returns an object, one that returns nothing and one that returns a number, each declared `#[wasm_bindgen(catch)]`.
The test calls each of them 1000 times with a failure and compares `wasm_bindgen::externref_heap_live_count()` before and after.

| Import            | Throws | Slots left live after 1000 calls |
| ----------------- | ------ | -------------------------------- |
| returns an object | yes    | **1000**                         |
| returns nothing   | yes    | 0                                |
| returns a number  | yes    | 0                                |
| returns an object | no     | 0                                |
| returns nothing   | no     | 0                                |

Measured with wasm-bindgen 0.2.129 and 0.2.130 (the latest published on 2026-11-06).
The caller drops the `Err` at once, so nothing in the caller holds the slot.

## Running it

```sh
wasm-pack test --node                       # in this directory; needs wasm-pack and Node
node check.mjs                              # the pinned version; exits 0 if the leak is present, as expected
node check.mjs --version 0.2.131            # another version
node check.mjs --expect fixed               # exits 0 only if the leak is gone, and the controls still leave nothing
```

`check.mjs` copies the crate to a temporary directory, pins the version there and reads the three tests.
It says `leak`, `fixed` or `other`.
`other` means the behaviour changed shape and has to be looked at before it is believed.

## Why this is kept in the repository

The `svg-dom-graph` harness accounts for these slots (`endurance.lost_externref_slots_are_explained`), which reports the behaviour without removing it.
This crate is the test that tells a dependency update apart.
When a wasm-bindgen release fixes the problem,

- `a_failed_import_that_returns_an_object_should_leave_no_slot` passes, and
- `today_a_failed_import_that_returns_an_object_leaves_one_slot_each` fails,

and `node check.mjs` (expecting `leak`) exits 1 in CI.
That is the signal to bump the pin, change the expectation to `fixed`, and remove the accounting from the harness.

## Report for upstream

See `UPSTREAM_ISSUE.md`.
It has not been filed.
