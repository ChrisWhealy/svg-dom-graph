//! Buffers of complex numbers, for the demos that transform signals.
//!
//! The numbers themselves, [`Complex`] and [`Complex32`], live in `svg_dom_graph::complex`, because the library draws
//! them. They are re-exported here.
//!
//! - [`ComplexBuf`] and [`ComplexBuf32`] — many numbers, stored as one array of real parts and one of imaginary parts.
//!   A one-number `repr(C)` pair is right for one value, for display and for tests. A buffer is right for a transform,
//!   because a vector register then holds several real parts or several imaginary parts, and a complex product needs
//!   no shuffling. The kernels are plain loops that the compiler vectorises, see [`buffer`].
//!
//! Each buffer is generated from one source by a macro, so the two precisions cannot drift apart. Single precision holds
//! twice as many numbers in a vector register.
//!
//! On WebAssembly the vector instructions are used only when the build enables `simd128`, see `.cargo/config.toml`.

pub use svg_dom_graph::complex::{Complex, Complex32};

pub mod buffer;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
