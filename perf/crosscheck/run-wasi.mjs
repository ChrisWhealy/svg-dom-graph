// A `cargo` test runner that runs a wasm32-wasip1 test binary in Node, so that the unit tests can run on a 32-bit platform.
//
//   CARGO_TARGET_WASM32_WASIP1_RUNNER="node perf/crosscheck/run-wasi.mjs" \
//     cargo test -p perf-harness --no-default-features --target wasm32-wasip1
//
// Cargo calls it with the path of the test binary, and then the arguments for the test harness.
import fs from 'node:fs'
import { WASI } from 'node:wasi'

const [wasmPath, ...args] = process.argv.slice(2)
const wasi = new WASI({ version: 'preview1', args: [wasmPath, ...args], env: { RUST_BACKTRACE: '0' }, returnOnExit: true })
const module = await WebAssembly.compile(fs.readFileSync(wasmPath))
const instance = await WebAssembly.instantiate(module, wasi.getImportObject())
process.exit(wasi.start(instance))
