# Gallery panel, P0: staging and injector proof

The specification is `design-specs/dess_demo.pdf`, revision 4.
The specification names this folder `dess/gallery/`.
It is `dess/demo-panel/` here, since it supports the demo gallery's simulator panel and is not a gallery of its own.
P0 comes first, and it decides whether the cross-size scenario switch can be built at all.
Nothing in the gallery is built until its gates are recorded here.

P0 is not a gallery acceptance test.
The real-gallery tests G01 to G20 are in `dess/demo-panel/test-gallery.mjs`, which a later phase adds.

## Table of Contents

- [Layout](#layout)
- [P0 implementation notes](#p0-implementation-notes)
- [Exit condition](#exit-condition)
- [What this README must record](#what-this-readme-must-record)

## Layout

```text
dess/demo-panel/
├── p0/
│   ├── README.md
│   ├── staging.mjs
│   ├── injector.mjs
│   └── evidence/
│       ├── staging.json
│       ├── injector.json
│       └── features.txt
└── test-gallery.mjs          # Added in a later phase
```

| File | Holds |
|---|---|
| `staging.mjs` | The staging probe: presence, geometry, focus, pointer and accessibility of a candidate host |
| `injector.mjs` | The self-test of the shared injector's detach mode |
| `evidence/staging.json` | What the staging probe observed |
| `evidence/injector.json` | The armed and fired records of the detach self-test, with the call counts before and after |
| `evidence/features.txt` | The two `cargo tree` outputs of the feature-isolation check |

## P0 implementation notes

These notes are normative for P0.

### P0-C: use the existing `service_perf` fixture

Run the injector against the existing harness page, which already holds a real `Surface` and controller.
Load `perf/page/fault-injector.js` before the page's application scripts, using the existing CDP mechanism.
Arm a synchronous detach during an actual overlay or data-node mutation.
Then verify the firing ordinal, the controller's outcome and its recovery behaviour.

No temporary gallery panel is needed.
G14 repeats the relevant test against the finished gallery in P2.

### P0-D: prove that the feature-isolation check works

Run both commands.

```sh
cargo tree -p svg-dom-graph-demo --target wasm32-unknown-unknown -e features
cargo tree -p perf-harness-wasm --target wasm32-unknown-unknown -e features
```

The demo graph must exclude the `fault-hook` feature of `service-sim`.
The harness graph must include it.
That is a positive control: it shows that the check can find the feature when it is there.

Repeat the demo check in P2, once `service-sim` is a demo dependency.
At that point also require `service-sim` itself to be present in the demo graph, so that the check is not satisfied by its absence.

### P0 evidence

Keep the spike apart from the gallery acceptance tests.
Use `dess/demo-panel/p0/` for the staging probe, the injector self-test, the recorded browser observations and the feature-graph evidence.
Reserve `dess/demo-panel/test-gallery.mjs` for the real-gallery tests that follow.

### P0-F: compatibility of the shared injector

The existing `selftest-recovery.mjs` compatibility run remains part of P0-F.
The new detach mode must be unarmed by default, and it must preserve the existing recorder's counts.
It must demonstrate both a successful firing and a correctly reported non-firing.

## Exit condition

The exit condition is unchanged.
P0 is complete when the following three things are shown.

1. A verified staging strategy.
2. A proven synchronous detach mechanism.
3. Demonstrated feature isolation.

The two additional checks above make that evidence stronger, and they do not widen the approved scope.
A failed required gate blocks P1.

## What this README must record

When P0 is run, this section is completed with the following.

| Record | Content |
|---|---|
| Tested commit | The git commit that was measured |
| Chrome version | The browser that the staging and injector results come from |
| Commands | The exact commands that produced each piece of evidence |
| Observed results | The measurements, readbacks and armed and fired records |
| Conclusions | Pass or fail for each gate, with the chosen staging strategy |

Distinguish a gate that was designed from a result that was measured.
Chrome is the release gate, and Firefox and Safari results are recorded as observations.

## P0-A result: staging

Run it with `node dess/demo-panel/p0/staging.mjs`, which writes `evidence/staging.json`.
It builds the real simulator surface and controller into a connected host under each candidate's CSS, and then probes it.

| Item | Value |
|---|---|
| Browser | HeadlessChrome 157.0.0.0 |
| Toolchain | rustc 1.99.1, wasm-pack 0.15.1, Node v26.13.0 |
| Tree | Commit 9d0af83, with uncommitted changes, so the run is to be repeated from a clean commit before it is cited |

| Candidate | CSS and attributes | Result |
|---|---|---|
| `opacity_inert` | `opacity:0; pointer-events:none` and `inert`, in the flow of the panel | Passes S1 to S4 |
| `offscreen_inert` | `position:absolute; left:-20000px; width:1000px` and `inert` | Passes S1 to S4 |

The strategy that is selected is `opacity_inert`, the first candidate, so the fallback is not needed.

What was observed for the selected candidate:

- **Presence:** the host is connected, `checkVisibility()` is true, and the host's own presence test says `Live`, with a clean controller.
- **Geometry:** the surface was built under the staging and drew, with a finite 1000 by 480 box and one content group.
  All 54 text elements are measurable, the data node's setter was called, and the page equals the projection.
- **Isolation:** twelve Tab presses never reach the container, a scripted `focus()` on an SVG probe and an HTML probe does not take, and a real mouse click reaches neither.
  None of the 33 names in the scene is in the accessibility tree.
- **Publish and rollback:** with the staging removed, Tab reaches both probes, a click reaches both, and all 33 names are exposed, with presence still `Live`.
  Staging it again isolates it as before.

Two controls show that the probe can see.

- **A shown scene** is found by every check: Tab, scripted focus, a real click and the accessibility tree.
- **`aria-hidden` without `inert`** is the negative control.
  The accessibility tree has none of the scene, but Tab and scripted focus still reach it.
  That confirms the specification's statement that `aria-hidden` alone does not isolate focus.

The probe reads the accessibility tree before anything takes focus.
Chrome exposes content that has the focus even under `aria-hidden`, so a later reading would call a leaking scene hidden.

## P0-B and P0-C result: the injector's detach mode

Run it with `node dess/demo-panel/p0/injector.mjs`, which writes `evidence/injector.json`.
It needs `evidence/injector-baseline.json`, which `injector-baseline.mjs` captured from the injector before the mode was added.

The mode is `__faults.armDetach({ svgId, method, nth })`, with `disarmDetach()` and `reattach()`, in `perf/page/fault-injector.js`.
It makes the nth call of `method` inside the `<svg>` natively, and then removes the `<svg>` before control returns to the caller.
It is off until it is armed, it is one-shot, and it is refused while an exception is armed or a detach already is.

| Check | Observed |
|---|---|
| Compatibility, unarmed | One slice makes the same 8 counted calls in the same order, and arming three ordinals counts and fails what it did before |
| Firing | Armed at calls 1, 4 and 8 of 8, it fires once at that call, and the report names the method, the ordinal, the tag and the attribute |
| Controller outcome | `Unavailable`, the cursor unmoved, the page dirty and the host `Detached` |
| Recovery | After `reattach()` a repair makes the page the committed slice again, and the next slice commits |
| Not firing | An ordinal past the last reports not fired, and the helper that demands a firing throws |
| Counting while armed | With a detach armed that cannot fire, recording and an armed exception count what they do without it |
| Cleanup | No arm state, timer, frame or listener is left, and the counting afterwards is the prior vector again |
| Shared injector | The graph recovery self-test, `selftest-recovery.mjs`, still passes |

The self-test fails for the right reasons.
A detach that fires at the wrong ordinal, and an injector that stops counting while a detach is armed, were each made on purpose, and each was caught.

The detach interrupts a real `Surface` draw through the harness fixture, as P0-C requires, and no temporary gallery panel was built.
G14 repeats it on the finished gallery in P2.

## P0-D result: feature isolation

Run it with `node dess/demo-panel/p0/features.mjs`, which writes the two complete trees to `evidence/features.txt`.
It reads the resolved feature graph of each build with `cargo tree -p <package> --target wasm32-unknown-unknown -e features`.

| Build | Needs to show | Observed |
|---|---|---|
| `svg-dom-graph-demo` | No `service-sim` feature `fault-hook` | Not present |
| `perf-harness-wasm` | `service-sim` feature `fault-hook`, as the positive control | Present |

The demo does not yet depend on `service-sim`, so before P2 its graph has no `fault-hook` because it has no `service-sim`.
That is recorded as what it is, and not as a result about the hook.

Three things show that the check can tell.

- **A parser check** shows that the reading finds the feature in a tree that has it, and not in one that has the crate without it or lacks the crate.
- **The positive control** shows that the same reading finds the feature in the harness graph.
- **Two edits of the demo's dependencies**, made temporarily and then undone, show both outcomes.
  A dependency on `service-sim` with `features = ["fault-hook"]` fails the check.
  A dependency without the feature passes it, with `service-sim` present in the graph.

P2 runs the check with `--require-service-sim`.
Then the absence of the hook is a fact about a dependency that exists.

The check reads the demo package's own resolution, which is what the demo's wasm build uses.
A build of the whole workspace at once would unify the feature with the harness's, and the demo is not built that way.

## P0 status

All three gates have a result.

| Gate | Result |
|---|---|
| Staging strategy | `opacity:0` plus `inert` passes, and the off-screen fallback also passes |
| Synchronous detach | Proven on a real surface draw, with the shared injector unchanged when unarmed |
| Feature isolation | The demo graph has no `fault-hook`, with a positive control |

Not yet done: a repeat of the evidence from a clean commit, and Firefox and Safari observations.
