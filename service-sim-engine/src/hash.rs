//! The byte writer of the canonical state, and the hashes. The encoding is fixed in `dess/s1/docs/contract.md`.

use sha2::{Digest, Sha256};

/// The SHA-256 of `data`, as lower-case hex.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// A byte writer. Integers are little-endian and unsigned, a string is a `u32` length and its UTF-8 bytes, and a list
/// is a `u32` count and its items.
#[derive(Debug, Default)]
pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.bytes.push(v);
        self
    }

    /// A `u32`. A count or a length that does not fit is a programming error, since limits keep every one far below it.
    pub fn u32(&mut self, v: usize) -> &mut Self {
        let v = u32::try_from(v).expect("a count or a length fits in 32 bits");
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn string(&mut self, s: &str) -> &mut Self {
        self.u32(s.len());
        self.bytes.extend_from_slice(s.as_bytes());
        self
    }

    pub fn u64_list(&mut self, list: &[u64]) -> &mut Self {
        self.u32(list.len());
        for v in list {
            self.u64(*v);
        }
        self
    }

    #[must_use]
    pub fn digest(&self) -> String {
        sha256_hex(&self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_abc_is_the_published_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_writer_is_little_endian_and_length_prefixed() {
        let mut w = Writer::new();
        w.u8(1).u32(2).u64(3).string("hi").u64_list(&[4, 5]);
        assert_eq!(
            w.bytes,
            [
                1, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, b'h', b'i', 2, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 5,
                0, 0, 0, 0, 0, 0, 0
            ]
        );
    }
}
