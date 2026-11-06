## perf-smoke: every deterministic gate passed and 2 comparison(s) were reported

### w2-connector-rerouting-smoke

### w2-connector-rerouting-smoke, layer A

Baseline `9630405c4f` (promoted 2026-11-06, 3 runs) against candidate `9630405c4f` (3 runs). Ratio tolerance 10%, 95% interval, 4000 resamples, seed 7.

Statistical verdicts only report: the baseline has 3 runs, and blocking needs 10.

3 diagnostic

Diagnostic only, with no interval and no verdict: 3 baseline and 3 candidate repetitions, and a verdict needs at least 5 on each side. The figures are shown and no interval or verdict is given.

| metric | statistic | baseline median | candidate median | ratio |
|---|---|---|---|---|
| rust.connector_endpoints | p50 | 0.04 | 0.04 | 1.101 |
| rust.connector_endpoints | p95 | 0.06 | 0.07 | 1.040 |
| rust.connector_endpoints | p99 | 0.06 | 0.07 | 1.145 |

### w2-connector-rerouting-smoke, layer B

Baseline `9630405c4f` (promoted 2026-11-06, 3 runs) against candidate `9630405c4f` (3 runs). Ratio tolerance 10%, 95% interval, 4000 resamples, seed 7.

Statistical verdicts only report: the baseline has 3 runs, and blocking needs 10.

1 diagnostic

Diagnostic only, with no interval and no verdict: 3 baseline and 3 candidate repetitions, and a verdict needs at least 5 on each side. The figures are shown and no interval or verdict is given.

| metric | statistic | baseline median | candidate median | ratio |
|---|---|---|---|---|
| mutation.move_node | p50 | 3.24 | 3.29 | 1.014 |

