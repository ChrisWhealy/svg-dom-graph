//! The byte writer of the canonical encodings, and the hashes. The encodings are fixed in `spmv/docs/contract.md` section 6.

use sha2::{Digest, Sha256};

/// A byte writer. Integers are little-endian.
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

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// The SHA-256 of what has been written, as lower-case hex.
    #[must_use]
    pub fn digest(&self) -> String {
        sha256_hex(&self.bytes)
    }
}

/// The SHA-256 of `data`, as lower-case hex.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let hash = Sha256::digest(data);
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// The hash of a vector: `u8 1`, `u64` length, and the entries.
#[must_use]
pub fn vector_hash(v: &[i64]) -> String {
    let mut w = Writer::new();
    w.u8(1).u64(v.len() as u64);
    for e in v {
        w.i64(*e);
    }
    w.digest()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_nothing_and_of_abc() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_vector_hash_depends_on_length_and_order() {
        assert_ne!(vector_hash(&[1, 2]), vector_hash(&[2, 1]));
        assert_ne!(vector_hash(&[0]), vector_hash(&[0, 0]));
    }
}
