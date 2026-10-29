//! The data codewords: the mode, the length and the bytes, then the terminator and the padding that fill 19 codewords.

use super::{
    EncodeError,
    bits::BitWriter,
    input::{MAX_BYTES, check_length},
};

/// How many data codewords a Version 1-L symbol has, and so how many bits.
pub(crate) const DATA_CODEWORDS: usize = 19;
const DATA_BITS: usize = DATA_CODEWORDS * 8;

/// The byte mode indicator, which is 4 bits, and the width of the count that follows it for Version 1.
const MODE_INDICATOR: u32 = 0b0100;
const MODE_BITS: usize = 4;
const COUNT_BITS: usize = 8;

/// The most zero bits that end the data.
const TERMINATOR_BITS: usize = 4;

/// The two codewords that pad what is left, one after the other.
pub(crate) const PAD_CODEWORDS: [u8; 2] = [0xEC, 0x11];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every stage of the data, so that each can be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataCodewords {
    /// The mode indicator, the count and the bytes, one bit to a `u8`.
    pub(crate) bitstream: Vec<u8>,
    /// How many zero bits ended the data, which is at most 4, and fewer if the symbol is nearly full.
    pub(crate) terminator_bits: usize,
    /// How many zero bits then filled the last byte.
    pub(crate) boundary_bits: usize,
    /// How many whole pad codewords then filled the rest, which alternate between 0xEC and 0x11, starting with 0xEC.
    pub(crate) pad_codewords: usize,
    /// The 19 data codewords.
    pub(crate) codewords: [u8; DATA_CODEWORDS],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The data codewords of `input`.
///
/// # Errors
///
/// Returns `Err` if there are more than 17 bytes. Anything else that goes wrong means the arithmetic here is wrong, and
/// is reported as an internal error that says where.
pub(crate) fn encode_data(input: &[u8]) -> Result<DataCodewords, EncodeError> {
    check_length(input.len())?;
    let internal =
        |stage: &'static str| move |e: super::bits::BitError| EncodeError::Internal { stage, detail: e.to_string() };

    let mut writer = BitWriter::new(DATA_BITS);
    writer.push(MODE_INDICATOR, MODE_BITS).map_err(internal("mode indicator"))?;
    // The count fits in 8 bits, since the length was checked against 17 above.
    let count = u32::try_from(input.len()).map_err(|_| EncodeError::TooLong {
        bytes: input.len(),
        limit: MAX_BYTES,
    })?;
    writer.push(count, COUNT_BITS).map_err(internal("character count"))?;
    for &byte in input {
        writer.push(u32::from(byte), 8).map_err(internal("payload"))?;
    }
    let bitstream = writer.bits().to_vec();

    let terminator_bits = TERMINATOR_BITS.min(DATA_BITS - writer.len());
    writer.push_zeros(terminator_bits).map_err(internal("terminator"))?;
    let boundary_bits = writer.pad_to_byte().map_err(internal("byte boundary"))?;

    let mut codewords = writer.to_bytes().map_err(internal("codewords"))?;
    let pad_codewords = DATA_CODEWORDS - codewords.len();
    codewords.extend(PAD_CODEWORDS.iter().cycle().take(pad_codewords));
    let codewords: [u8; DATA_CODEWORDS] = codewords.try_into().map_err(|rest: Vec<u8>| EncodeError::Internal {
        stage: "padding",
        detail: format!("{} codewords, not {DATA_CODEWORDS}", rest.len()),
    })?;

    Ok(DataCodewords {
        bitstream,
        terminator_bits,
        boundary_bits,
        pad_codewords,
        codewords,
    })
}
