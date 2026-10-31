//! What the encoder accepts: up to 17 bytes, and for text only ASCII.
//!
//! Text is taken one byte to one character, so that what is shown is what is encoded. A character above U+007F is
//! refused, never turned into bytes by some encoding the reader cannot see. No ECI is written.

use super::EncodeError;

/// The most bytes a Version 1 symbol holds at level L in byte mode.
pub(crate) const MAX_BYTES: usize = 17;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bytes of `text`, one for each character.
///
/// # Errors
///
/// Returns `Err` for the first character above U+007F, or if there are more than [`MAX_BYTES`] characters. The first is
/// reported before the second, so that a long text with a bad character in it is told about the character.
pub(crate) fn text_to_bytes(text: &str) -> Result<Vec<u8>, EncodeError> {
    if let Some((index, character)) = text.chars().enumerate().find(|(_, c)| !c.is_ascii()) {
        return Err(EncodeError::NotAscii { index, character });
    }
    // Every character is ASCII, so it is one byte, and the length in bytes is the length in characters.
    check_length(text.len())?;
    Ok(text.as_bytes().to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks that `len` bytes fit.
///
/// # Errors
///
/// Returns `Err` if `len` is more than [`MAX_BYTES`].
pub(crate) fn check_length(len: usize) -> Result<(), EncodeError> {
    if len > MAX_BYTES {
        return Err(EncodeError::TooLong { bytes: len, limit: MAX_BYTES });
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bytes` as text that can be read: a printable ASCII character as itself, a backslash doubled, tab, line feed and
/// carriage return by name, and any other byte as `\xNN`.
pub(crate) fn escaped(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            b'\\' => "\\\\".to_owned(),
            b'\t' => "\\t".to_owned(),
            b'\n' => "\\n".to_owned(),
            b'\r' => "\\r".to_owned(),
            0x20..=0x7E => char::from(byte).to_string(),
            _ => format!("\\x{byte:02X}"),
        })
        .collect()
}
