//! Internal unit tests for the complex numbers.
//!
//! - [`scalar`] — the arithmetic, the polar form, the roots of unity and the display, for `f64` and `f32` alike.
//! - [`buffer`] — the split buffers and their kernels, checked against the scalar type.

mod buffer;
mod scalar;
