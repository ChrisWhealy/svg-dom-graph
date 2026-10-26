#!/bin/sh
# Builds the butterfly benchmark twice, without and with WebAssembly SIMD, shows which vector instructions the butterfly
# compiled to, and times both builds in Node.
#
#   ./bench-butterfly/run.sh
#
# Needs the `wasm32-unknown-unknown` target and `node`. `wasm2wat` (from wabt) is optional: without it the instruction
# count is skipped. `RUSTFLAGS` in the environment replaces the flags in `.cargo/config.toml`, so each build sets its own.
set -eu
cd "$(dirname "$0")"
mkdir -p out

build() {
    RUSTFLAGS="$1" cargo build --release --target wasm32-unknown-unknown
    cp target/wasm32-unknown-unknown/release/butterfly_bench.wasm "$2"
}
build "" out/scalar.wasm
build "-C target-feature=+simd128" out/simd.wasm

if command -v wasm2wat >/dev/null 2>&1; then
    for build in scalar simd; do
        wasm2wat --enable-all "out/$build.wasm" -o "out/$build.wat"
        echo "== $build: vector instructions in the whole module"
        for op in f64x2.mul f64x2.add f64x2.sub f32x4.mul f32x4.add f32x4.sub v128.load v128.store; do
            printf '%-10s %s\n' "$op" "$(grep -c "$op" "out/$build.wat" || true)"
        done
    done
    # A build without the flag that contains a vector instruction is not the baseline it claims to be.
    if grep -q 'f64x2\.' out/scalar.wat; then
        echo "error: out/scalar.wasm contains vector instructions, so RUSTFLAGS did not replace the configured flags" >&2
        exit 1
    fi
fi

node --no-liftoff bench.mjs
