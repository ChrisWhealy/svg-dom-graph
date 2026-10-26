//! Does LLVM vectorise the split-buffer butterfly, and is it faster than interleaved scalar code?
//!
//! Every function here is `extern "C"`, takes plain integers and returns a checksum, so that `bench.mjs` can call it from
//! Node with no bindings. The checksum keeps the optimiser from removing the work, and the script compares it across the
//! builds to show that all of them compute the same numbers.
//!
//! For each precision there are three groups of functions:
//!
//! - `butterfly_block_*`, one block of butterflies that is never inlined, so that its code can be read on its own in the
//!   disassembly (`run.sh` counts its vector instructions).
//! - `stage_soa_*` and `stage_aos_*`, `reps` passes of a single stage over a buffer. `soa` is the split buffer under test,
//!   `aos` an interleaved `Vec<Complex>` doing the same sums with the scalar type. `base_*` is the scaling pass that the
//!   stage functions also do, timed alone so that it can be subtracted.
//! - `transform_soa_*` and `transform_aos_*`, every stage of an `n`-point transform, one call per block.
use demo_common::complex::{ComplexBuf, ComplexBuf32};
use svg_dom_graph::complex::{Complex, Complex32};

macro_rules! kernels {
    (
        $block:ident, $stage_soa:ident, $stage_aos:ident, $base_soa:ident, $base_aos:ident,
        $transform_soa:ident, $transform_aos:ident, $numbers:ident, $buf:ident, $cx:ident, $f:ident
    ) => {
        /// One block of butterflies, not inlined.
        #[inline(never)]
        #[unsafe(no_mangle)]
        pub extern "C" fn $block(data: &mut $buf, twiddles: &$buf, start: usize, half: usize) {
            data.butterfly(start, half, twiddles);
        }

        fn $numbers(n: usize) -> Vec<$cx> {
            (0..n).map(|i| $cx::new(i as $f * 0.5, 1.0 - i as $f * 0.25)).collect()
        }

        /// `reps` passes of one stage, each followed by a scaling that keeps the numbers from growing.
        #[unsafe(no_mangle)]
        pub extern "C" fn $stage_soa(n: usize, half: usize, reps: usize) -> $f {
            let mut data = $buf::from_complex(&$numbers(n));
            let mut twiddles = $buf::unit_roots(2 * half, half).unwrap();
            twiddles.conj();
            for _ in 0..reps {
                for start in (0..n).step_by(2 * half) {
                    data.butterfly(start, half, &twiddles);
                }
                data.scale(std::$f::consts::FRAC_1_SQRT_2);
            }
            data.re()[0] + data.im()[n - 1]
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $stage_aos(n: usize, half: usize, reps: usize) -> $f {
            let mut data = $numbers(n);
            let twiddles: Vec<$cx> = (0..half).map(|k| $cx::unit_root(k, 2 * half).unwrap().conj()).collect();
            for _ in 0..reps {
                for start in (0..n).step_by(2 * half) {
                    let (lo, hi) = data[start..start + 2 * half].split_at_mut(half);
                    for i in 0..half {
                        let t = hi[i] * twiddles[i];
                        hi[i] = lo[i] - t;
                        lo[i] += t;
                    }
                }
                for z in data.iter_mut() {
                    *z = z.scale(std::$f::consts::FRAC_1_SQRT_2);
                }
            }
            data[0].re + data[n - 1].im
        }

        /// The cost of the scaling pass alone, for `reps` scalings of each, so that it can be taken off a stage. It
        /// scales by a number and then by its reciprocal: scaling one way only would reach denormal numbers, which are
        /// slow, and a stage never does. So this is twice the scaling a stage does, and the script halves it.
        #[unsafe(no_mangle)]
        pub extern "C" fn $base_soa(n: usize, reps: usize) -> $f {
            let mut data = $buf::from_complex(&$numbers(n));
            for _ in 0..reps {
                data.scale(std::$f::consts::FRAC_1_SQRT_2);
                data.scale(1.0 / std::$f::consts::FRAC_1_SQRT_2);
            }
            data.re()[0] + data.im()[n - 1]
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $base_aos(n: usize, reps: usize) -> $f {
            let mut data = $numbers(n);
            for _ in 0..reps {
                for z in data.iter_mut() {
                    *z = z.scale(std::$f::consts::FRAC_1_SQRT_2);
                }
                for z in data.iter_mut() {
                    *z = z.scale(1.0 / std::$f::consts::FRAC_1_SQRT_2);
                }
            }
            data[0].re + data[n - 1].im
        }

        /// `reps` complete `n`-point transforms' worth of stages, from `half = 1` up, with no bit reversal.
        #[unsafe(no_mangle)]
        pub extern "C" fn $transform_soa(n: usize, reps: usize) -> $f {
            let mut data = $buf::from_complex(&$numbers(n));
            let mut stages = Vec::new();
            let mut half = 1;
            while half < n {
                let mut twiddles = $buf::unit_roots(2 * half, half).unwrap();
                twiddles.conj();
                stages.push((half, twiddles));
                half *= 2;
            }
            let norm = 1.0 / (n as $f).sqrt();
            for _ in 0..reps {
                for (half, twiddles) in &stages {
                    for start in (0..n).step_by(2 * half) {
                        data.butterfly(start, *half, twiddles);
                    }
                }
                data.scale(norm);
            }
            data.re()[0] + data.im()[n - 1]
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $transform_aos(n: usize, reps: usize) -> $f {
            let mut data = $numbers(n);
            let mut stages: Vec<(usize, Vec<$cx>)> = Vec::new();
            let mut half = 1;
            while half < n {
                stages.push((half, (0..half).map(|k| $cx::unit_root(k, 2 * half).unwrap().conj()).collect()));
                half *= 2;
            }
            let norm = 1.0 / (n as $f).sqrt();
            for _ in 0..reps {
                for (half, twiddles) in &stages {
                    for start in (0..n).step_by(2 * half) {
                        let (lo, hi) = data[start..start + 2 * half].split_at_mut(*half);
                        for i in 0..*half {
                            let t = hi[i] * twiddles[i];
                            hi[i] = lo[i] - t;
                            lo[i] += t;
                        }
                    }
                }
                for z in data.iter_mut() {
                    *z = z.scale(norm);
                }
            }
            data[0].re + data[n - 1].im
        }
    };
}

kernels!(
    butterfly_block_f64,
    stage_soa_f64,
    stage_aos_f64,
    base_soa_f64,
    base_aos_f64,
    transform_soa_f64,
    transform_aos_f64,
    numbers_f64,
    ComplexBuf,
    Complex,
    f64
);
kernels!(
    butterfly_block_f32,
    stage_soa_f32,
    stage_aos_f32,
    base_soa_f32,
    base_aos_f32,
    transform_soa_f32,
    transform_aos_f32,
    numbers_f32,
    ComplexBuf32,
    Complex32,
    f32
);
