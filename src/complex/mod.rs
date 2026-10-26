//! Complex numbers: a pair of floats with the arithmetic, the polar form and the roots of unity.
//!
//! [`Complex`] and [`Complex32`] are a `repr(C)` pair of `f64` or `f32`, so a slice of them is interleaved real and
//! imaginary parts. They are what [`NodeValues::Complex64`](crate::scene::NodeValues::Complex64) and
//! [`NodeValues::Complex32`](crate::scene::NodeValues::Complex32) hold, and what
//! [`DataFormat::ComplexNumber`](crate::scene::DataFormat::ComplexNumber) writes.
//!
//! Each is generated from one source by a macro, not written as a generic over `num-traits`, so there are no trait
//! bounds to get in the way of inlining, and the two precisions cannot drift apart. Double precision is the default.
//! Single precision is accurate to about one part in ten million, which is visible when a result that ought to be
//! exactly zero is displayed.
//!
//! Three choices are made for speed.
//!
//! - Every small method is `#[inline]`, so a call from another crate is inlined without link-time optimisation.
//! - Nothing here uses `mul_add`. WebAssembly has no fused multiply-add, so on the wasm target it becomes a call into a
//!   software routine that is far slower than a multiply and an add.
//! - Multiplying by `i` or `-i` (`mul_i`, `mul_neg_i`) swaps the two parts and negates one, with no multiplication,
//!   which is what a radix-2 butterfly wants at its quarter-turn twiddle factor.
//!
//! The roots of unity are built by `unit_root`, which returns a [`RootOfUnityError`] and does not panic, and is exact at every multiple of a quarter turn and reduces every
//! other angle to the first eighth of a turn before calling `sin_cos`, so that the table a transform reads is both
//! symmetric and as accurate as the precision allows.
//!
//! Complex division is Smith's algorithm for numbers of ordinary size. For numbers near the ends of the range it
//! scales the numerator and the divisor by powers of two first, so that it is right at any size, and never `NaN`
//! where the quotient is finite. Division by a real is a true division. Everything else is the schoolbook formula.

use std::{
    fmt,
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

mod scalar;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Error during the calculation of the roots of unity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RootOfUnityError {
    /// The order was zero. There is no zeroth root of unity: a root of order `n` is one of `n` evenly spaced points
    /// round the unit circle, and there is no way to space zero of them.
    ZeroOrder,
}

impl fmt::Display for RootOfUnityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroOrder => write!(f, "there are no roots of unity of order zero"),
        }
    }
}

impl std::error::Error for RootOfUnityError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
scalar::define_complex!(Complex, f64, "A double precision complex number `re + im·i`");
scalar::define_complex!(Complex32, f32, "A single precision complex number `re + im·i`");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
