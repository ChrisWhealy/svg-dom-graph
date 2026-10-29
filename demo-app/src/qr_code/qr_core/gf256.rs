//! The finite field GF(2^8) that Reed-Solomon coding in a QR code works in.
//!
//! An element is a byte, read as a polynomial over GF(2) of degree below 8. Adding is exclusive or. Multiplying is
//! polynomial multiplication, reduced by x^8 + x^4 + x^3 + x^2 + 1, which is 0x11D. The element 2, which is `x`, is a
//! primitive element, called alpha: its powers 0 to 254 are every non-zero element once.

use std::ops::{Add, Mul};

/// The reducing polynomial, with its x^8 term.
const REDUCING_POLYNOMIAL: u16 = 0x11D;

/// How many non-zero elements the field has, and so the period of the powers of alpha.
const ORDER: usize = 255;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The powers of alpha, then the same again so that a sum of two logarithms needs no reduction, and the logarithms.
const TABLES: ([u8; 2 * ORDER], [u8; 256]) = {
    let mut powers = [0_u8; 2 * ORDER];
    let mut logs = [0_u8; 256];
    let mut x: u16 = 1;
    let mut i = 0;
    while i < ORDER {
        powers[i] = x as u8;
        logs[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= REDUCING_POLYNOMIAL;
        }
        i += 1;
    }
    while i < 2 * ORDER {
        powers[i] = powers[i - ORDER];
        i += 1;
    }
    (powers, logs)
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One element of GF(2^8). It is private to the encoder, and is not a node value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gf256(u8);

impl Gf256 {
    pub(crate) const ZERO: Self = Self(0);
    pub(crate) const ONE: Self = Self(1);

    pub(crate) const fn new(value: u8) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u8 {
        self.0
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Alpha to the power `exponent`. The powers repeat every 255.
    pub(crate) const fn alpha_pow(exponent: usize) -> Self {
        Self(TABLES.0[exponent % ORDER])
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Add for Gf256 {
    type Output = Self;

    /// Adding and subtracting are the same, and are exclusive or.
    #[allow(
        clippy::suspicious_arithmetic_impl,
        reason = "addition in GF(2^8) is exclusive or"
    )]
    fn add(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Mul for Gf256 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        if self.0 == 0 || other.0 == 0 {
            return Self::ZERO;
        }
        let (powers, logs) = &TABLES;
        Self(powers[usize::from(logs[usize::from(self.0)]) + usize::from(logs[usize::from(other.0)])])
    }
}
