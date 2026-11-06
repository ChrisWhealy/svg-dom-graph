#!/bin/zsh
cargo clippy --lib --tests -- -D warnings
cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p demo-server --all-targets -- -D warnings
cargo clippy -p demo-common --all-targets -- -D warnings
cargo clippy -p svg-dom-graph-demo -p demo-common --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p cdp-test-fixture --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p cdp-integration-test --all-targets -- -D warnings
cargo clippy -p perf-harness --all-targets -- -D warnings
cargo clippy -p perf-harness --all-targets --no-default-features -- -D warnings
cargo clippy -p perf-harness --lib --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo clippy -p perf-harness-wasm --all-targets -- -D warnings
cargo clippy -p perf-harness-wasm --lib --target wasm32-unknown-unknown -- -D warnings
cargo clippy --manifest-path perf/repro/wasm-bindgen-externref-slot/Cargo.toml --lib --tests --target wasm32-unknown-unknown -- -D warnings
