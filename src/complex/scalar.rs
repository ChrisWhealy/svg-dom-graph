//! The scalar complex number, written once and instantiated for `f64` and `f32`.

/// Defines a complex number type over one float type, with all its methods and operators.
///
/// The type is written as a macro rather than as a generic over `num-traits`, so it needs no trait bounds, and each
/// instance is plain code that inlines.
macro_rules! define_complex {
    ($name:ident, $float:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        #[repr(C)]
        pub struct $name {
            /// The real part.
            pub re: $float,
            /// The imaginary part.
            pub im: $float,
        }

        impl $name {
            pub const ZERO: Self = Self::new(0.0, 0.0);
            pub const ONE: Self = Self::new(1.0, 0.0);
            /// The imaginary unit.
            pub const I: Self = Self::new(0.0, 1.0);

            #[inline]
            pub const fn new(re: $float, im: $float) -> Self {
                Self { re, im }
            }

            /// A real number as a complex one.
            #[inline]
            pub const fn from_real(re: $float) -> Self {
                Self::new(re, 0.0)
            }

            /// The number at a distance `norm` from the origin, turned by `arg` radians from the positive real axis.
            #[inline]
            pub fn from_polar(norm: $float, arg: $float) -> Self {
                let (sin, cos) = arg.sin_cos();
                Self::new(norm * cos, norm * sin)
            }

            /// The point on the unit circle `arg` radians round: `cos(arg) + i·sin(arg)`.
            #[inline]
            pub fn cis(arg: $float) -> Self {
                let (sin, cos) = arg.sin_cos();
                Self::new(cos, sin)
            }

            /// The `k`th of the `n` complex roots of unity, `e^(2πik/n)`, counted anticlockwise from 1.
            ///
            /// `k` is taken modulo `n`, so any `k` is accepted. A quarter, a half and three quarters of a turn are
            /// exact, so `unit_root(1, 4)` is exactly `i`. Every other root is computed from an angle of at most an
            /// eighth of a turn and then reflected into place. That makes the roots conjugate-symmetric to the last
            /// bit: the root for `n - k` is exactly the conjugate of the root for `k`.
            ///
            /// A forward transform uses the conjugates of these.
            ///
            /// # Errors
            ///
            /// Returns [`RootOfUnityError::ZeroOrder`] if `n` is zero, since there is no root of order zero.
            pub fn unit_root(k: usize, n: usize) -> Result<Self, $crate::complex::RootOfUnityError> {
                if n == 0 {
                    return Err($crate::complex::RootOfUnityError::ZeroOrder);
                }

                // The wide type means that `4 * r` cannot overflow, whatever `n` is.
                let (n, r) = (n as u128, (k as u128) % (n as u128));
                let quadrant = (4 * r) / n;
                let into_quadrant = 4 * r - quadrant * n;

                // Past the eighth turn of its quadrant, the angle is measured from the far side and the two parts swap.
                let mirrored = 2 * into_quadrant > n;
                let from_near_side = if mirrored { n - into_quadrant } else { into_quadrant };
                let (x, y) = if 2 * into_quadrant == n {
                    // Exactly an eighth of a turn: the two parts are equal, and `sin_cos` would differ between them by
                    // a bit.
                    (std::$float::consts::FRAC_1_SQRT_2, std::$float::consts::FRAC_1_SQRT_2)
                } else {
                    let (sin, cos) =
                        (std::$float::consts::FRAC_PI_2 * from_near_side as $float / n as $float).sin_cos();
                    if mirrored { (sin, cos) } else { (cos, sin) }
                };

                // Each part is negated as `0 - part`, which is `+0` for a zero, where `-part` would be `-0`. A root on an
                // axis then has a zero that is written `0`, and not `-0`.
                Ok(match quadrant {
                    0 => Self::new(x, y),
                    1 => Self::new(0.0 - y, x),
                    2 => Self::new(0.0 - x, 0.0 - y),
                    _ => Self::new(y, 0.0 - x),
                })
            }

            /// The first `count` of the `n` roots of unity, in order, for a transform to read from.
            ///
            /// # Errors
            ///
            /// Returns [`RootOfUnityError::ZeroOrder`] if `n` is zero, even if `count` is too.
            pub fn unit_roots(n: usize, count: usize) -> Result<Vec<Self>, $crate::complex::RootOfUnityError> {
                if n == 0 {
                    return Err($crate::complex::RootOfUnityError::ZeroOrder);
                }
                (0..count).map(|k| Self::unit_root(k, n)).collect()
            }

            /// The conjugate: the imaginary part negated.
            #[inline]
            pub const fn conj(self) -> Self {
                Self::new(self.re, -self.im)
            }

            /// The square of the distance from the origin. Use it in preference to [`norm`](Self::norm) when only
            /// comparing.
            #[inline]
            pub fn norm_sqr(self) -> $float {
                self.re * self.re + self.im * self.im
            }

            /// The distance from the origin, without intermediate overflow or underflow.
            #[inline]
            pub fn norm(self) -> $float {
                self.re.hypot(self.im)
            }

            /// The angle from the positive real axis, in radians, from -π to π.
            #[inline]
            pub fn arg(self) -> $float {
                self.im.atan2(self.re)
            }

            /// The distance and the angle together.
            #[inline]
            pub fn to_polar(self) -> ($float, $float) {
                (self.norm(), self.arg())
            }

            /// Multiplication by a real number: two multiplications, where a full complex product takes four.
            #[inline]
            pub fn scale(self, factor: $float) -> Self {
                Self::new(self.re * factor, self.im * factor)
            }

            /// Multiplication by `i`, a quarter turn anticlockwise. No arithmetic beyond a negation.
            #[inline]
            pub const fn mul_i(self) -> Self {
                Self::new(-self.im, self.re)
            }

            /// Multiplication by `-i`, a quarter turn clockwise. No arithmetic beyond a negation.
            #[inline]
            pub const fn mul_neg_i(self) -> Self {
                Self::new(self.im, -self.re)
            }

            /// Both parts are finite: neither infinite nor NaN.
            #[inline]
            pub fn is_finite(self) -> bool {
                self.re.is_finite() && self.im.is_finite()
            }

            /// The sum and difference of two numbers together, which is the whole of a butterfly's additions.
            #[inline]
            pub fn sum_and_difference(self, other: Self) -> (Self, Self) {
                (self + other, self - other)
            }

            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
            // What division needs. Everything here is exact: it only moves the exponent of a float.
            // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

            /// The bits of a stored mantissa, and the bias of the exponent, of this float type.
            const MANTISSA_BITS: u32 = $float::MANTISSA_DIGITS - 1;
            const BIAS: i32 = $float::MAX_EXP - 1;

            /// How far a number is scaled by one step of [`scaled_by_pow2`](Self::scaled_by_pow2). Small enough that a
            /// number between a third and three, scaled down by it, is still a normal number, so a step never rounds.
            const SCALE_STEP: i32 = Self::BIAS - 3;

            /// The exponent of the finite, non-zero `x`: the `e` for which `x` is between `2^e` and `2^(e + 1)`. It is
            /// found for a subnormal number too, whose exponent field is zero.
            fn ilogb(x: $float) -> i32 {
                let bits = u64::from(x.to_bits());
                let mantissa = bits & ((1_u64 << Self::MANTISSA_BITS) - 1);
                let width = (std::mem::size_of::<$float>() * 8) as u32 - Self::MANTISSA_BITS - 1;
                let field = ((bits >> Self::MANTISSA_BITS) & ((1_u64 << width) - 1)) as i32;
                if field == 0 {
                    // A subnormal number is `mantissa * 2^-(bias + mantissa bits - 1)`.
                    (63 - mantissa.leading_zeros()) as i32 - (Self::BIAS + Self::MANTISSA_BITS as i32 - 1)
                } else {
                    field - Self::BIAS
                }
            }

            /// `2^k`, for any `k` a normal number can have: from `1 - BIAS` to `BIAS`.
            fn pow2(k: i32) -> $float {
                let bits = ((k + Self::BIAS) as u64) << Self::MANTISSA_BITS;
                $float::from_bits(bits as _)
            }

            /// `x * 2^k`, for any `k`, rounded once. It is what `scalbn` is in C. A number is scaled in steps that keep a
            /// number of the size this is used for normal, and the last step is the only one that can round, as it does
            /// when the result is subnormal. A result too large is infinite and one too small is zero, as it should be.
            fn scaled_by_pow2(x: $float, k: i32) -> $float {
                let (mut x, mut k) = (x, k);
                while k > Self::SCALE_STEP && x.is_finite() && x != 0.0 {
                    x *= Self::pow2(Self::SCALE_STEP);
                    k -= Self::SCALE_STEP;
                }
                while k < -Self::SCALE_STEP && x != 0.0 {
                    x *= Self::pow2(-Self::SCALE_STEP);
                    k += Self::SCALE_STEP;
                }
                x * Self::pow2(k.clamp(-Self::SCALE_STEP, Self::SCALE_STEP))
            }

            /// `true` if the larger of `x` and `y` is zero, or is between `2^-(BIAS / 2)` and `2^(BIAS / 2)`: a size at
            /// which the plain formulas cannot overflow or lose their digits to underflow.
            fn is_comfortable(x: $float, y: $float) -> bool {
                let larger = x.abs().max(y.abs());
                let half = Self::BIAS / 2;
                larger == 0.0 || (larger >= Self::pow2(-half) && larger <= Self::pow2(half))
            }

            /// `(a + bi) / (c + di)` by Smith's algorithm: divide through by the larger part of the divisor, so that
            /// nothing is squared. It overflows, or loses its digits, only if the numbers are near the ends of the range.
            fn divide_smith(a: $float, b: $float, c: $float, d: $float) -> Self {
                if c.abs() >= d.abs() {
                    let ratio = d / c;
                    let denominator = c + d * ratio;
                    Self::new((a + b * ratio) / denominator, (b - a * ratio) / denominator)
                } else {
                    let ratio = c / d;
                    let denominator = c * ratio + d;
                    Self::new((a * ratio + b) / denominator, (b * ratio - a) / denominator)
                }
            }

            /// `(a + bi) / (c + di)` for finite numbers of any size, with a divisor that is not zero.
            ///
            /// The numerator and the divisor are each scaled, by a power of two, until the larger of its parts is between
            /// 1 and 2. That is exact, and it puts every number where Smith's algorithm cannot overflow or underflow.
            /// The quotient is then scaled back by the difference of the two powers, in one rounding. So the quotient
            /// of two numbers of the same enormous size is worked out as that of two numbers near 1, and is `1`, and a
            /// quotient that really is too large or too small becomes infinity or zero, never `NaN`.
            fn divide_scaled(a: $float, b: $float, c: $float, d: $float) -> Self {
                let exponent = |x: $float, y: $float| {
                    let (x, y) = (x.abs(), y.abs());
                    // The exponent of the larger. A part that is zero has no exponent.
                    match (x == 0.0, y == 0.0) {
                        (true, true) => None,
                        (false, true) => Some(Self::ilogb(x)),
                        (true, false) => Some(Self::ilogb(y)),
                        (false, false) => Some(Self::ilogb(x).max(Self::ilogb(y))),
                    }
                };
                let Some(divisor) = exponent(c, d) else {
                    return Self::ZERO;
                };
                // Nothing divided by a number is nothing.
                let Some(numerator) = exponent(a, b) else {
                    return Self::ZERO;
                };
                let down = |x: $float, e: i32| Self::scaled_by_pow2(x, -e);
                let quotient =
                    Self::divide_smith(down(a, numerator), down(b, numerator), down(c, divisor), down(d, divisor));
                let back = numerator - divisor;
                Self::new(Self::scaled_by_pow2(quotient.re, back), Self::scaled_by_pow2(quotient.im, back))
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Add implementations
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Add for $name {
            type Output = Self;

            #[inline]
            fn add(self, other: Self) -> Self {
                Self::new(self.re + other.re, self.im + other.im)
            }
        }

        impl AddAssign for $name {
            #[inline]
            fn add_assign(&mut self, other: Self) {
                *self = *self + other;
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Subtract implementations
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Sub for $name {
            type Output = Self;

            #[inline]
            fn sub(self, other: Self) -> Self {
                Self::new(self.re - other.re, self.im - other.im)
            }
        }

        impl SubAssign for $name {
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                *self = *self - other;
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Multiplication implementations
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Mul for $name {
            type Output = Self;

            #[inline]
            fn mul(self, other: Self) -> Self {
                Self::new(self.re * other.re - self.im * other.im, self.re * other.im + self.im * other.re)
            }
        }

        impl Mul<$float> for $name {
            type Output = Self;

            #[inline]
            fn mul(self, factor: $float) -> Self {
                self.scale(factor)
            }
        }

        impl Mul<$name> for $float {
            type Output = $name;

            #[inline]
            fn mul(self, number: $name) -> $name {
                number.scale(self)
            }
        }

        impl MulAssign for $name {
            #[inline]
            fn mul_assign(&mut self, other: Self) {
                *self = *self * other;
            }
        }

        impl MulAssign<$float> for $name {
            #[inline]
            fn mul_assign(&mut self, factor: $float) {
                *self = self.scale(factor);
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Division implementations
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Div for $name {
            type Output = Self;

            /// Divides by `other`, and gives a number that is right at any size.
            ///
            /// * Numbers of ordinary size are divided by Smith's algorithm, which is the quick way.
            /// * Numbers near either end of the range, which that would overflow on or lose their digits to, are scaled
            ///   by powers of two first, and the quotient scaled back, so `MAX` over `MAX` is `1` and not `NaN`. See
            ///   [`divide_scaled`](Self::divide_scaled).
            /// * Dividing by zero is dividing each part by the real part of the divisor: an infinity, or `NaN` for a
            ///   numerator of zero.
            /// * A number that is not finite is divided as Smith's algorithm divides it.
            #[inline]
            fn div(self, other: Self) -> Self {
                let (a, b, c, d) = (self.re, self.im, other.re, other.im);
                if c == 0.0 && d == 0.0 {
                    return Self::new(a / c, b / c);
                }
                if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
                    return Self::divide_smith(a, b, c, d);
                }
                if Self::is_comfortable(a, b) && Self::is_comfortable(c, d) {
                    Self::divide_smith(a, b, c, d)
                } else {
                    Self::divide_scaled(a, b, c, d)
                }
            }
        }

        impl Div<$float> for $name {
            type Output = Self;

            /// Divides each part by `divisor`. It is a true division, so it is right at any size: a reciprocal
            /// multiplied through would overflow for a divisor near the smallest, and divide `1e-320` by `1e-320` to
            /// infinity. To scale many numbers by one divisor, take the reciprocal once and use
            /// [`scale`](Self::scale), as a transform does by `1/n`.
            #[inline]
            fn div(self, divisor: $float) -> Self {
                Self::new(self.re / divisor, self.im / divisor)
            }
        }

        impl DivAssign for $name {
            #[inline]
            fn div_assign(&mut self, other: Self) {
                *self = *self / other;
            }
        }

        impl DivAssign<$float> for $name {
            #[inline]
            fn div_assign(&mut self, divisor: $float) {
                *self = *self / divisor;
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Neg for $name {
            type Output = Self;

            #[inline]
            fn neg(self) -> Self {
                Self::new(-self.re, -self.im)
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl Sum for $name {
            #[inline]
            fn sum<I: Iterator<Item = Self>>(numbers: I) -> Self {
                numbers.fold(Self::ZERO, Add::add)
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        impl From<$float> for $name {
            #[inline]
            fn from(re: $float) -> Self {
                Self::from_real(re)
            }
        }

        impl From<($float, $float)> for $name {
            #[inline]
            fn from((re, im): ($float, $float)) -> Self {
                Self::new(re, im)
            }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        /// Written as `re+imi` or `re-imi`, with the same precision, if any, for both parts.
        /// A negative zero is written as zero, so that a rounded result never reads `-0.000`.
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // Adding positive zero turns a negative zero into a positive one and changes nothing else.
                let (re, im) = (self.re + 0.0, self.im + 0.0);
                let sign = if im.is_sign_negative() { '-' } else { '+' };

                match f.precision() {
                    Some(digits) => write!(f, "{re:.digits$}{sign}{:.digits$}i", im.abs()),
                    None => write!(f, "{re}{sign}{}i", im.abs()),
                }
            }
        }
    };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) use define_complex;
