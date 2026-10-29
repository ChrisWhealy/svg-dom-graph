//! The encoder of a Version 1-L symbol, in byte mode: from up to 17 bytes to the finished 21 by 21 symbol.
//!
//! It is plain Rust. It has no DOM, no scene and no timing, so each stage can be tested, and each is kept so that it
//! can be shown. [`encode`] runs the first three stages, and [`symbol::build_symbol`] the rest:
//!
//! 1. [`input`] checks the bytes, or turns ASCII text into them.
//! 2. [`data`] writes the mode, the count and the bytes with a [`bits::BitWriter`], ends them, and pads them to 19
//!    codewords.
//! 3. [`reed_solomon`] divides those by a generator, in [`gf256`], one byte at a time. The remainder is the 7 error
//!    correction codewords.
//! 4. [`symbol`] reserves the function modules, places the 26 codewords, and makes eight candidates. Each candidate has
//!    one of the [`mod@mask`] patterns applied and its own [`mod@format`] information written. [`score`] scores each,
//!    and the least total wins.
//!
//! The quiet zone is not part of the symbol. An export adds it.
//!
//! The golden vectors come from python-qrcode, and segno is only a cross-check. `golden.rs` has the codewords, and
//! `symbol_golden.rs` has the eight candidate matrices, format bits and scores of each input. segno adds a whole extra
//! zero codeword where the standard adds none, so its symbols hold other codewords than this encoder's, and a matrix or
//! a mask candidate from it cannot be compared. Its scores of python-qrcode's matrices can, and are. python-qrcode
//! counts N3 without a border and scores before the format information is written, so it too can choose another mask
//! than the design's. `generate_golden.py` and `generate_symbol_golden.py` give the details.

pub(crate) mod bits;
pub(crate) mod data;
pub(crate) mod format;
pub(crate) mod gf256;
pub(crate) mod input;
pub(crate) mod mask;
pub(crate) mod reed_solomon;
pub(crate) mod score;
pub(crate) mod symbol;

#[cfg(test)]
mod golden;
#[cfg(test)]
mod symbol_golden;

use data::{DATA_CODEWORDS, DataCodewords, encode_data};
use input::text_to_bytes;
use reed_solomon::{DivisionStep, divide, remainder};
use std::fmt;

/// How many error correction codewords a Version 1-L symbol has.
pub(crate) const ERROR_CORRECTION_CODEWORDS: usize = 7;

/// The codewords of the whole symbol: the data, then the error correction.
pub(crate) const TOTAL_CODEWORDS: usize = DATA_CODEWORDS + ERROR_CORRECTION_CODEWORDS;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why an input could not be encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EncodeError {
    /// There are more bytes than a Version 1 symbol holds at level L in byte mode.
    TooLong { bytes: usize, limit: usize },
    /// The text has a character above U+007F, which would need an encoding to become bytes.
    NotAscii { index: usize, character: char },
    /// A stage did something its own arithmetic rules out. No symbol is made.
    Internal { stage: &'static str, detail: String },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { bytes, limit } => write!(
                f,
                "{bytes} bytes is too many: a Version 1 symbol holds at most {limit} in byte mode at level L"
            ),
            Self::NotAscii { index, character } => write!(
                f,
                "character {} is {character:?}, which is not ASCII. Only U+0000 to U+007F can be encoded",
                index + 1
            ),
            Self::Internal { stage, detail } => write!(f, "internal error in the {stage} stage: {detail}"),
        }
    }
}

impl std::error::Error for EncodeError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every stage of the encoding of one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Encoded {
    pub(crate) input: Vec<u8>,
    pub(crate) data: DataCodewords,
    /// One for each data codeword.
    pub(crate) division: Vec<DivisionStep>,
    pub(crate) error_correction: [u8; ERROR_CORRECTION_CODEWORDS],
}

impl Encoded {
    /// The 26 codewords in the order they are placed: the 19 of data, then the 7 of error correction.
    pub(crate) fn codewords(&self) -> [u8; TOTAL_CODEWORDS] {
        let mut all = [0; TOTAL_CODEWORDS];
        all[..DATA_CODEWORDS].copy_from_slice(&self.data.codewords);
        all[DATA_CODEWORDS..].copy_from_slice(&self.error_correction);
        all
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes `input`, any bytes, as the codewords of a Version 1-L symbol.
///
/// # Errors
///
/// Returns `Err` if there are more than 17 bytes. A stage whose own arithmetic fails returns an internal error that
/// says which, and no symbol.
pub(crate) fn encode(input: &[u8]) -> Result<Encoded, EncodeError> {
    let data = encode_data(input)?;
    let division = divide(&data.codewords, ERROR_CORRECTION_CODEWORDS);
    let error_correction: [u8; ERROR_CORRECTION_CODEWORDS] = remainder(&division, ERROR_CORRECTION_CODEWORDS)
        .try_into()
        .map_err(|rest: Vec<u8>| EncodeError::Internal {
            stage: "Reed-Solomon",
            detail: format!("a remainder of {} bytes, not {ERROR_CORRECTION_CODEWORDS}", rest.len()),
        })?;
    Ok(Encoded {
        input: input.to_vec(),
        data,
        division,
        error_correction,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes ASCII `text`, one byte to a character.
///
/// # Errors
///
/// Returns `Err` for a character above U+007F, or for more than 17 characters.
pub(crate) fn encode_text(text: &str) -> Result<Encoded, EncodeError> {
    encode(&text_to_bytes(text)?)
}

#[cfg(test)]
mod symbol_tests;
#[cfg(test)]
mod unit_tests;
