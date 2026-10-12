# Testing

## Table of Contents

- [Running the tests](#running-the-tests)
- [Error handling during testing](#error-handling-during-testing)

| Page | What it covers |
|---|---|
| [Unit Tests](unit-tests.md) | The native, DOM-free tests run by `cargo test` |
| [Demo App Tests](demo-tests.md) | The tests of `demo-app` and `demo-server`, including the hash algorithms |
| [Browser Integration Tests](browser-tests.md) | The tests in `tests/drag/`, run in a real browser by `wasm-pack test`, with a page for each area they cover |
| [Tests Using Chrome DevTools Protocol (CDP)](cdp-tests.md) | Real mouse and key input, and the browser's own accessibility tree |

## Running the tests

| Command | Runs |
|---|---|
| `cargo test` | The native unit tests and doc tests |
| `cargo test -p svg-dom-graph-demo` and `cargo test -p demo-server` | The demo's own tests |
| `wasm-pack test --headless --firefox` and `wasm-pack test --headless --chrome` | The browser integration tests, which CI runs in both browsers |
| `cargo test -p cdp-integration-test` | The CDP tests, against a local Chrome |

## Error handling during testing

All tests in this crate follow the convention that they return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the noisy clutter created by reams of stack trace output.
