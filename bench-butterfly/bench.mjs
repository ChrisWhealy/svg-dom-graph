// Times the butterfly kernels in `out/scalar.wasm` (built without simd128) and `out/simd.wasm` (built with it), in Node.
//
// Two tables per precision and size:
//
//   Stages: one stage at a time, in ns per butterfly, with the scaling pass that every stage function also does taken
//   off. `half` is the block size, so `half = 1` is the first stage of a transform and `half = n / 2` the last. This is
//   where the shape of the result shows: how a block that is smaller than a vector register fares against a large one.
//
//   Transforms: every stage of an n-point transform, in ns per butterfly, with the scaling pass left in.
//
// "split" is the `ComplexBuf` under test. "interleaved" is a `Vec<Complex>` doing the same sums with the scalar type.
// The columns of each row are checked to compute the same checksum.
//
// Run it through `run.sh`, which builds both files first. Use `node --no-liftoff` to be sure that V8 has used its
// optimising compiler by the time it is timed. Timings are best of 15. They are only as steady as the machine.
import { readFileSync } from 'node:fs';

const load = async (file) => {
  const module = await WebAssembly.compile(readFileSync(new URL(file, import.meta.url)));
  // The module carries bindings that no function here calls, so any import can be a stub.
  const imports = {};
  for (const { module: m, name, kind } of WebAssembly.Module.imports(module)) {
    imports[m] ??= {};
    imports[m][name] = kind === 'function' ? () => 0 : undefined;
  }
  return (await WebAssembly.instantiate(module, imports)).exports;
};

const scalar = await load('./out/scalar.wasm');
const simd = await load('./out/simd.wasm');
const BUILDS = [['split scalar', scalar, 'soa'], ['split simd', simd, 'soa'], ['interleaved scalar', scalar, 'aos'], ['interleaved simd', simd, 'aos']];
const SIZES = [1024, 4096];
const PRECISIONS = ['f64', 'f32'];
const REPEATS = 15;

const best = (call) => {
  let fastest = Infinity;
  for (let i = 0; i < REPEATS; i++) {
    const start = performance.now();
    call();
    fastest = Math.min(fastest, performance.now() - start);
  }
  return fastest;
};
const agree = (sums) => Math.max(...sums) - Math.min(...sums) < 1e-2 * (1 + Math.abs(sums[0]));

for (const n of SIZES) {
  const stages = [];
  const transforms = [];
  for (const prec of PRECISIONS) {
    const reps = Math.floor(4e6 / n);
    const pairs = reps * (n / 2);
    for (const half of [1, 2, 4, 8, 16, 64, 256, n / 2]) {
      const row = { prec, half };
      const sums = [];
      for (const [label, wasm, layout] of BUILDS) {
        const stage = wasm[`stage_${layout}_${prec}`];
        const base = wasm[`base_${layout}_${prec}`];
        sums.push(stage(n, half, 3));
        // The base scales twice per rep, a stage once.
        const scaling = best(() => base(n, reps)) / 2;
        row[label] = (Math.max(best(() => stage(n, half, reps)) - scaling, 0) * 1e6 / pairs).toFixed(2);
      }
      row.agree = agree(sums);
      stages.push(row);
    }

    const stageCount = Math.log2(n);
    const transformReps = Math.max(1, Math.floor(2e6 / (n * stageCount)));
    const butterflies = transformReps * (n / 2) * stageCount;
    const row = { prec };
    const sums = [];
    for (const [label, wasm, layout] of BUILDS) {
      const transform = wasm[`transform_${layout}_${prec}`];
      sums.push(transform(n, 3));
      row[label] = (best(() => transform(n, transformReps)) * 1e6 / butterflies).toFixed(2);
    }
    row.agree = agree(sums);
    transforms.push(row);
  }
  console.log(`\nStages: ${n}-point buffer, ns per butterfly, scaling pass subtracted (best of ${REPEATS})`);
  console.table(stages);
  console.log(`Transforms: ${n} points, every stage, ns per butterfly, scaling pass included (best of ${REPEATS})`);
  console.table(transforms);
}
