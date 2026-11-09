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
cargo clippy -p service-sim-engine --all-targets -- -D warnings
cargo clippy -p service-sim-engine --lib --bins --target wasm32-wasip1 -- -D warnings
cargo clippy -p service-sim --all-targets -- -D warnings
cargo clippy -p service-sim --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p service-sim --lib --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p spmv-core -p spmv-scene --all-targets -- -D warnings
cargo clippy -p spmv-scene --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p spmv-core --lib --bins --target wasm32-wasip1 -- -D warnings
