//! A bit string that is written most significant bit first, with a limit it cannot pass.
//!
//! The bits are kept one to a `u8`, each `0` or `1`. That is the form a one-bit [`BitValues`] takes, so a stage of the
//! encoder can be drawn as it is.
//!
//! [`BitValues`]: svg_dom_graph::scene::BitValues

use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a [`BitWriter`] refused to write. The writer is unchanged whenever one of these is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BitError {
    /// A group of bits is between 1 and 32 wide.
    BadWidth { width: usize },
    /// `value` does not fit in `width` bits.
    ValueTooWide { value: u32, width: usize },
    /// Writing `width` more bits would pass the limit of `limit` bits, from `len`.
    Overflow { len: usize, width: usize, limit: usize },
    /// The bits are not a whole number of bytes.
    NotWholeBytes { len: usize },
}

impl fmt::Display for BitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::BadWidth { width } => write!(f, "{width} is not a width of between 1 and 32 bits"),
            Self::ValueTooWide { value, width } => write!(f, "{value} does not fit in {width} bits"),
            Self::Overflow { len, width, limit } => {
                write!(f, "writing {width} bits after {len} would pass the limit of {limit}")
            },
            Self::NotWholeBytes { len } => write!(f, "{len} bits are not a whole number of bytes"),
        }
    }
}

impl std::error::Error for BitError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A bit string with a limit on its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BitWriter {
    bits: Vec<u8>,
    limit: usize,
}

impl BitWriter {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// An empty bit string that can hold `limit` bits.
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            bits: Vec::with_capacity(limit),
            limit,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many bits have been written. It is also the offset the next bit goes to.
    pub(crate) fn len(&self) -> usize {
        self.bits.len()
    }

    /// The bits written so far, each `0` or `1`.
    pub(crate) fn bits(&self) -> &[u8] {
        &self.bits
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the low `width` bits of `value`, the highest of them first.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `width` is not between 1 and 32, if `value` does not fit in it, or if the bits would pass the
    /// limit.
    pub(crate) fn push(&mut self, value: u32, width: usize) -> Result<(), BitError> {
        if !(1..=32).contains(&width) {
            return Err(BitError::BadWidth { width });
        }
        if width < 32 && value >> width != 0 {
            return Err(BitError::ValueTooWide { value, width });
        }
        self.check_room(width)?;
        self.bits.extend((0..width).rev().map(|bit| u8::from(value >> bit & 1 == 1)));
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes `count` zero bits.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the bits would pass the limit.
    pub(crate) fn push_zeros(&mut self, count: usize) -> Result<(), BitError> {
        self.check_room(count)?;
        self.bits.resize(self.bits.len() + count, 0);
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the zero bits that bring the length to a multiple of 8, and says how many there were.
    ///
    /// # Errors
    ///
    /// Returns `Err` if those bits would pass the limit.
    pub(crate) fn pad_to_byte(&mut self) -> Result<usize, BitError> {
        let count = (8 - self.bits.len() % 8) % 8;
        self.push_zeros(count)?;
        Ok(count)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The bits as bytes, the first bit being the highest bit of the first byte.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the length is not a multiple of 8.
    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, BitError> {
        if self.bits.len() % 8 != 0 {
            return Err(BitError::NotWholeBytes { len: self.bits.len() });
        }
        Ok(self
            .bits
            .chunks(8)
            .map(|byte| byte.iter().fold(0, |value, &bit| value << 1 | bit))
            .collect())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Checks that `width` more bits stay within the limit.
    fn check_room(&self, width: usize) -> Result<(), BitError> {
        if self.bits.len() + width > self.limit {
            return Err(BitError::Overflow {
                len: self.bits.len(),
                width,
                limit: self.limit,
            });
        }
        Ok(())
    }
}
