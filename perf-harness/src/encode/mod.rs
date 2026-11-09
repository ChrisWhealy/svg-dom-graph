//! The canonical byte encoding that an operation stream is hashed in.
//!
//! Every value starts with a one-byte type tag. Integers are little-endian at a fixed width. A string is a
//! little-endian `u32` byte length and then its UTF-8. A float is its IEEE-754 bit pattern, with `-0.0` written as
//! `+0.0`, and NaN and infinity are refused. The whole stream begins with [`ENCODING_VERSION`], so a later change of
//! this layout cannot be mistaken for the same stream.

/// The version byte that opens every encoded stream.
pub const ENCODING_VERSION: u8 = 1;

const TAG_U64: u8 = 0x01;
const TAG_I64: u8 = 0x02;
const TAG_STR: u8 = 0x03;
const TAG_F64: u8 = 0x04;
const TAG_BOOL: u8 = 0x05;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// NaN and infinity have no canonical encoding, so no manifest or operation may carry one.
    NotFinite,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFinite => write!(f, "a float that is not finite cannot be encoded"),
        }
    }
}

impl std::error::Error for EncodeError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Collects the encoding of one operation at a time.
#[derive(Debug, Default)]
pub struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self::default()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many bytes the buffer can hold without growing. It only ever grows, to the largest operation encoded.
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The one-byte code of an operation, which is not tagged because it always comes first.
    pub fn code(&mut self, code: u8) {
        self.bytes.push(code);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn put_u64(&mut self, value: u64) {
        self.bytes.push(TAG_U64);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn put_i64(&mut self, value: i64) {
        self.bytes.push(TAG_I64);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn put_str(&mut self, value: &str) {
        self.bytes.push(TAG_STR);
        let length = u32::try_from(value.len()).expect("a string longer than u32::MAX bytes is not a workload value");
        self.bytes.extend_from_slice(&length.to_le_bytes());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn put_bool(&mut self, value: bool) {
        self.bytes.push(TAG_BOOL);
        self.bytes.push(u8::from(value));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// # Errors
    ///
    /// Returns [`EncodeError::NotFinite`] for NaN and infinity.
    pub fn put_f64(&mut self, value: f64) -> Result<(), EncodeError> {
        if !value.is_finite() {
            return Err(EncodeError::NotFinite);
        }
        // `-0.0` and `+0.0` compare equal, so they must encode alike.
        let value = if value == 0.0 { 0.0 } else { value };
        self.bytes.push(TAG_F64);
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
