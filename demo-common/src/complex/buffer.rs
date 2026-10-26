//! A buffer of complex numbers stored as two arrays, written once and instantiated for `f64` and `f32`.
//!
//! The scalar type keeps a number's two parts side by side, which is right for one number and wrong for a transform.
//! A vector register loaded from interleaved numbers has the real and imaginary parts in neighbouring lanes, so a
//! complex product needs a lane swap, two splats and a sign flip. Here the real parts are one array and the imaginary
//! parts another, so the same product is four multiplications and two additions on whole registers, with no shuffling.
//!
//! No kernel contains a vector instruction. Each is a plain loop over slices of the same length, which LLVM turns into
//! vector code whenever the target has any: SSE2 on x86-64, NEON on Arm, and `simd128` on WebAssembly when the build
//! enables it. The same source is therefore tested natively with no `unsafe` and no second implementation to drift.
//!
//! Two habits make that work. Every slice a loop reads or writes is cut to one length before the loop, so the
//! compiler can see that no index is out of bounds and drops the checks that would stop it vectorising. And the slices
//! a kernel writes to are `&mut`, so the compiler knows they do not overlap anything it reads.

/// Defines a split-layout buffer of one scalar complex type.
macro_rules! define_buffer {
    ($buf:ident, $complex:ident, $float:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $buf {
            re: Vec<$float>,
            im: Vec<$float>,
        }

        impl $buf {
            /// A buffer of `len` zeros.
            pub fn new(len: usize) -> Self {
                Self {
                    re: vec![0.0; len],
                    im: vec![0.0; len],
                }
            }

            /// Splits interleaved numbers into two arrays.
            pub fn from_complex(numbers: &[$complex]) -> Self {
                Self {
                    re: numbers.iter().map(|z| z.re).collect(),
                    im: numbers.iter().map(|z| z.im).collect(),
                }
            }

            /// Joins the two arrays back into interleaved numbers.
            pub fn to_complex(&self) -> Vec<$complex> {
                self.re.iter().zip(&self.im).map(|(&re, &im)| $complex::new(re, im)).collect()
            }

            /// The first `count` of the `n` roots of unity, split, for a transform stage to read in step with its data.
            ///
            /// # Errors
            ///
            /// Returns [`RootOfUnityError::ZeroOrder`](svg_dom_graph::complex::RootOfUnityError::ZeroOrder) if `n` is
            /// zero, even if `count` is too.
            pub fn unit_roots(n: usize, count: usize) -> Result<Self, svg_dom_graph::complex::RootOfUnityError> {
                if n == 0 {
                    return Err(svg_dom_graph::complex::RootOfUnityError::ZeroOrder);
                }
                let mut roots = Self::new(count);

                for k in 0..count {
                    roots.set(k, $complex::unit_root(k, n)?);
                }
                Ok(roots)
            }

            pub fn len(&self) -> usize {
                self.re.len()
            }

            pub fn is_empty(&self) -> bool {
                self.re.is_empty()
            }

            pub fn re(&self) -> &[$float] {
                &self.re
            }

            pub fn im(&self) -> &[$float] {
                &self.im
            }

            /// Both arrays, writable. Their lengths cannot change, so they stay equal.
            pub fn parts_mut(&mut self) -> (&mut [$float], &mut [$float]) {
                (&mut self.re, &mut self.im)
            }

            /// # Panics
            ///
            /// If `index` is out of range.
            #[inline]
            pub fn get(&self, index: usize) -> $complex {
                $complex::new(self.re[index], self.im[index])
            }

            /// # Panics
            ///
            /// If `index` is out of range.
            #[inline]
            pub fn set(&mut self, index: usize, number: $complex) {
                self.re[index] = number.re;
                self.im[index] = number.im;
            }

            /// Every part of every number is finite.
            pub fn is_finite(&self) -> bool {
                self.re.iter().chain(&self.im).all(|part| part.is_finite())
            }

            /// Adds `other` to this buffer, number by number.
            ///
            /// # Panics
            ///
            /// If the buffers differ in length.
            pub fn add_assign(&mut self, other: &Self) {
                let n = self.len();
                assert_eq!(n, other.len(), "buffers of different lengths");
                let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
                let (other_re, other_im) = (&other.re[..n], &other.im[..n]);

                for i in 0..n {
                    re[i] += other_re[i];
                    im[i] += other_im[i];
                }
            }

            /// Subtracts `other` from this buffer, number by number.
            ///
            /// # Panics
            ///
            /// If the buffers differ in length.
            pub fn sub_assign(&mut self, other: &Self) {
                let n = self.len();
                assert_eq!(n, other.len(), "buffers of different lengths");
                let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
                let (other_re, other_im) = (&other.re[..n], &other.im[..n]);

                for i in 0..n {
                    re[i] -= other_re[i];
                    im[i] -= other_im[i];
                }
            }

            /// Multiplies this buffer by `other`, number by number.
            ///
            /// # Panics
            ///
            /// If the buffers differ in length.
            pub fn mul_assign(&mut self, other: &Self) {
                let n = self.len();
                assert_eq!(n, other.len(), "buffers of different lengths");
                let (re, im) = (&mut self.re[..n], &mut self.im[..n]);
                let (other_re, other_im) = (&other.re[..n], &other.im[..n]);

                for i in 0..n {
                    let (a, b, c, d) = (re[i], im[i], other_re[i], other_im[i]);
                    re[i] = a * c - b * d;
                    im[i] = a * d + b * c;
                }
            }

            /// Multiplies every number by a real factor.
            pub fn scale(&mut self, factor: $float) {
                // Two loops, not one over a chain of the two arrays: a chained iterator does not vectorise.
                for part in self.re.iter_mut() {
                    *part *= factor;
                }
                for part in self.im.iter_mut() {
                    *part *= factor;
                }
            }

            /// Replaces every number by its conjugate.
            pub fn conj(&mut self) {
                for part in self.im.iter_mut() {
                    *part = -*part;
                }
            }

            /// Writes the squared magnitude of every number into `out`.
            ///
            /// # Panics
            ///
            /// If `out` is not as long as the buffer.
            pub fn norm_sqr_into(&self, out: &mut [$float]) {
                let n = self.len();
                assert_eq!(n, out.len(), "output of a different length");
                let (re, im, out) = (&self.re[..n], &self.im[..n], &mut out[..n]);

                for i in 0..n {
                    out[i] = re[i] * re[i] + im[i] * im[i];
                }
            }

            /// One radix-2 butterfly stage over one block: the numbers `start..start + half` are `a`, the numbers
            /// `start + half..start + 2 * half` are `b`, and `twiddles` holds one factor `w` for each pair.
            /// Each pair becomes `(a + w·b, a - w·b)`.
            ///
            /// This is the inner loop of a transform. Every operand is read from a contiguous run, so it needs no
            /// gather, and its loads and stores are aligned to whatever the element size gives.
            ///
            /// # Panics
            ///
            /// If the block does not lie inside the buffer, or `twiddles` has fewer than `half` factors.
            pub fn butterfly(&mut self, start: usize, half: usize, twiddles: &Self) {
                let end = start.checked_add(half.saturating_mul(2)).filter(|&end| end <= self.len());
                let end = end.expect("the butterfly block lies outside the buffer");
                assert!(twiddles.len() >= half, "fewer twiddle factors than butterflies");

                let (a_re, b_re) = self.re[start..end].split_at_mut(half);
                let (a_im, b_im) = self.im[start..end].split_at_mut(half);
                let (a_re, a_im, b_re, b_im) =
                    (&mut a_re[..half], &mut a_im[..half], &mut b_re[..half], &mut b_im[..half]);
                let (w_re, w_im) = (&twiddles.re[..half], &twiddles.im[..half]);

                for i in 0..half {
                    let t_re = b_re[i] * w_re[i] - b_im[i] * w_im[i];
                    let t_im = b_re[i] * w_im[i] + b_im[i] * w_re[i];
                    b_re[i] = a_re[i] - t_re;
                    b_im[i] = a_im[i] - t_im;
                    a_re[i] += t_re;
                    a_im[i] += t_im;
                }
            }
        }
    };
}

pub(super) use define_buffer;
