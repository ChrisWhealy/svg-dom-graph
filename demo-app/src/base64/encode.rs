//! Base64 encoding as RFC 4648 defines it, worked out one group of three bytes at a time so that the demo can show every
//! step. Nothing here touches the DOM or the library.

/// The 64 characters a six-bit value stands for in standard Base64, in order (RFC 4648 §4).
const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The alphabet of Base64URL (RFC 4648 §5): the same, but with `-` for `+` and `_` for `/`, so that the text can be used
/// in a URL or a file name without escaping.
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Which 64 characters the six-bit values stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Alphabet {
    Standard,
    UrlSafe,
}

impl Alphabet {
    /// The characters of this alphabet, in the order of the values they stand for.
    pub(super) fn characters(self) -> &'static [u8; 64] {
        match self {
            Self::Standard => STANDARD,
            Self::UrlSafe => URL_SAFE,
        }
    }
}

/// The character that fills the last group when the input does not divide into whole groups of three bytes.
pub(super) const PADDING: u8 = b'=';

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four six-bit values that three bytes make when their 24 bits are cut into four pieces. A group that is short of
/// bytes is padded with zero bits first.
pub(super) fn sextets(group: [u8; 3]) -> [u8; 4] {
    [
        group[0] >> 2,
        (group[0] & 0b11) << 4 | group[1] >> 4,
        (group[1] & 0b1111) << 2 | group[2] >> 6,
        group[2] & 0b11_1111,
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The UTF-8 bytes of `text`, cut off at the last whole character that fits in `max` bytes. Cutting at exactly `max`
/// could leave the first byte or two of a multi-byte character, which no one could decode back into text.
pub(crate) fn utf8_prefix(text: &str, max: usize) -> &[u8] {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text.as_bytes()[..end]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the page says about the length of the text: how many bytes it has out of `max`, and when it has more than are
/// encoded, how many were left out. `typed` is the length of the whole text in bytes and `encoded` the length of
/// [`utf8_prefix`] of it.
pub(crate) fn length_note(typed: usize, encoded: usize, max: usize) -> String {
    if typed <= max {
        return format!("{typed} of {max} bytes");
    }
    format!(
        "{typed} bytes typed. Only the first {encoded} are encoded, since the limit is {max} and the cut falls between \
         characters. The last {} are left out.",
        typed - encoded
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many characters `len` input bytes encode to, counting the padding: four for each group of three bytes, or part
/// of one.
pub(super) fn encoded_len(len: usize) -> usize {
    len.div_ceil(3) * 4
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything the demo shows about encoding `input`: the six-bit values of every group, and the characters they stand
/// for, with the padding.
pub(super) struct Encoded {
    /// Four values for each group of three bytes. The last group is padded with zero bits if it is short.
    pub(super) sextets: Vec<u8>,
    /// Four characters for each group. A group of one byte ends in two padding characters, and a group of two bytes in
    /// one.
    pub(super) text: Vec<u8>,
}

/// Encodes `input` with `alphabet`. Both alphabets pad the last group with `=`.
pub(super) fn encode(input: &[u8], alphabet: Alphabet) -> Encoded {
    let mut sextets_out = Vec::with_capacity(encoded_len(input.len()));
    let mut text = Vec::with_capacity(encoded_len(input.len()));
    for chunk in input.chunks(3) {
        let mut group = [0_u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let values = sextets(group);
        // A chunk of n bytes needs n + 1 characters, and the rest of the four are padding.
        for (i, value) in values.iter().enumerate() {
            sextets_out.push(*value);
            text.push(if i <= chunk.len() {
                alphabet.characters()[usize::from(*value)]
            } else {
                PADDING
            });
        }
    }
    Encoded { sextets: sextets_out, text }
}
