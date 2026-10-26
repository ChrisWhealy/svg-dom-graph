# Butterfly benchmark

Does LLVM turn the split-buffer butterfly (`ComplexBuf::butterfly` and `ComplexBuf32::butterfly` in `demo-common`) into
WebAssembly SIMD, and is that faster than plain interleaved scalar code?

```sh
./bench-butterfly/run.sh
```

It needs the `wasm32-unknown-unknown` target and `node`. `wasm2wat` (from wabt) is optional.

The script builds the crate twice, once with no flags and once with `-C target-feature=+simd128`, then does two things:

1. **Reads the code.** It counts the vector instructions (`f64x2.mul`, `f32x4.add`, `v128.load` and so on) in each module.
   The build without the flag must contain none, and the script stops if it does.
2. **Times it** in Node, with `bench.mjs`. A "split" run uses the `ComplexBuf` under test. An "interleaved" run does the
   same sums on a `Vec<Complex>` with the scalar type. Every row checks that the builds compute the same checksum.

`RUSTFLAGS` in the environment replaces the flags in `.cargo/config.toml`, which is why each build sets its own.

## What it showed

Measured on an Intel Core i9-9880H, in Node 22 (V8 12.4) with `--no-liftoff`, on a busy machine. Take the ratios and the
shape of the result as reliable, and the absolute numbers (a few nanoseconds per butterfly) as inflated.

- **The code is vectorised.** The butterfly body holds 4 `f64x2.mul`, 3 `f64x2.add` and 3 `f64x2.sub` with `v128` loads and
  stores, which is the arithmetic of one pair, and the same for `f32x4`. The scalar remainder loop follows it.
- **Large blocks are faster.** For one stage with a block of `n / 2` pairs, the split-buffer SIMD kernel is about 1.5× to
  2× faster than scalar for `f64` and about 3× for `f32`.
- **Medium blocks are about level.** From a block of 16 up to about `n / 8` the SIMD kernel is within noise of scalar.
- **Small blocks are slower.** At `half = 1` the SIMD build takes about 25 ns per butterfly against about 10 for scalar, and
  the penalty falls away by `half = 16`. The vector loop cannot run on a block smaller than a register, and each call
  pays for its checks and its remainder loop.
- **A whole transform gains nothing.** Every stage of an `n`-point transform, one `butterfly` call per block, takes about
  the same time with and without SIMD, for both layouts. The `n / 2` blocks of the first stage cost more, and make up what
  the last stages save. The interleaved scalar loop is the fastest of the four in `f64`.

So LLVM's vector code is not the problem, and explicit intrinsics are not justified. What the numbers point at is the
per-block cost on small blocks. A transform that wants the benefit should do its first stages another way: fuse the first
two or three into one pass, or run all of a stage's blocks inside one call, and leave `butterfly` for blocks that are at
least a register wide.

## Limits

- Node is not a browser. Chrome and Firefox compile WebAssembly SIMD with their own optimisers, so run it there too
  before relying on a number.
- There is no bit reversal in the timed transform, and only the two sizes in `bench.mjs`.
- The scaling pass that each stage function does, to keep the numbers from growing, is timed separately and subtracted
  from the stage table. In the transform table it is left in.
