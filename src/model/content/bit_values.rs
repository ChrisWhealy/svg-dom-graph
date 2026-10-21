//! `NodeValues::Bits`: an array of values that are each fewer than eight bits wide, such as the six-bit values that
//! Base64 makes.

use crate::error::Error;

/// The widest a value of [`BitValues`] can be. A value of eight bits is a `u8`, which [`NodeValues::U8`] holds.
///
/// [`NodeValues::U8`]: super::NodeValues::U8
pub const MAX_BIT_WIDTH: u8 = 7;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A list of values that are each exactly `width` bits wide, from 1 to [`MAX_BIT_WIDTH`], for
/// [`NodeValues::Bits`](super::NodeValues::Bits).
///
/// Each value is held in a `u8`, and must fit in `width` bits: `0..(1 << width)`. A value that does not is rejected
/// by [`new`](Self::new), so a `BitValues` can never hold one.
///
/// # Formats
///
/// - [`DataFormat::Binary`](super::DataFormat::Binary) writes exactly `width` binary digits with no grouping. So a
///   six-bit value is `010011`, not `0001 0011`. The grouping into nybbles that a whole byte has is meaningless for a
///   width that is not a multiple of four.
/// - [`DataFormat::Hexadecimal`](super::DataFormat::Hexadecimal) writes `width.div_ceil(4)` uppercase digits, so a
///   six-bit value is two digits.
/// - [`DataFormat::Decimal`](super::DataFormat::Decimal) writes the value as a plain decimal number.
/// - [`DataFormat::Ascii`](super::DataFormat::Ascii) writes the value as the byte it is held in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitValues {
    width: u8,
    values: Vec<u8>,
}

impl BitValues {
    /// Holds `values`, each of which is `width` bits wide.
    ///
    /// ```
    /// use svg_dom_graph::scene::BitValues;
    /// let sextets = BitValues::new(6, vec![19, 22, 5, 46]).unwrap();
    /// assert_eq!(sextets.width(), 6);
    /// // 64 does not fit in six bits.
    /// assert!(BitValues::new(6, vec![64]).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBitValues`] with no index if `width` is `0` or more than [`MAX_BIT_WIDTH`]. It returns
    /// it with the index of the first value that does not fit in `width` bits otherwise.
    pub fn new(width: u8, values: Vec<u8>) -> Result<Self, Error> {
        if width == 0 || width > MAX_BIT_WIDTH {
            return Err(Error::InvalidBitValues { width, index: None });
        }
        match values.iter().position(|&v| v >> width != 0) {
            Some(index) => Err(Error::InvalidBitValues { width, index: Some(index) }),
            None => Ok(Self { width, values }),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many bits wide each value is.
    #[must_use]
    pub fn width(&self) -> u8 {
        self.width
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The values, each in `0..(1 << width)`.
    #[must_use]
    pub fn values(&self) -> &[u8] {
        &self.values
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `value` as exactly `width` binary digits into `out`, most significant first, with no grouping.
pub(super) fn format_bits_binary_into(value: u8, width: u8, out: &mut String) {
    out.clear();
    for bit in (0..width).rev() {
        out.push(if value >> bit & 1 == 1 { '1' } else { '0' });
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `value` as `width.div_ceil(4)` uppercase hexadecimal digits into `out`.
pub(super) fn format_bits_hex_into(value: u8, width: u8, out: &mut String) {
    use std::fmt::Write as _;
    out.clear();
    let _ = write!(out, "{value:0digits$X}", digits = usize::from(width.div_ceil(4)));
}
