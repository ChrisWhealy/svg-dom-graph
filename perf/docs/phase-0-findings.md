# Phase 0 findings

Measured on one machine: macOS, headless Chrome 157.0.8103.39, against a `cdp-test-fixture` built from commit `2fc8997`.
The raw outputs are kept in [`evidence/phase0/`](../evidence/phase0/), and the figures below can be checked against them.
Take them as answers to design questions, not as performance figures.

## 1. Leaving a gallery panel does not tear it down

`init_panel` builds a panel once and returns at once on every later call, and `selectDemo` only changes which section is active.
So a loop of selecting and leaving a gallery panel measures nothing about teardown.

`Scene` has no public remove or destroy call.
Its `Drop` only flushes a pending view write, and the DOM it drew stays where it is.
What exists is dropping the last `Scene` handle and removing the host `<svg>`.
Three demos (`n_queens`, `tarjan`, `springs`) also stop their own clocks on `pagehide`.

Consequences for the design:

- W3 (repeated scene lifecycle) is a Layer B workload: build on a fresh `<svg>`, drop the handle, remove the element.
- The gallery adapter reports load and revisit cost, and says plainly that it does not exercise teardown.

## 2. DOM ids are not stable across scenes

The only `id` the scene writes into its own subtree is the arrow marker, `svg-dom-graph-arrow-N`.
`N` comes from `NEXT_SCENE_ID`, a process-wide counter, so a second scene in the same page gets `-1` and never `-0`.
The fixture confirmed it: two runs gave two markers and nothing else with an id.
Checkpoints therefore compare by logical key, which is the name the scene was given, and check only that ids are unique.

## 3. The library uses no `on*` property handlers

Every listener goes through `add_event_listener_with_callback`.
The tracker needs no `on*` handling.
The library itself does not use `once`, `AbortSignal` or listener options, but a demo might.
So the tally keeps the full key, and treats a duplicate registration as one.

## 4. Closures that are leaked on purpose are invisible to the JS heap and the listener count

`svg_buttons.rs` and `sliders/mod.rs` call `Closure::forget()` (three call sites in the library, and one more in `demo-common`).
The boxed closure and what it captured stay allocated in the wasm module for the life of the page.
Neither `JSHeapUsedSize` nor a listener count shows it.

The size of the wasm linear memory does not reliably show it either, and it must not be read as live memory:

- Linear memory grows in 64 KiB pages and is never handed back, so its size is a high-water mark of what was ever needed.
- A constant size can hide retained allocations, because a freed block can be reused by a retained one.
- Growth is not by itself a leak.
  It can be the first use of capacity that later cycles then reuse.

So the harness keeps wasm memory out of the heap and listener accounting and reports it as two separate things:

| Figure | Kind | What it can tell you | Where |
|---|---|---|---|
| Linear-memory size (`capture_wasm_capacity`) | `capacity`, a high-water mark | How much the module ever needed. Reported as a diagnostic | Layers B and C |
| Live bytes and allocation count (`capture_wasm_live_allocation`) | `count` | What is allocated now, so a `forget`ten closure shows as live bytes that do not come back | Layer B only |

Live bytes come from a counting global allocator in `perf-harness-wasm`.
The allocator is chosen by the final module, so the library links into it unchanged and its allocations are counted.
The demo gallery's module has no such allocator, so Layer C reports capacity only.
This is a design, not a result.
Its figures are not authoritative until Phase 2 has verified it with a deliberately retained allocation:

1. Allocate a known number of bytes and keep them.
   Live bytes must rise by at least that number, and the allocation count by at least one.
2. Release them.
   Live bytes must fall back to within a stated tolerance of where they started.
3. Leak a closure with `Closure::forget()`.
   Live bytes must rise and stay risen.
4. Repeat each step many times, so that a counter that drifts or double-counts shows.

A repeated lifecycle test reads these figures differently from a heap trend:

- It does not expect linear memory to return to its starting size after a teardown.
- After a warm-up of cycles it asks whether capacity is reused: the number of times memory grew during the measured cycles should be zero once it has levelled off (`wasm.memory_grow_events_after_warmup`).
- Live bytes after a teardown are compared with live bytes before the build, as listener counts are.
- Growth in capacity with flat live bytes is reported as an observation to examine, not as a leak.
