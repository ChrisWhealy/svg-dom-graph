//! Complex numbers, for the demos that transform signals.
//!
//! - [`Complex`] and [`Complex32`] — one number, in double and single precision. A `repr(C)` pair, so a slice of them
//!   is interleaved real and imaginary parts. Right for one value, for display and for tests.
//! - [`ComplexBuf`] and [`ComplexBuf32`] — many numbers, stored as one array of real parts and one of imaginary parts.
//!   Right for a transform, because a vector register then holds several real parts or several imaginary parts, and a
//!   complex product needs no shuffling. The kernels are plain loops that the compiler vectorises, see [`buffer`].
//!
//! Each pair is generated from one source by a macro, not written as a generic over `num-traits`, so there are no trait
//! bounds to get in the way of inlining, and the two precisions cannot drift apart. Double precision is the default.
//! Single precision holds twice as many numbers in a vector register, and is accurate to about one part in ten million,
//! which is visible when a result that ought to be exactly zero is displayed.
//!
//! Three choices are made for speed.
//!
//! - Every small method is `#[inline]`. The demos are separate crates, and without the attribute a call into this crate
//!   would not be inlined unless link-time optimisation were switched on.
//! - Nothing here uses `mul_add`. WebAssembly has no fused multiply-add, so on the wasm target it becomes a call into a
//!   software routine that is far slower than a multiply and an add.
//! - Multiplying by `i` or `-i` (`mul_i`, `mul_neg_i`) swaps the two parts and negates one, with no multiplication,
//!   which is what a radix-2 butterfly wants at its quarter-turn twiddle factor.
//!
//! The roots of unity are built by `unit_root`, which is exact at every multiple of a quarter turn and reduces every
//! other angle to the first eighth of a turn before calling `sin_cos`, so that the table a transform reads is both
//! symmetric and as accurate as the precision allows.
//!
//! Complex division uses Smith's algorithm, so it does not overflow when the plain formula would. Everything else is
//! the schoolbook formula.
//!
//! On WebAssembly the vector instructions are used only when the build enables `simd128`, see `.cargo/config.toml`.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub mod buffer;
mod scalar;

buffer::define_buffer!(
    ComplexBuf,
    Complex,
    f64,
    "A buffer of [`Complex`] numbers stored as a real array and an imaginary array."
);
buffer::define_buffer!(
    ComplexBuf32,
    Complex32,
    f32,
    "A buffer of [`Complex32`] numbers stored as a real array and an imaginary array."
);
scalar::define_complex!(Complex, f64, "A complex number `re + im·i`, in double precision.");
scalar::define_complex!(Complex32, f32, "A complex number `re + im·i`, in single precision.");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
