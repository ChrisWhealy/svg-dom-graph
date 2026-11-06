# Evidence

`evidence/phase5/` holds what the Phase 5 checks printed, the self-tests and the calibration.
It also holds two runs of the perf-smoke profile.
The first had no baseline, and the second ran against a baseline promoted from the first.
The second run has its comparison reports and summary.
`node perf/runner/record-phase5-evidence.mjs` writes it again.

`evidence/phase4/` holds what the Phase 4 checks printed, the recovery and endurance result documents with the table of every failure that was injected, and `findings.json`.
`node perf/runner/record-phase4-evidence.mjs` writes it again.

`evidence/phase3/` holds what the Phase 3 checks printed, the spike's raw output, the W4 result documents and raw samples, and the gallery report.
`node perf/runner/record-phase3-evidence.mjs` writes it again.

`evidence/phase2/` holds what the Phase 2 checks printed, the allocator verification, and the result documents and raw samples of the realistic manifests.
`node perf/runner/record-phase2-evidence.mjs` writes it again.

`evidence/phase1/` holds what the Phase 1 checks printed, the reports of the reference manifests and the environment, and is described in [Phase 1: the workload engine](phase-1-workload-engine.md).

`evidence/phase0/` holds what the Phase 0 experiments printed, and the environment they ran in.

| File | Holds |
|---|---|
| `environment.json` | Commit and uncommitted changes, machine, full Chrome version, Node, `rustc` and `wasm-pack` versions, the SHA-256 and size of each built fixture file, and each command with its exit code |
| `timer-and-counters-not-isolated.json` | Timer resolution, Chrome's counters, and the scene and listener figures, for a page that is not cross-origin isolated |
| `timer-and-counters-isolated.json` | The same, served with COOP and COEP |
| `listener-ownership.txt` | Which registrations a second scene build added and removed (finding 7) |
| `listener-tracker-tests.txt` | The result of the tracker's tests, including the check against Chrome's count |

To record it again, for example after a new Chrome or a change to `svg-dom`:

```sh
node perf/spike/record-evidence.mjs --build
```

`--build` rebuilds the fixture first.
`cdp-test-fixture/pkg/` is not committed, so without `--build` the record says when the existing one was built, and it may not match the checkout.
Compare the new `environment.json` with the old one before comparing any figure, since a different Chrome, `svg-dom` or fixture hash can explain a difference.
The Phase 0 figures were first taken against an earlier fixture build.
They were reproduced exactly on this one: 100 µs and 5 µs, 780 and 1,560 elements, and 139 listeners before a collection and 37 after.

Port numbers are replaced with `PORT`, since they change every run.
`uncommitted_changes` lists what was not committed when the record was made.
